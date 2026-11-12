//! A branch push from CI: where the credential comes from, and where it must not go.
//!
//! `publish` pushes a snapshot from a scratch repository it creates itself, so the token
//! `actions/checkout` left in the outer repository's config is invisible to it. The store's
//! manifest names a secret instead, the renderer exports it, and this verb hands it to git as
//! an `extraheader` through the environment — never on the command line, which every process
//! on the host can read, and never in output, which ends up in a public log.
//!
//! These tests drive the real binary so the environment is the mechanism under test rather
//! than something a test's own scope hides. The remote is a local bare repository: no network,
//! and git's local transport has no use for a token, so what is being asserted is that the
//! credential's presence changes nothing about a push that does not need it, and that it is
//! never echoed.

use std::path::Path;
use std::process::Command;

use tempfile::TempDir;
use universe_core::manifest::Manifest;
use universe_workflow::PUBLISH_FILE;

/// Deliberately invented fixture names: the rule under test is the store-prefixed spelling,
/// which must hold for *any* declared secret. The repository's own branch store names
/// `GITHUB_TOKEN` and renders `BRANCH_GITHUB_TOKEN`.
const TOKEN_VARIABLE: &str = "CHANNEL_BRANCH_FIXTURE_CHANNEL_TOKEN";
const TOKEN: &str = "published-from-a-workflow-secret";

struct Fixture {
    directory: TempDir,
}

impl Fixture {
    fn new() -> Self {
        let directory = tempfile::tempdir().expect("temp dir");
        let fixture = Self { directory };
        std::fs::write(fixture.root().join("universe.toml"), fixture.manifest()).expect("manifest");
        fixture.add_package();
        fixture
    }

    fn root(&self) -> &Path {
        self.directory.path()
    }

    /// A project with one package and one branch store that names a secret.
    fn manifest(&self) -> String {
        r#"
schema = 1

[project]
name = "fixture-credential"
platforms = ["linux-64"]

[[packages]]
name = "example-airlock"
kind = "conda"
command = ["cargo", "build"]
outputs = ["output/**/*.conda"]
smoke = ["example-airlock --version"]
publish = ["channel-branch"]

[[stores]]
name = "channel-branch"
kind = "branch"
writable = true
needs_index = true
auth = "token"
secrets = ["FIXTURE_CHANNEL_TOKEN"]
verifier = "file"

[workflow.checks]
format = "cargo fmt --check"
"#
        .to_owned()
    }

    fn add_package(&self) {
        let relative = "output/example-airlock-1.0.0-0.conda";
        let path = self.root().join(relative);
        std::fs::create_dir_all(path.parent().expect("parent")).expect("output dir");
        std::fs::write(&path, b"conda bytes").expect("artifact");
        let digest = universe_workflow::sha256::sha256_hex(b"conda bytes");
        let fragments = self.root().join("output/fragments");
        std::fs::create_dir_all(&fragments).expect("fragment dir");
        std::fs::write(
            fragments.join("example-airlock-linux-64.json"),
            format!(
                r#"{{
  "package": "example-airlock",
  "version": "1.0.0",
  "build": "0",
  "build_number": 0,
  "subdir": "linux-64",
  "digest": "{digest}",
  "size_bytes": 11,
  "depends": [],
  "license": "MIT",
  "license_files": [],
  "path": "{relative}"
}}
"#
            ),
        )
        .expect("fragment");
    }

    fn remote(&self) -> std::path::PathBuf {
        let path = self.root().join("remote.git");
        let status = Command::new("git")
            .args(["init", "--bare", "--quiet"])
            .arg(&path)
            .status()
            .expect("git is installed");
        assert!(status.success(), "the bare repository is created");
        path
    }

    /// The rendered publish workflow, so the variable this test exports is the one the
    /// renderer emits rather than one the test invented.
    fn rendered_publish_file(&self) -> String {
        let parsed = Manifest::from_toml(&self.manifest()).expect("the manifest parses");
        let output =
            universe_workflow::render(&parsed, &self.manifest()).expect("the workflow set renders");
        output
            .files
            .iter()
            .find(|file| file.path == PUBLISH_FILE)
            .expect("the publish file")
            .content
            .clone()
    }

    /// Run `universe publish` with the given environment added, as a job would.
    fn publish(&self, remote: &Path, environment: &[(&str, &str)]) -> std::process::Output {
        let mut command = Command::new(env!("CARGO_BIN_EXE_universe-kit"));
        command
            .args(["publish", "--store", "channel-branch", "--json", "--remote"])
            .arg(remote)
            .arg("--root")
            .arg(self.root());
        for (name, value) in environment {
            command.env(name, value);
        }
        // A prompt would hang the test rather than fail it; the ambient credential is not what
        // is under test.
        command.env("GIT_TERMINAL_PROMPT", "0");
        command.output().expect("run universe-kit publish")
    }
}

/// The ref the tool pushes to when the store names none, and the only place the default is
/// observable from outside: the fixture's manifest declares a branch store and no `branch`, so
/// what lands on the remote is the default rather than a declaration this test made.
const DEFAULT_REF: &str = "refs/heads/universe/conda-channel";

/// Whether the bare remote holds the snapshot at the default ref.
fn channel_branch_exists(remote: &Path) -> bool {
    let output = Command::new("git")
        .args(["ls-remote", "--heads"])
        .arg(remote)
        .output()
        .expect("git is installed");
    String::from_utf8_lossy(&output.stdout).contains(DEFAULT_REF)
}

#[test]
fn a_store_secret_in_the_environment_is_used_and_never_printed() {
    let fixture = Fixture::new();
    let remote = fixture.remote();

    // The name below is only meaningful if it is the name a job would actually export.
    let rendered = fixture.rendered_publish_file();
    assert!(
        rendered.contains(&format!(
            "{TOKEN_VARIABLE}: ${{{{ secrets.FIXTURE_CHANNEL_TOKEN }}}}"
        )),
        "the renderer exports a different variable than this test sets:\n{rendered}"
    );

    let output = fixture.publish(&remote, &[(TOKEN_VARIABLE, TOKEN)]);

    assert_eq!(
        output.status.code(),
        Some(0),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(channel_branch_exists(&remote), "the snapshot was pushed");
    let printed = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        !printed.contains(TOKEN),
        "the token reached the log:\n{printed}"
    );
    // The bare secret name is the fallback spelling a local shell would use, and it must not
    // appear in a message about something that worked.
    assert!(!printed.contains("FIXTURE_CHANNEL_TOKEN"), "{printed}");
}

#[test]
fn a_store_that_names_a_secret_the_environment_lacks_still_pushes() {
    let fixture = Fixture::new();
    let remote = fixture.remote();

    // A developer's machine: git's credential helper, an SSH agent or a cached token is the
    // authority, and a declared secret that no one exported must not turn the verb into a
    // refusal. (The credential itself is asserted at the socket in `git`'s unit tests; a local
    // remote has no use for one, which is why this test can only say nothing broke.)
    let output = fixture.publish(&remote, &[]);

    assert_eq!(
        output.status.code(),
        Some(0),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(channel_branch_exists(&remote), "the snapshot was pushed");
}
