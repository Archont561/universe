//! The channel a tag publishes is one a solver can use.
//!
//! Every other test about `publish` asks whether *we* can read back what we wrote. This one
//! asks the question the milestone is actually about (`m-12`, exit condition 4): a developer
//! points a fresh `pixi.toml` at the branch, and `universe-airlock` resolves. Three things
//! have to be true at once and none of them is visible from inside this repository's own
//! JSON reader — the metadata has to be in the layout a solver expects (`linux-64/` and
//! `noarch/` repodata, `.conda` entries under `packages.conda`), the bytes have to sit where
//! the metadata says, and the client's digest check has to agree with both.
//!
//! The transport is the real one: `universe publish` pushes to a bare repository, and the
//! consumer side is `git clone` — because the premise of a branch channel is a host whose
//! only reachable thing is Git. Neither the clone nor the solve is stubbed, and the solve is
//! `--offline`, so nothing here can pass by reaching a network the sandbox does not have.
//!
//! `pixi` itself is pinned by this repository's own toolchain rather than guessed: it is on
//! `PATH` when the gates run under `pixi run`, and the restored tool is used when it is not.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;

use tempfile::TempDir;
use universe_core::CommandOutput;

/// The package the fixture publishes, and the file the solve is asked to install.
const PACKAGE: &str = "example-airlock";
const VERSION: &str = "1.0.0";
const BINARY: &str = "bin/example-airlock";

/// A project whose single package publishes to one branch store.
fn manifest() -> String {
    format!(
        r#"
schema = 1

[project]
name = "fixture-resolves"
platforms = ["linux-64"]

[[packages]]
name = "{PACKAGE}"
kind = "conda"
command = ["cargo", "build"]
outputs = ["output/**/*.conda"]
smoke = ["{PACKAGE} --version"]
publish = ["channel-branch"]

[[stores]]
name = "channel-branch"
kind = "branch"
writable = true
needs_index = true
auth = "token"
verifier = "file"

[workflow.checks]
format = "cargo fmt --check"
"#
    )
}

struct Fixture {
    directory: TempDir,
}

impl Fixture {
    fn new() -> Self {
        let directory = tempfile::tempdir().expect("temp dir");
        let fixture = Self { directory };
        std::fs::write(fixture.root().join("universe.toml"), manifest()).expect("manifest");
        fixture
    }

    fn root(&self) -> &Path {
        self.directory.path()
    }

    /// Write the archive, and the fragment that describes the bytes just written.
    ///
    /// The fragment is written rather than produced by `universe audit`: the subject here is
    /// the channel a publish leaves behind, and the audit's own output is proved elsewhere.
    fn add_package(&self) {
        let relative = format!("output/{PACKAGE}-{VERSION}-0.conda");
        let archive = self.root().join(&relative);
        std::fs::create_dir_all(archive.parent().expect("parent")).expect("output dir");
        let bytes = conda_archive();
        std::fs::write(&archive, &bytes).expect("archive");

        let fragments = self.root().join("output/fragments");
        std::fs::create_dir_all(&fragments).expect("fragment dir");
        std::fs::write(
            fragments.join(format!("{PACKAGE}-linux-64.json")),
            format!(
                r#"{{
  "package": "{PACKAGE}",
  "version": "{VERSION}",
  "build": "0",
  "build_number": 0,
  "subdir": "linux-64",
  "digest": "{}",
  "size_bytes": {},
  "depends": [],
  "license": "MIT",
  "license_files": [],
  "path": "{relative}"
}}
"#,
                universe_workflow::sha256::sha256_hex(&bytes),
                bytes.len()
            ),
        )
        .expect("fragment");
    }

    fn remote(&self) -> PathBuf {
        let path = self.root().join("remote.git");
        let status = Command::new("git")
            .args(["init", "--bare", "--quiet"])
            .arg(&path)
            .status()
            .expect("git is installed");
        assert!(status.success(), "the bare repository is created");
        path
    }
}

/// A `.conda` a client can install: metadata, a file list, and one executable.
///
/// Built here rather than committed: a binary fixture is opaque, and this spells out the
/// three members a `conda` 2 format archive is read for.
fn conda_archive() -> Vec<u8> {
    let zstd = |data: &[u8]| zstd::encode_all(data, 0).expect("compress");
    let index = format!(
        r#"{{"name":"{PACKAGE}","version":"{VERSION}","build":"0","build_number":0,"subdir":"linux-64","depends":[],"license":"MIT"}}"#
    );
    let file_list = format!("{BINARY}\n");
    let info = zstd(&tar(&[
        ("info/index.json", index.as_bytes()),
        ("info/files", file_list.as_bytes()),
        ("info/about.json", br#"{"license":"MIT"}"#),
    ]));
    let payload = zstd(&tar(&[(
        BINARY,
        b"#!/bin/sh\necho example-airlock 1.0.0\n",
    )]));

    let mut buffer = std::io::Cursor::new(Vec::new());
    {
        let mut zip = zip::ZipWriter::new(&mut buffer);
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Stored);
        zip.start_file("metadata.json", options).expect("metadata");
        zip.write_all(br#"{"conda_pkg_format_version": 2}"#)
            .expect("metadata bytes");
        zip.start_file("info-example.tar.zst", options)
            .expect("info");
        zip.write_all(&info).expect("info bytes");
        zip.start_file("pkg-example.tar.zst", options).expect("pkg");
        zip.write_all(&payload).expect("pkg bytes");
        zip.finish().expect("finish archive");
    }
    buffer.into_inner()
}

/// A tar in memory, with fixed ownership and timestamps so the archive is reproducible.
fn tar(files: &[(&str, &[u8])]) -> Vec<u8> {
    let mut builder = tar::Builder::new(Vec::new());
    for (name, contents) in files {
        let mut header = tar::Header::new_gnu();
        header.set_size(contents.len() as u64);
        // The executable bit is what a client links and a user runs.
        header.set_mode(0o755);
        header.set_mtime(0);
        header.set_cksum();
        builder
            .append_data(&mut header, name, *contents)
            .expect("append entry");
    }
    builder.into_inner().expect("finish tar")
}

/// Run `universe publish` as the rendered job would.
fn publish(root: &Path, remote: &Path) -> CommandOutput {
    universe_kit::run([
        "publish",
        "--store",
        "channel-branch",
        "--remote",
        &remote.display().to_string(),
        "--root",
        &root.display().to_string(),
    ])
}

/// `git clone` the branch, which is how a consumer with nothing but Git obtains the channel.
fn clone_branch(remote: &Path, into: &Path) {
    // A `file://` URL rather than a bare path: a path is a local optimisation that ignores
    // `--depth`, and the point is to clone the way a consumer's remote does.
    let url = format!("file://{}", remote.display());
    let status = Command::new("git")
        .args([
            "clone",
            "--quiet",
            "--branch",
            "channel",
            "--depth",
            "1",
            &url,
            &into.display().to_string(),
        ])
        .status()
        .expect("git is installed");
    assert!(status.success(), "the branch clones");
}

/// The `pixi` this repository's own toolchain installed.
///
/// `PATH` first, because that is how the gates run; then the restored tool beside the
/// checkout, because a test can be run from a shell that has not been re-activated.
fn pixi() -> PathBuf {
    let on_path = Command::new("pixi")
        .arg("--version")
        .output()
        .is_ok_and(|output| output.status.success());
    if on_path {
        return PathBuf::from("pixi");
    }
    let restored = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.pixi/tools/linux-64/pixi");
    assert!(
        restored.exists(),
        "no pixi on PATH and none restored at {}: the solve below cannot be skipped, because \
         a channel nobody resolved is a channel nobody knows works",
        restored.display()
    );
    restored
}

#[test]
fn the_published_branch_holds_a_channel_a_solver_can_read() {
    let fixture = Fixture::new();
    fixture.add_package();
    let remote = fixture.remote();

    let output = publish(fixture.root(), &remote);
    assert_eq!(output.exit_code(), 0, "stderr: {}", output.stderr());

    let checkout = fixture.root().join("checkout");
    clone_branch(&remote, &checkout);

    // The layout a solver asks for: one repodata per subdir, `noarch` included even though
    // this project builds nothing for it.
    for relative in [
        "linux-64/repodata.json",
        "noarch/repodata.json",
        "channeldata.json",
    ] {
        assert!(
            checkout.join(relative).exists(),
            "the clone has no {relative}, so a solve would 404"
        );
    }

    let repodata: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(checkout.join("linux-64/repodata.json")).expect("repodata"),
    )
    .expect("repodata is JSON");
    let entry = &repodata["packages.conda"][format!("{PACKAGE}-{VERSION}-0.conda")];
    assert_eq!(entry["name"], PACKAGE, "the entry names another package");
    assert_eq!(entry["subdir"], "linux-64");
    assert_eq!(entry["build"], "0");
    assert!(
        repodata["packages"]
            .as_object()
            .is_some_and(serde_json::Map::is_empty),
        "a `.conda` belongs under `packages.conda`, not with the legacy `packages`"
    );
    // The bytes are where the metadata says, under the digest it claims.
    let archive = checkout.join(format!("linux-64/{PACKAGE}-{VERSION}-0.conda"));
    let bytes = std::fs::read(&archive).expect("the archive travels with the metadata");
    assert_eq!(
        entry["sha256"],
        universe_workflow::sha256::sha256_hex(&bytes).as_str()
    );
    assert_eq!(entry["size"], bytes.len());

    // The receipt travels with the artifact, so metadata is one kilobyte rather than a
    // download — and it is the auditor's own text, not a re-serialization of it.
    let metadata = checkout.join(format!("meta/linux-64/{PACKAGE}-{VERSION}-0.conda.json"));
    let receipt = std::fs::read_to_string(&metadata)
        .expect("the fragment is published beside the artifact it describes");
    let receipt: serde_json::Value = serde_json::from_str(&receipt).expect("the receipt is JSON");
    assert_eq!(receipt["digest"], entry["sha256"]);
    assert_eq!(receipt["size_bytes"], entry["size"]);
    assert_eq!(receipt["package"], PACKAGE);

    // The branch also has to say how to use it: a channel whose instructions a consumer has
    // to re-derive is one a developer gets wrong.
    let readme = std::fs::read_to_string(checkout.join("README.md")).expect("the README");
    assert!(readme.contains("channels = ["), "{readme}");
    assert!(
        readme.contains(PACKAGE),
        "the README omits the package: {readme}"
    );
}

/// Whether `path` runs, and what it printed.
fn runs(path: &Path) -> Option<String> {
    let output = Command::new(path).output().ok()?;
    output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).trim().to_owned())
}

#[test]
fn a_fresh_pixi_environment_resolves_the_package_from_the_cloned_branch() {
    let fixture = Fixture::new();
    fixture.add_package();
    let remote = fixture.remote();
    let output = publish(fixture.root(), &remote);
    assert_eq!(output.exit_code(), 0, "stderr: {}", output.stderr());

    let checkout = fixture.root().join("checkout");
    clone_branch(&remote, &checkout);

    // A developer's environment: nothing but a channel and a dependency.
    let project = fixture.root().join("consumer");
    std::fs::create_dir_all(&project).expect("consumer directory");
    std::fs::write(
        project.join("pixi.toml"),
        format!(
            r#"[project]
name = "channel-consumer"
channels = ["{}"]
platforms = ["linux-64"]

[dependencies]
{PACKAGE} = "*"
"#,
            checkout.display()
        ),
    )
    .expect("pixi.toml");

    let solved = Command::new(pixi())
        .args(["install", "--offline"])
        .current_dir(&project)
        .output()
        .expect("run pixi install");
    assert!(
        solved.status.success(),
        "pixi could not resolve the published channel\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&solved.stdout),
        String::from_utf8_lossy(&solved.stderr)
    );

    let installed = project.join(".pixi/envs/default").join(BINARY);
    assert!(
        installed.exists(),
        "the solve succeeded but {BINARY} is not in the environment"
    );
    // Installed and runnable, which is what the "develop" beat claims: the archive's mode
    // and contents survived the round trip through a git branch and a client's extraction.
    assert_eq!(
        runs(&installed).as_deref(),
        Some("example-airlock 1.0.0"),
        "the installed entrypoint does not run"
    );
}
