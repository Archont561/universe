//! The branch store's transport: one orphan snapshot, force-pushed.
//!
//! A channel on a Git branch is a poor CDN and a very good transport for a disconnected host:
//! it needs no endpoint, no credentials beyond the ones that already push the repository, and
//! a consumer restores it by cloning. The costs are real and are stated where they bite —
//! a blob above the ceiling, a repository that grows with every build, a force-push that
//! bounds what is *served* without shrinking what is *stored*.
//!
//! Every `git` invocation goes through [`GitTransport::run`], and every argv it issues is
//! visible to a test through that one seam.
//!
//! # Where the push credential comes from
//!
//! A developer's `git` finds a credential because a helper, an SSH key or a cached token is
//! already installed for them. A CI job has none of that: `actions/checkout` writes its token
//! into the *outer* repository's config, and this transport pushes from a scratch repository
//! it creates itself, so the checkout credential is invisible to it. That is the whole bug
//! `UNI-11` records — a tag run that renders, verifies and then fails at the last step.
//!
//! The fix is [`PushCredential`]: the token the store's manifest names reaches git as
//! `http.<url>.extraheader` through `GIT_CONFIG_COUNT`/`GIT_CONFIG_KEY_n`/`GIT_CONFIG_VALUE_n`
//! environment variables. Never argv — argv is readable by every process on the host and is
//! what an error message quotes back — and never a file under the repository, where it would
//! be one `git add -A` away from being committed.

use std::path::{Path, PathBuf};
use std::process::Command;

use universe_channel::transport::{
    ChannelTree, PushOutcome, Transport, TransportError, TreeIdentity,
};

/// A channel held on a Git branch, reached through an existing remote.
#[derive(Debug)]
pub(crate) struct GitTransport {
    remote: String,
    branch: String,
    scratch: PathBuf,
    /// Fixed identity and date, so the commit id is a function of the content.
    identity: [(&'static str, &'static str); 4],
    /// A token to present, when the caller supplied one rather than leaving git to find it.
    credential: Option<PushCredential>,
}

impl GitTransport {
    /// A transport for `remote` at `branch`, staging work in `scratch`.
    pub(crate) fn new(remote: &str, branch: &str, scratch: &Path) -> Self {
        Self {
            remote: remote.to_owned(),
            branch: branch.to_owned(),
            scratch: scratch.to_path_buf(),
            identity: [
                ("user.name", "Universe"),
                ("user.email", "universe@invalid"),
                ("GIT_AUTHOR_DATE", "2000-01-01T00:00:00 +0000"),
                ("GIT_COMMITTER_DATE", "2000-01-01T00:00:00 +0000"),
            ],
            credential: None,
        }
    }

    /// The same transport, presenting `token` to the remote when it speaks HTTPS.
    ///
    /// A remote that does not — an SSH one, or a local path — keeps whatever authority it
    /// already had: an SSH key is not something a token can improve on.
    pub(crate) fn authenticated(mut self, token: Option<&str>) -> Self {
        self.credential = token.and_then(|token| PushCredential::new(&self.remote, token));
        self
    }

    /// Run one git command in the scratch repository.
    fn run(&self, arguments: &[&str]) -> Result<String, TransportError> {
        let mut command = Command::new("git");
        // Global options come before the subcommand: `git -c a=b init` is config, `git init
        // -c a=b` is an unknown switch.
        command.arg("-C").arg(&self.scratch);
        for (name, value) in self.identity {
            // The dates are environment and the identity is config; each mechanism ignores
            // the other's variables.
            if name.starts_with("GIT_") {
                command.env(name, value);
            } else {
                command.arg("-c").arg(format!("{name}={value}"));
            }
        }
        if let Some(credential) = &self.credential {
            credential.apply(&mut command);
        }
        command.args(arguments);
        let output = command.output().map_err(|error| TransportError::Failed {
            operation: "git".to_owned(),
            message: format!("cannot run git: {error}"),
        })?;
        if !output.status.success() {
            return Err(TransportError::Failed {
                operation: format!("git {}", arguments.join(" ")),
                message: String::from_utf8_lossy(&output.stderr).trim().to_owned(),
            });
        }
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_owned())
    }

    fn prepare(&self) -> Result<(), TransportError> {
        std::fs::create_dir_all(&self.scratch).map_err(|error| TransportError::Failed {
            operation: "create scratch".to_owned(),
            message: error.to_string(),
        })?;
        if self.scratch.join(".git").exists() {
            return Ok(());
        }
        self.run(&["init", "--quiet"]).map(|_| ())
    }

    /// Stage `tree` and return the git tree hash it produces.
    fn write_tree(&self, tree: &ChannelTree) -> Result<String, TransportError> {
        self.prepare()?;
        // A stale index would carry files from a previous snapshot into this one.
        self.run(&["rm", "-r", "--cached", "--quiet", "."]).ok();
        for (path, bytes) in tree.files() {
            let target = self.scratch.join(path);
            if let Some(parent) = target.parent() {
                std::fs::create_dir_all(parent).map_err(|error| TransportError::Failed {
                    operation: format!("create {}", parent.display()),
                    message: error.to_string(),
                })?;
            }
            std::fs::write(&target, bytes).map_err(|error| TransportError::Failed {
                operation: format!("write {path}"),
                message: error.to_string(),
            })?;
        }
        self.run(&["add", "--all", "--force"])?;
        self.run(&["write-tree"])
    }

    /// The git tree the given snapshot produces, without pushing it.
    ///
    /// `--check` compares *this* with the remote, not the snapshot's content digest: the
    /// question is whether git would write a different tree, and only git can answer it.
    pub(crate) fn staged_tree(&self, tree: &ChannelTree) -> Result<String, TransportError> {
        self.write_tree(tree)
    }

    /// The tree hash the remote branch currently holds, if it holds one.
    fn remote_tree(&self) -> Result<Option<String>, TransportError> {
        let listing = self.run(&[
            "ls-remote",
            &self.remote,
            &format!("refs/heads/{}", self.branch),
        ])?;
        if listing.is_empty() {
            return Ok(None);
        }
        self.run(&[
            "fetch",
            "--quiet",
            "--depth",
            "1",
            &self.remote,
            &self.branch,
        ])?;
        Ok(Some(self.run(&["rev-parse", "FETCH_HEAD^{tree}"])?))
    }
}

/// A bearer of authority to push, handed to git out of band.
///
/// The value is a header, not a URL: embedding a token in `https://token@host/...` puts it in
/// argv for every `git` call, in `.git/config` once the remote is remembered, and in every
/// error message that quotes the remote. An `extraheader` scoped to one host's URL does none
/// of those, and applies to `ls-remote` and `fetch` as well as `push`, which is what a
/// transport that reads before it writes needs.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct PushCredential {
    /// The `git config` key, `http.<url>.extraheader` with the URL's host filled in.
    key: String,
    /// The header line git sends, `AUTHORIZATION: basic <base64>`.
    header: String,
}

impl PushCredential {
    /// The credential `token` authorizes, when `remote` is one a header can address.
    ///
    /// Returns `None` for an SSH or local remote, whose authority lives elsewhere.
    pub(crate) fn new(remote: &str, token: &str) -> Option<Self> {
        if token.trim().is_empty() {
            return None;
        }
        let origin = origin_of(remote)?;
        let value = if is_github(&origin) {
            // GitHub's documented shape for a token used as a password, and the shape
            // `actions/checkout` writes: any user name works, this one is conventional.
            format!(
                "AUTHORIZATION: basic {}",
                base64(format!("x-access-token:{token}").as_bytes())
            )
        } else {
            // Every other host this pipeline reaches takes the token directly, and a bearer
            // header is the one form that does not require guessing a user name.
            format!("Authorization: Bearer {token}")
        };
        Some(Self {
            key: format!("http.{origin}/.extraheader"),
            header: value,
        })
    }

    /// The environment that makes git read this credential.
    ///
    /// `GIT_CONFIG_COUNT` plus a numbered key and value is git's supported way of adding
    /// config for one command without a file, and it is inherited by the `git-remote-https`
    /// helper that actually speaks the protocol.
    fn apply(&self, command: &mut Command) {
        command
            .env("GIT_CONFIG_COUNT", "1")
            .env("GIT_CONFIG_KEY_0", &self.key)
            .env("GIT_CONFIG_VALUE_0", &self.header);
    }
}

/// The `scheme://host[:port]` a credential is scoped to, from any URL-shaped remote.
///
/// `None` for the scp-like `git@host:path` form and for a local path, neither of which is
/// spoken over HTTP.
fn origin_of(remote: &str) -> Option<String> {
    let (scheme, rest) = remote.split_once("://")?;
    if !scheme.eq_ignore_ascii_case("http") && !scheme.eq_ignore_ascii_case("https") {
        return None;
    }
    let authority = rest.split('/').next().unwrap_or(rest);
    // Credentials in the remote are dropped rather than carried: the header is the credential.
    let authority = authority
        .rsplit_once('@')
        .map_or(authority, |(_, host)| host);
    if authority.is_empty() {
        return None;
    }
    let (host, port) = match authority.rsplit_once(':') {
        Some((host, port)) if port.chars().all(|digit| digit.is_ascii_digit()) => {
            (host, Some(port))
        }
        _ => (authority, None),
    };
    if host.is_empty() || host.contains(':') {
        return None;
    }
    let host = host.to_ascii_lowercase();
    Some(match port {
        Some(port) => format!("{scheme}://{host}:{port}"),
        None => format!("{scheme}://{host}"),
    })
}

/// Whether `origin` addresses GitHub, whose token exchange has its own header shape.
fn is_github(origin: &str) -> bool {
    let host = origin
        .split_once("://")
        .map_or(origin, |(_, rest)| rest)
        .split(':')
        .next()
        .unwrap_or(origin);
    host == "github.com" || host.ends_with(".github.com")
}

/// Standard base64 with padding.
///
/// Sixteen lines rather than a dependency: the only base64 this crate needs is the user name
/// and token in one header, and a new dependency is a lockfile round trip on the relock bot.
fn base64(bytes: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut encoded = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let triple = [
            chunk[0],
            chunk.get(1).copied().unwrap_or(0),
            chunk.get(2).copied().unwrap_or(0),
        ];
        let word =
            (u32::from(triple[0]) << 16) | (u32::from(triple[1]) << 8) | u32::from(triple[2]);
        for index in 0..4 {
            if index <= chunk.len() {
                let sextet = (word >> (18 - 6 * index)) & 0b11_1111;
                encoded.push(char::from(ALPHABET[sextet as usize]));
            } else {
                encoded.push('=');
            }
        }
    }
    encoded
}

impl Transport for GitTransport {
    fn current(&self) -> Result<Option<TreeIdentity>, TransportError> {
        self.prepare()?;
        match self.remote_tree()? {
            None => Ok(None),
            Some(tree) => Ok(Some(TreeIdentity {
                commit: self.run(&["rev-parse", "FETCH_HEAD"])?,
                tree,
            })),
        }
    }

    fn push(&self, tree: &ChannelTree, message: &str) -> Result<PushOutcome, TransportError> {
        let staged = self.write_tree(tree)?;
        if self.remote_tree()?.as_deref() == Some(staged.as_str()) {
            return Ok(PushOutcome::Unchanged);
        }
        // No parent: the branch is a snapshot, not a changelog.
        let commit = self.run(&["commit-tree", &staged, "-m", message])?;
        self.run(&[
            "push",
            "--quiet",
            "--force",
            &self.remote,
            &format!("{commit}:refs/heads/{}", self.branch),
        ])?;
        Ok(PushOutcome::Published { commit })
    }
}

#[cfg(test)]
mod tests {
    use super::{GitTransport, PushCredential, base64, is_github, origin_of};
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::thread;
    use universe_channel::transport::Transport;

    #[test]
    fn base64_matches_the_padding_rules() {
        assert_eq!(base64(b""), "");
        assert_eq!(base64(b"f"), "Zg==");
        assert_eq!(base64(b"fo"), "Zm8=");
        assert_eq!(base64(b"foo"), "Zm9v");
        assert_eq!(base64(b"foob"), "Zm9vYg==");
        assert_eq!(base64(b"fooba"), "Zm9vYmE=");
        assert_eq!(base64(b"foobar"), "Zm9vYmFy");
        assert_eq!(base64(b"\xff\xfe\xfd"), "//79");
    }

    #[test]
    fn a_remote_is_scoped_to_its_origin() {
        assert_eq!(
            origin_of("https://github.com/archont561/universe.git").as_deref(),
            Some("https://github.com")
        );
        assert_eq!(
            origin_of("https://GitHub.com:443/owner/repo").as_deref(),
            Some("https://github.com:443")
        );
        // Credentials in the remote are the thing being replaced.
        assert_eq!(
            origin_of("https://user:secret@github.com/o/r").as_deref(),
            Some("https://github.com")
        );
        assert_eq!(
            origin_of("http://127.0.0.1:8080/o/r.git").as_deref(),
            Some("http://127.0.0.1:8080")
        );
        for remote in [
            "git@github.com:archont561/universe.git",
            "ssh://git@github.com/owner/repo.git",
            "./local/bare",
            "/srv/git/channel.git",
            "https://",
        ] {
            assert_eq!(origin_of(remote), None, "{remote}");
        }
    }

    #[test]
    fn github_is_recognised_by_host_and_not_by_lookalike() {
        assert!(is_github("https://github.com"));
        assert!(is_github("https://ghe.github.com:8443"));
        assert!(!is_github("https://github.com.evil.example"));
        assert!(!is_github("https://notgithub.com"));
    }

    #[test]
    fn github_gets_the_checkout_token_shape_and_others_a_bearer() {
        let github =
            PushCredential::new("https://github.com/o/r.git", "secret").expect("credential");
        assert_eq!(github.key, "http.https://github.com/.extraheader");
        assert_eq!(
            github.header,
            format!("AUTHORIZATION: basic {}", base64(b"x-access-token:secret"))
        );

        let other =
            PushCredential::new("https://git.example.org/o/r.git", "secret").expect("credential");
        assert_eq!(other.key, "http.https://git.example.org/.extraheader");
        assert_eq!(other.header, "Authorization: Bearer secret");
    }

    #[test]
    fn a_remote_a_header_cannot_reach_carries_no_credential() {
        assert_eq!(
            PushCredential::new("git@github.com:o/r.git", "secret"),
            None,
            "an ssh remote is the key's business"
        );
        assert_eq!(
            PushCredential::new("https://github.com/o/r.git", "  "),
            None,
            "a blank token is no token"
        );
    }

    /// A listener that answers one request with a `404` and returns what it was sent.
    ///
    /// Enough of HTTP to see whether git carried the header: the transport is about to fail
    /// either way, and the failure is not what the test is reading.
    fn one_404_server() -> (String, thread::JoinHandle<String>) {
        let listener = TcpListener::bind("127.0.0.1:0").expect("a listening socket");
        let port = listener.local_addr().expect("an address").port();
        let handle = thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("one connection");
            let mut buffer = [0_u8; 4096];
            let read = stream.read(&mut buffer).expect("a request");
            stream
                .write_all(b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\n\r\n")
                .expect("an answer");
            String::from_utf8_lossy(&buffer[..read]).into_owned()
        });
        (format!("http://127.0.0.1:{port}/channel.git"), handle)
    }

    #[test]
    fn a_supplied_token_reaches_git_as_a_request_header() {
        let (remote, server) = one_404_server();
        let scratch = tempfile::tempdir().expect("a scratch directory");
        let transport = GitTransport::new(&remote, "channel", scratch.path())
            .authenticated(Some("s3cret-token"));

        // The repository does not exist, so this fails — at the first request, which is what
        // the server under test is holding on to.
        assert!(transport.current().is_err(), "an empty remote is not found");

        let request = server.join().expect("the server thread");
        assert!(
            request.contains("Authorization: Bearer s3cret-token"),
            "git did not present the token; it sent:\n{request}"
        );
    }

    #[test]
    fn without_a_token_git_is_left_to_find_its_own_credentials() {
        let (remote, server) = one_404_server();
        let scratch = tempfile::tempdir().expect("a scratch directory");
        let transport = GitTransport::new(&remote, "channel", scratch.path());

        assert!(transport.current().is_err(), "an empty remote is not found");

        let request = server.join().expect("the server thread");
        assert!(
            !request.contains("Authorization:"),
            "an unauthenticated push must not invent a header:\n{request}"
        );
    }

    #[test]
    fn a_remote_that_cannot_take_a_header_is_pushed_without_one() {
        // `authenticated` narrows to a remote a header can address, so an SSH remote keeps the
        // behaviour it had: the key is the credential.
        let scratch = tempfile::tempdir().expect("a scratch directory");
        let transport =
            GitTransport::new("git@github.com:owner/repo.git", "channel", scratch.path())
                .authenticated(Some("s3cret-token"));
        assert_eq!(transport.credential, None);
    }
}
