//! `universe airlock mirror`, run against a lock, a local artifact server, and a real remote.
//!
//! The three pieces of this test are the three claims `UNI-51` makes, and none of them can be
//! observed from a unit test. The **lock** is the only source of what to fetch: the fixtures
//! below are text, and every URL in them points at a server this file starts. The **server** is
//! a `TcpListener` on the loopback interface answering `GET`s from a map, so a test can serve
//! bytes, serve wrong bytes, or serve nothing, and can count what was asked for — which is how
//! "a changed lock adds only the new artifacts" is checked rather than described. The
//! **remote** is a bare repository under a `TempDir` and the real `git` binary, so the branch
//! this test reads back is the branch a consumer would clone.
//!
//! Nothing here reaches the network, and no test writes outside its own temporary directories.

use std::collections::BTreeMap;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::thread;

use universe_workflow::sha256::sha256_hex;

/// A lock adapter's own file name in the fixture project, for the format under test.
const PIXI_LOCK: &str = "pixi.lock";
const CARGO_LOCK: &str = "Cargo.lock";

/// The branch a run writes when the caller names none and the manifest says `mirror`.
const PIXI_BRANCH: &str = "mirror/pixi";
const CARGO_BRANCH: &str = "mirror/cargo";

/// A manifest with a mirror prefix and nothing else: a project that pins its toolchain before
/// it ships an entrypoint, which is the order this verb exists in.
fn manifest() -> String {
    r#"
schema = 1

[project]
name = "fixture-mirror"
platforms = ["linux-64"]

[airlock.developer]
adapters = ["pixi", "cargo"]
mirror = "mirror"
"#
    .to_owned()
}

/// What one artifact is made of: the file name it keeps and the bytes served for it.
#[derive(Clone)]
struct Artifact {
    /// Path below the server's root, which is also the tail of its URL.
    path: String,
    /// The bytes the server answers with.
    bytes: Vec<u8>,
}

impl Artifact {
    fn new(path: &str, bytes: &[u8]) -> Self {
        Self {
            path: path.to_owned(),
            bytes: bytes.to_vec(),
        }
    }

    fn digest(&self) -> String {
        sha256_hex(&self.bytes)
    }

    fn filename(&self) -> String {
        self.path
            .rsplit('/')
            .next()
            .unwrap_or(&self.path)
            .to_owned()
    }

    /// Where the mirror puts it, which is the digest and the file name.
    fn mirrored(&self) -> String {
        format!(
            "blobs/sha256/{}/{}/{}",
            &self.digest()[..2],
            self.digest(),
            self.filename()
        )
    }
}

/// A local artifact server, and the record of what it was asked for.
struct Server {
    base: String,
    hits: Arc<Mutex<Vec<String>>>,
}

impl Server {
    fn start(artifacts: &[Artifact]) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind a loopback port");
        let port = listener.local_addr().expect("the bound address").port();
        let files: BTreeMap<String, Vec<u8>> = artifacts
            .iter()
            .map(|artifact| (format!("/{}", artifact.path), artifact.bytes.clone()))
            .collect();
        let hits = Arc::new(Mutex::new(Vec::new()));
        let recorded = Arc::clone(&hits);
        // Detached on purpose: the test owns it for its whole life and the listener closes when
        // the process exits. Joining would need a shutdown protocol that observes nothing.
        thread::spawn(move || {
            for stream in listener.incoming() {
                let Ok(mut stream) = stream else {
                    continue;
                };
                let mut buffer = [0_u8; 8192];
                let read = stream.read(&mut buffer).unwrap_or(0);
                let request = String::from_utf8_lossy(&buffer[..read]).into_owned();
                let path = request.split_whitespace().nth(1).unwrap_or("/").to_owned();
                recorded.lock().expect("the hit record").push(path.clone());
                let response = match files.get(&path) {
                    Some(bytes) => {
                        let mut response = format!(
                            "HTTP/1.1 200 OK\r\nContent-Type: application/octet-stream\r\n\
                             Content-Length: {}\r\nConnection: close\r\n\r\n",
                            bytes.len()
                        )
                        .into_bytes();
                        response.extend_from_slice(bytes);
                        response
                    }
                    None => {
                        b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
                            .to_vec()
                    }
                };
                let _ = stream.write_all(&response);
                let _ = stream.flush();
            }
        });
        Self {
            base: format!("http://127.0.0.1:{port}"),
            hits,
        }
    }

    fn url(&self, path: &str) -> String {
        if path.is_empty() {
            // A base URL, for `--crates`: the adapter appends `/name/name-version.crate`.
            return self.base.clone();
        }
        format!("{}/{}", self.base, path)
    }

    /// Every path this server was asked for, in order.
    fn hits(&self) -> Vec<String> {
        self.hits.lock().expect("the hit record").clone()
    }
}

/// A project, a bare remote, and the helpers the tests share.
struct Fixture {
    project: tempfile::TempDir,
    remote: tempfile::TempDir,
}

impl Fixture {
    fn new() -> Self {
        let project = tempfile::tempdir().expect("a project directory");
        let remote = tempfile::tempdir().expect("a remote directory");
        std::fs::write(project.path().join("universe.toml"), manifest()).expect("manifest");
        git(&["init", "--bare", "--quiet", "remote.git"], remote.path());
        Self { project, remote }
    }

    fn remote(&self) -> PathBuf {
        self.remote.path().join("remote.git")
    }

    fn write(&self, name: &str, text: &str) {
        std::fs::write(self.project.path().join(name), text).expect("the fixture file");
    }

    /// Run `universe airlock mirror` against this fixture, with `extra` arguments appended.
    fn mirror(&self, extra: &[&str]) -> (i32, String, String) {
        let mut arguments: Vec<String> = vec![
            "airlock".to_owned(),
            "mirror".to_owned(),
            "--root".to_owned(),
            self.project.path().display().to_string(),
            "--remote".to_owned(),
            self.remote().display().to_string(),
        ];
        arguments.extend(extra.iter().map(|argument| (*argument).to_owned()));
        let output = universe_kit::run(arguments);
        (
            i32::from(output.exit_code()),
            output.stdout().to_owned(),
            output.stderr().to_owned(),
        )
    }

    /// Every file on `branch`, by path.
    fn branch(&self, branch: &str) -> BTreeMap<String, Vec<u8>> {
        let listing = git_ok(
            &[
                "--git-dir",
                &self.remote().display().to_string(),
                "ls-tree",
                "-r",
                "--name-only",
                branch,
            ],
            self.remote.path(),
        );
        let mut files = BTreeMap::new();
        for path in listing.lines().filter(|line| !line.is_empty()) {
            let bytes = git(
                &[
                    "--git-dir",
                    &self.remote().display().to_string(),
                    "cat-file",
                    "-p",
                    &format!("{branch}:{path}"),
                ],
                self.remote.path(),
            );
            files.insert(path.to_owned(), bytes.stdout);
        }
        files
    }

    /// The commit `branch` points at, or an empty string when the branch does not exist.
    fn commit(&self, branch: &str) -> String {
        let output = Command::new("git")
            .args([
                "--git-dir",
                &self.remote().display().to_string(),
                "rev-parse",
                "--verify",
                "--quiet",
                &format!("refs/heads/{branch}"),
            ])
            .current_dir(self.remote.path())
            .output()
            .expect("git rev-parse");
        if !output.status.success() {
            return String::new();
        }
        String::from_utf8_lossy(&output.stdout).trim().to_owned()
    }
}

fn git(arguments: &[&str], directory: &Path) -> Output {
    let output = Command::new("git")
        .args(arguments)
        .current_dir(directory)
        .output()
        .expect("run git");
    assert!(
        output.status.success(),
        "git {arguments:?} failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    Output {
        stdout: output.stdout,
    }
}

fn git_ok(arguments: &[&str], directory: &Path) -> String {
    String::from_utf8_lossy(&git(arguments, directory).stdout).into_owned()
}

struct Output {
    stdout: Vec<u8>,
}

/// A `pixi.lock` naming the given artifact URLs, with their real digests.
fn pixi_lock(urls: &[&str], digests: &[(String, u64)]) -> String {
    let mut text = String::from("version: 7\npackages:\n");
    for (url, (digest, size)) in urls.iter().zip(digests) {
        let filename = url.rsplit('/').next().unwrap_or(url);
        if let Some(stem) = filename.strip_suffix(".whl") {
            // A PyPI record carries its own identity, so the fixture does too.
            let fields: Vec<&str> = stem.split('-').collect();
            text.push_str(&format!(
                "- pypi: {url}\n  name: {}\n  version: {}\n  sha256: {digest}\n",
                fields[0], fields[1]
            ));
        } else {
            text.push_str(&format!(
                "- conda: {url}\n  sha256: {digest}\n  size: {size}\n"
            ));
        }
    }
    text
}

/// One Conda artifact as a lock would name it.
fn conda_artifact(name: &str, version: &str) -> Artifact {
    let bytes = format!("the bytes of {name} {version}\n").into_bytes();
    Artifact::new(
        &format!("conda-forge/linux-64/{name}-{version}-h1234567_0.conda"),
        &bytes,
    )
}

#[test]
fn a_lock_is_mirrored_into_a_content_addressed_branch() {
    let artifact = conda_artifact("libgcc", "16.2.0");
    let server = Server::start(std::slice::from_ref(&artifact));
    let url = server.url(&artifact.path);
    let fixture = Fixture::new();
    fixture.write(
        PIXI_LOCK,
        &pixi_lock(&[&url], &[(artifact.digest(), artifact.bytes.len() as u64)]),
    );

    let (code, stdout, stderr) = fixture.mirror(&["--adapter", "pixi", "--json"]);
    assert_eq!(code, 0, "stderr: {stderr}\nstdout: {stdout}");

    // The branch holds the artifact under its digest, the index, and nothing else.
    let files = fixture.branch(PIXI_BRANCH);
    let mut expected: Vec<&String> = files.keys().collect();
    expected.sort();
    assert_eq!(
        expected,
        vec![&artifact.mirrored(), &"mirror.json".to_owned()],
        "unexpected tree: {expected:?}"
    );
    assert_eq!(
        files.get(&artifact.mirrored()).expect("the artifact"),
        &artifact.bytes,
        "the published bytes are not the served bytes"
    );
    let index: serde_json::Value =
        serde_json::from_slice(files.get("mirror.json").expect("the index")).expect("JSON");
    assert_eq!(index["schema"], 1);
    assert_eq!(index["adapter"], "pixi");
    assert_eq!(index["artifacts"][0]["name"], "libgcc");
    assert_eq!(index["artifacts"][0]["version"], "16.2.0");
    assert_eq!(index["artifacts"][0]["sha256"], artifact.digest());
    assert_eq!(index["artifacts"][0]["path"], artifact.mirrored());
    assert_eq!(index["artifacts"][0]["size"], artifact.bytes.len());
    assert_eq!(server.hits(), vec![format!("/{}", artifact.path)]);
    assert!(!fixture.commit(PIXI_BRANCH).is_empty());
    assert!(stdout.contains("\"fetched\":1"), "stdout: {stdout}");
}

#[test]
fn republishing_an_unchanged_lock_produces_no_new_commit() {
    let artifact = conda_artifact("libgcc", "16.2.0");
    let server = Server::start(std::slice::from_ref(&artifact));
    let url = server.url(&artifact.path);
    let fixture = Fixture::new();
    fixture.write(
        PIXI_LOCK,
        &pixi_lock(&[&url], &[(artifact.digest(), artifact.bytes.len() as u64)]),
    );

    let (code, _, stderr) = fixture.mirror(&["--adapter", "pixi"]);
    assert_eq!(code, 0, "stderr: {stderr}");
    let first = fixture.commit(PIXI_BRANCH);
    assert!(!first.is_empty());

    let (code, stdout, stderr) = fixture.mirror(&["--adapter", "pixi", "--json"]);
    assert_eq!(code, 0, "stderr: {stderr}");
    assert_eq!(
        fixture.commit(PIXI_BRANCH),
        first,
        "an unchanged lock moved the branch"
    );
    assert!(
        stdout.contains("\"unchanged\":true"),
        "the run did not report the destination agreeing: {stdout}"
    );
    // The second run read the mirror instead of the server.
    assert_eq!(
        server.hits().len(),
        1,
        "the cached artifact was fetched again: {:?}",
        server.hits()
    );
}

#[test]
fn a_changed_lock_adds_only_the_new_artifact() {
    let first = conda_artifact("libgcc", "16.2.0");
    let second = conda_artifact("libstdcxx", "16.2.0");
    let server = Server::start(&[first.clone(), second.clone()]);
    let fixture = Fixture::new();
    let lock = |artifacts: &[&Artifact]| {
        let urls: Vec<String> = artifacts
            .iter()
            .map(|artifact| server.url(&artifact.path))
            .collect();
        let urls: Vec<&str> = urls.iter().map(String::as_str).collect();
        let digests: Vec<(String, u64)> = artifacts
            .iter()
            .map(|artifact| (artifact.digest(), artifact.bytes.len() as u64))
            .collect();
        pixi_lock(&urls, &digests)
    };

    fixture.write(PIXI_LOCK, &lock(&[&first]));
    let (code, _, stderr) = fixture.mirror(&["--adapter", "pixi", "--json"]);
    assert_eq!(code, 0, "stderr: {stderr}");
    let before = fixture.commit(PIXI_BRANCH);

    fixture.write(PIXI_LOCK, &lock(&[&first, &second]));
    let (code, stdout, stderr) = fixture.mirror(&["--adapter", "pixi", "--json"]);
    assert_eq!(code, 0, "stderr: {stderr}");
    assert_ne!(
        fixture.commit(PIXI_BRANCH),
        before,
        "the branch did not move"
    );

    let files = fixture.branch(PIXI_BRANCH);
    assert!(
        files.contains_key(&first.mirrored()),
        "the old artifact left"
    );
    assert!(
        files.contains_key(&second.mirrored()),
        "the new artifact did not arrive"
    );
    // Exactly one new artifact was fetched: the first was already in the mirror, and the
    // server answers one path per artifact.
    assert_eq!(
        server.hits(),
        vec![format!("/{}", first.path), format!("/{}", second.path)],
        "the changed lock refetched something it already had"
    );
    assert!(stdout.contains("\"fetched\":1"), "stdout: {stdout}");
}

#[test]
fn a_tampered_artifact_is_refused_and_nothing_is_published() {
    let artifact = conda_artifact("libgcc", "16.2.0");
    let tampered = Artifact::new(&artifact.path, b"not the bytes the lock pinned\n");
    let server = Server::start(std::slice::from_ref(&tampered));
    let url = server.url(&artifact.path);
    let fixture = Fixture::new();
    fixture.write(
        PIXI_LOCK,
        &pixi_lock(&[&url], &[(artifact.digest(), tampered.bytes.len() as u64)]),
    );

    let (code, stdout, stderr) = fixture.mirror(&["--adapter", "pixi"]);
    assert_eq!(code, 1, "a digest mismatch is a finding, not a warning");
    assert!(
        stdout.is_empty(),
        "diagnostics never land on stdout: {stdout}"
    );
    assert!(
        stderr.contains("airlock.mirror.fetch.digest"),
        "stderr: {stderr}"
    );
    assert!(
        stderr.contains("libgcc 16.2.0"),
        "the refusal does not name the package: {stderr}"
    );
    assert!(
        fixture.commit(PIXI_BRANCH).is_empty(),
        "a refused run published a branch anyway"
    );
}

#[test]
fn a_missing_artifact_is_refused_and_named() {
    let artifact = conda_artifact("libgcc", "16.2.0");
    // The server holds nothing, so every request answers 404.
    let server = Server::start(&[]);
    let url = server.url(&artifact.path);
    let fixture = Fixture::new();
    fixture.write(
        PIXI_LOCK,
        &pixi_lock(&[&url], &[(artifact.digest(), artifact.bytes.len() as u64)]),
    );

    let (code, _, stderr) = fixture.mirror(&["--adapter", "pixi"]);
    assert_eq!(code, 1);
    assert!(
        stderr.contains("airlock.mirror.fetch.request"),
        "stderr: {stderr}"
    );
    assert!(
        stderr.contains("404"),
        "the status is not reported: {stderr}"
    );
    assert!(stderr.contains("libgcc 16.2.0"), "stderr: {stderr}");
}

#[test]
fn a_lock_that_pins_no_artifact_is_refused_rather_than_published_empty() {
    let fixture = Fixture::new();
    fixture.write(PIXI_LOCK, "version: 7\npackages: []\n");
    let (code, _, stderr) = fixture.mirror(&["--adapter", "pixi"]);
    assert_eq!(code, 1);
    assert!(stderr.contains("airlock.mirror.empty"), "stderr: {stderr}");
}

#[test]
fn a_cargo_lock_is_mirrored_through_the_same_verb() {
    let bytes = b"a crate archive, in the registry's own layout\n".to_vec();
    let digest = sha256_hex(&bytes);
    let artifact = Artifact::new("serde/serde-1.0.200.crate", &bytes);
    let server = Server::start(std::slice::from_ref(&artifact));
    let fixture = Fixture::new();
    fixture.write(
        CARGO_LOCK,
        &format!(
            "version = 4\n\n[[package]]\nname = \"serde\"\nversion = \"1.0.200\"\n\
             source = \"registry+https://github.com/rust-lang/crates.io-index\"\n\
             checksum = \"{digest}\"\n\n\
             [[package]]\nname = \"fixture-mirror\"\nversion = \"0.1.0\"\n"
        ),
    );

    // The same verb, one flag different, and no CLI shape change for the new format.
    let (code, stdout, stderr) =
        fixture.mirror(&["--adapter", "cargo", "--crates", &server.url(""), "--json"]);
    assert_eq!(code, 0, "stderr: {stderr}\nstdout: {stdout}");
    let files = fixture.branch(CARGO_BRANCH);
    assert!(
        files.contains_key(&format!(
            "blobs/sha256/{}/{}/serde-1.0.200.crate",
            &digest[..2],
            digest
        )),
        "the crate is not in the tree: {:?}",
        files.keys().collect::<Vec<_>>()
    );
    let index: serde_json::Value =
        serde_json::from_slice(files.get("mirror.json").expect("the index")).expect("JSON");
    assert_eq!(index["adapter"], "cargo");
    assert_eq!(index["artifacts"][0]["kind"], "crate");
    assert_eq!(index["artifacts"][0]["subdir"], "any");
    assert!(
        index["artifacts"][0]["url"]
            .as_str()
            .expect("a url")
            .contains("/serde/serde-1.0.200.crate"),
        "the index does not record where the bytes came from: {index}"
    );
}

#[test]
fn a_git_dependency_is_refused_before_anything_is_fetched() {
    let fixture = Fixture::new();
    fixture.write(
        CARGO_LOCK,
        "version = 4\n\n[[package]]\nname = \"patched\"\nversion = \"0.1.0\"\n\
         source = \"git+https://example.invalid/repo?rev=abc#abcdef\"\n",
    );
    let (code, _, stderr) = fixture.mirror(&["--adapter", "cargo"]);
    assert_eq!(code, 1);
    assert!(
        stderr.contains("airlock.mirror.lock.source"),
        "stderr: {stderr}"
    );
    assert!(stderr.contains("patched 0.1.0"), "stderr: {stderr}");
}

#[test]
fn an_adapter_this_build_cannot_read_is_a_usage_error_naming_the_ones_it_can() {
    let fixture = Fixture::new();
    let (code, _, stderr) = fixture.mirror(&["--adapter", "bun"]);
    assert_eq!(code, 2, "an unsupported format is the invocation's problem");
    assert!(
        stderr.contains("airlock.mirror.adapter.unknown"),
        "stderr: {stderr}"
    );
    assert!(stderr.contains("pixi, cargo"), "stderr: {stderr}");
}

#[test]
fn a_dry_run_fetches_nothing_and_publishes_nothing() {
    let artifact = conda_artifact("libgcc", "16.2.0");
    let server = Server::start(std::slice::from_ref(&artifact));
    let url = server.url(&artifact.path);
    let fixture = Fixture::new();
    fixture.write(
        PIXI_LOCK,
        &pixi_lock(&[&url], &[(artifact.digest(), artifact.bytes.len() as u64)]),
    );

    let (code, stdout, stderr) = fixture.mirror(&["--adapter", "pixi", "--dry-run", "--json"]);
    assert_eq!(code, 0, "stderr: {stderr}");
    assert!(server.hits().is_empty(), "a dry run fetched bytes");
    assert!(
        fixture.commit(PIXI_BRANCH).is_empty(),
        "a dry run published"
    );
    assert!(stdout.contains("\"would_fetch\":1"), "stdout: {stdout}");
    assert!(stdout.contains("\"fetched\":0"), "stdout: {stdout}");
}

#[test]
fn a_check_reports_the_destination_agreeing_without_writing_to_it() {
    let artifact = conda_artifact("libgcc", "16.2.0");
    let server = Server::start(std::slice::from_ref(&artifact));
    let url = server.url(&artifact.path);
    let fixture = Fixture::new();
    fixture.write(
        PIXI_LOCK,
        &pixi_lock(&[&url], &[(artifact.digest(), artifact.bytes.len() as u64)]),
    );

    let (code, _, stderr) = fixture.mirror(&["--adapter", "pixi"]);
    assert_eq!(code, 0, "stderr: {stderr}");
    let published = fixture.commit(PIXI_BRANCH);

    let (code, stdout, stderr) = fixture.mirror(&["--adapter", "pixi", "--check", "--json"]);
    assert_eq!(code, 0, "stderr: {stderr}");
    assert!(stdout.contains("\"matched\":true"), "stdout: {stdout}");
    assert_eq!(
        fixture.commit(PIXI_BRANCH),
        published,
        "a check moved the branch"
    );

    // A lock the branch does not hold is reported as a difference, not as a match.
    let other = conda_artifact("libstdcxx", "16.2.0");
    let server = Server::start(std::slice::from_ref(&other));
    let url = server.url(&other.path);
    fixture.write(
        PIXI_LOCK,
        &pixi_lock(&[&url], &[(other.digest(), other.bytes.len() as u64)]),
    );
    let (code, stdout, stderr) = fixture.mirror(&["--adapter", "pixi", "--check", "--json"]);
    assert_eq!(code, 0, "stderr: {stderr}");
    assert!(stdout.contains("\"matched\":false"), "stdout: {stdout}");
    assert_eq!(fixture.commit(PIXI_BRANCH), published, "a check published");
}

#[test]
fn a_republish_that_changes_nothing_says_so_in_a_line() {
    let artifact = conda_artifact("libgcc", "16.2.0");
    let server = Server::start(std::slice::from_ref(&artifact));
    let url = server.url(&artifact.path);
    let fixture = Fixture::new();
    fixture.write(
        PIXI_LOCK,
        &pixi_lock(&[&url], &[(artifact.digest(), artifact.bytes.len() as u64)]),
    );

    let (code, stdout, stderr) = fixture.mirror(&["--adapter", "pixi"]);
    assert_eq!(code, 0, "stderr: {stderr}");
    assert!(
        stdout.contains("mirrored 1 artifact(s) for pixi into mirror/pixi: 1 fetched"),
        "stdout: {stdout}"
    );
    let (code, stdout, stderr) = fixture.mirror(&["--adapter", "pixi"]);
    assert_eq!(code, 0, "stderr: {stderr}");
    assert!(
        stdout.contains("already holds 1 artifact(s) for pixi; nothing was published"),
        "stdout: {stdout}"
    );
}

#[test]
fn a_corrupt_cache_is_fetched_again_rather_than_published() {
    let artifact = conda_artifact("libgcc", "16.2.0");
    let server = Server::start(std::slice::from_ref(&artifact));
    let url = server.url(&artifact.path);
    let fixture = Fixture::new();
    fixture.write(
        PIXI_LOCK,
        &pixi_lock(&[&url], &[(artifact.digest(), artifact.bytes.len() as u64)]),
    );
    let (code, _, stderr) = fixture.mirror(&["--adapter", "pixi"]);
    assert_eq!(code, 0, "stderr: {stderr}");

    // A truncated download from last week is exactly what a mirror would otherwise publish.
    let cached = fixture
        .project
        .path()
        .join(".universe-airlock/mirror/pixi")
        .join(artifact.mirrored());
    std::fs::write(&cached, b"half a file").expect("corrupt the cache");

    let (code, stdout, stderr) = fixture.mirror(&["--adapter", "pixi"]);
    assert_eq!(code, 0, "stderr: {stderr}");
    assert_eq!(
        server.hits().len(),
        2,
        "the corrupt blob was not fetched again"
    );
    // The bytes are the ones the lock pins, so the tree is the one the branch already holds:
    // nothing is published, and the corrupt file is gone from the mirror either way.
    assert!(
        stdout.contains("nothing was published"),
        "the tree should have been unchanged: {stdout}"
    );
    let files = fixture.branch(PIXI_BRANCH);
    assert_eq!(
        files.get(&artifact.mirrored()).expect("the artifact"),
        &artifact.bytes,
        "the branch kept the corrupt bytes"
    );
}

#[test]
fn an_artifact_of_the_wrong_size_is_refused_even_with_the_right_digest() {
    let artifact = conda_artifact("libgcc", "16.2.0");
    let server = Server::start(std::slice::from_ref(&artifact));
    let url = server.url(&artifact.path);
    let fixture = Fixture::new();
    // The digest is right and the size is not: both are pins, so both are checked.
    fixture.write(PIXI_LOCK, &pixi_lock(&[&url], &[(artifact.digest(), 9999)]));

    let (code, _, stderr) = fixture.mirror(&["--adapter", "pixi"]);
    assert_eq!(code, 1);
    assert!(
        stderr.contains("airlock.mirror.fetch.size"),
        "stderr: {stderr}"
    );
    assert!(
        stderr.contains("9999"),
        "the recorded size is not reported: {stderr}"
    );
}

#[test]
fn a_remote_that_refuses_the_push_is_reported_as_the_transport_failing() {
    let artifact = conda_artifact("libgcc", "16.2.0");
    let server = Server::start(std::slice::from_ref(&artifact));
    let url = server.url(&artifact.path);
    let fixture = Fixture::new();
    fixture.write(
        PIXI_LOCK,
        &pixi_lock(&[&url], &[(artifact.digest(), artifact.bytes.len() as u64)]),
    );
    // A directory that is not a repository: `git ls-remote` fails rather than answering.
    let nowhere = tempfile::tempdir().expect("a directory");
    let (code, _, stderr) = fixture.mirror(&[
        "--adapter",
        "pixi",
        "--remote",
        &nowhere.path().display().to_string(),
    ]);
    assert_eq!(code, 1);
    assert!(
        stderr.contains("airlock.mirror.transport.failed"),
        "stderr: {stderr}"
    );
}

#[test]
fn a_lock_outside_the_project_root_is_read_from_where_it_is_named() {
    let artifact = conda_artifact("libgcc", "16.2.0");
    let server = Server::start(std::slice::from_ref(&artifact));
    let url = server.url(&artifact.path);
    let fixture = Fixture::new();
    std::fs::create_dir_all(fixture.project.path().join("locks")).expect("a locks directory");
    let text = pixi_lock(&[&url], &[(artifact.digest(), artifact.bytes.len() as u64)]);
    fixture.write("locks/pixi.yaml", &text);

    // Relative to the root, and then by absolute path: both name the same file.
    let (code, _, stderr) = fixture.mirror(&["--adapter", "pixi", "--lock", "locks/pixi.yaml"]);
    assert_eq!(code, 0, "stderr: {stderr}");
    let absolute = fixture.project.path().join("locks/pixi.yaml");
    let (code, _, stderr) = fixture.mirror(&[
        "--adapter",
        "pixi",
        "--lock",
        &absolute.display().to_string(),
        "--check",
    ]);
    assert_eq!(code, 0, "stderr: {stderr}");
}

#[test]
fn a_missing_lock_is_a_finding_rather_than_a_usage_error() {
    let fixture = Fixture::new();
    let (code, _, stderr) = fixture.mirror(&["--adapter", "pixi"]);
    assert_eq!(
        code, 1,
        "a repository without a lock is the repository's problem"
    );
    assert!(
        stderr.contains("airlock.mirror.lock.missing"),
        "stderr: {stderr}"
    );
}

#[test]
fn a_mirror_directory_can_be_put_where_the_caller_names_it() {
    let artifact = conda_artifact("libgcc", "16.2.0");
    let server = Server::start(std::slice::from_ref(&artifact));
    let url = server.url(&artifact.path);
    let fixture = Fixture::new();
    fixture.write(
        PIXI_LOCK,
        &pixi_lock(&[&url], &[(artifact.digest(), artifact.bytes.len() as u64)]),
    );
    let elsewhere = tempfile::tempdir().expect("a mirror directory");
    let mirror = elsewhere.path().join("kept");
    let (code, _, stderr) = fixture.mirror(&[
        "--adapter",
        "pixi",
        "--mirror",
        &mirror.display().to_string(),
    ]);
    assert_eq!(code, 0, "stderr: {stderr}");
    assert!(
        mirror.join("pixi").join(artifact.mirrored()).exists(),
        "the artifact did not land where the caller asked"
    );
    assert!(
        !fixture.project.path().join(".universe-airlock").exists(),
        "the default directory was written anyway"
    );
}

#[test]
fn a_dry_run_says_what_it_would_do_in_a_line() {
    let artifact = conda_artifact("libgcc", "16.2.0");
    let server = Server::start(std::slice::from_ref(&artifact));
    let url = server.url(&artifact.path);
    let fixture = Fixture::new();
    fixture.write(
        PIXI_LOCK,
        &pixi_lock(&[&url], &[(artifact.digest(), artifact.bytes.len() as u64)]),
    );
    let (code, stdout, stderr) = fixture.mirror(&["--adapter", "pixi", "--dry-run"]);
    assert_eq!(code, 0, "stderr: {stderr}");
    assert!(
        stdout.contains("would mirror 1 artifact(s) for pixi into mirror/pixi: 1 to fetch"),
        "stdout: {stdout}"
    );
    assert!(server.hits().is_empty(), "a dry run fetched bytes");
}

#[test]
fn a_check_says_whether_the_branch_matches_in_a_line() {
    let artifact = conda_artifact("libgcc", "16.2.0");
    let server = Server::start(std::slice::from_ref(&artifact));
    let url = server.url(&artifact.path);
    let fixture = Fixture::new();
    fixture.write(
        PIXI_LOCK,
        &pixi_lock(&[&url], &[(artifact.digest(), artifact.bytes.len() as u64)]),
    );
    let (code, _, stderr) = fixture.mirror(&["--adapter", "pixi"]);
    assert_eq!(code, 0, "stderr: {stderr}");

    let (code, stdout, stderr) = fixture.mirror(&["--adapter", "pixi", "--check"]);
    assert_eq!(code, 0, "stderr: {stderr}");
    assert!(
        stdout.starts_with("mirror/pixi already holds what this lock pins (tree "),
        "stdout: {stdout}"
    );

    let other = conda_artifact("libstdcxx", "16.2.0");
    let server = Server::start(std::slice::from_ref(&other));
    let url = server.url(&other.path);
    fixture.write(
        PIXI_LOCK,
        &pixi_lock(&[&url], &[(other.digest(), other.bytes.len() as u64)]),
    );
    let (code, stdout, stderr) = fixture.mirror(&["--adapter", "pixi", "--check"]);
    assert_eq!(code, 0, "stderr: {stderr}");
    assert!(
        stdout.starts_with("mirror/pixi differs from what this lock would publish"),
        "stdout: {stdout}"
    );
}
