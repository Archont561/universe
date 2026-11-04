//! `publish-artifacts`: sending built files to a store, and doing it twice safely.
//!
//! The destination here is a directory, because that is the one store a test can inspect
//! without a network — the registries are covered by the `wiremock` suites in
//! `universe-channel`. What is asserted is the contract a job depends on: the second run of an
//! unchanged build writes nothing, a changed file under an occupied name is refused unless
//! `--force` is given, and `--dry-run` writes nothing at all.

use std::path::{Path, PathBuf};

use serde_json::Value;
use tempfile::TempDir;
use universe_kit::run;

/// A project whose writable store is a directory beside the project.
fn manifest(published: &Path) -> String {
    format!(
        r#"
schema = 1

[project]
name = "fixture-artifacts"
platforms = ["linux-64"]

[[packages]]
name = "example-airlock"
kind = "conda"
command = ["cargo", "build"]
outputs = ["output/**/*.conda"]
smoke = ["example-airlock --version"]
publish = ["local-store"]

[[stores]]
name = "local-store"
kind = "filesystem"
url = "{published}"
writable = true
auth = "token"
"#,
        published = published.display()
    )
}

struct Fixture {
    directory: TempDir,
    published: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let directory = tempfile::tempdir().expect("temp dir");
        let published = directory.path().join("published");
        std::fs::write(directory.path().join("universe.toml"), manifest(&published))
            .expect("manifest");
        let fixture = Self {
            directory,
            published,
        };
        fixture.add_package("example-airlock", b"conda bytes");
        fixture
    }

    fn root(&self) -> &Path {
        self.directory.path()
    }

    /// Write a package under `output/` and the fragment that describes it.
    fn add_package(&self, package: &str, bytes: &[u8]) {
        let relative = format!("output/{package}-1.0.0-0.conda");
        let path = self.root().join(&relative);
        std::fs::create_dir_all(path.parent().expect("parent")).expect("output dir");
        std::fs::write(&path, bytes).expect("artifact");
        let digest = universe_workflow::sha256::sha256_hex(bytes);
        let fragments = self.root().join("output/fragments");
        std::fs::create_dir_all(&fragments).expect("fragment dir");
        std::fs::write(
            fragments.join(format!("{package}-linux-64.json")),
            format!(
                r#"{{
  "package": "{package}",
  "version": "1.0.0",
  "build": "0",
  "build_number": 0,
  "subdir": "linux-64",
  "digest": "{digest}",
  "size_bytes": {},
  "depends": [],
  "license": "MIT",
  "license_files": [],
  "path": "{relative}"
}}
"#,
                bytes.len()
            ),
        )
        .expect("fragment");
    }

    fn output_of(&self, args: &[&str]) -> universe_core::CommandOutput {
        let mut invocation: Vec<String> =
            args.iter().map(|argument| (*argument).to_owned()).collect();
        invocation.extend_from_slice(&[
            "--root".to_owned(),
            self.root().to_str().expect("utf-8 root").to_owned(),
        ]);
        run(invocation)
    }

    fn artifact(&self) -> PathBuf {
        self.published
            .join("linux-64/example-airlock-1.0.0-0.conda")
    }
}

/// The `result` document of a `--json` run.
fn result(output: &universe_core::CommandOutput) -> Value {
    let envelope: Value = serde_json::from_str(output.stdout().trim()).expect("a JSON envelope");
    envelope["result"].clone()
}

#[test]
fn publish_artifacts_writes_each_artifact_under_its_subdir() {
    let fixture = Fixture::new();

    let output = fixture.output_of(&["publish-artifacts", "--store", "local-store"]);

    assert_eq!(output.exit_code(), 0, "stderr: {}", output.stderr());
    assert_eq!(
        std::fs::read(fixture.artifact()).expect("the published artifact"),
        b"conda bytes"
    );
    assert!(
        output.stdout().contains("uploaded 1 artifact"),
        "{}",
        output.stdout()
    );
}

#[test]
fn a_second_run_of_an_unchanged_build_uploads_nothing() {
    let fixture = Fixture::new();
    fixture
        .output_of(&["publish-artifacts", "--store", "local-store"])
        .expect_success();

    let output = fixture.output_of(&["publish-artifacts", "--store", "local-store", "--json"]);

    assert_eq!(output.exit_code(), 0, "stderr: {}", output.stderr());
    let stores = result(&output)["stores"].clone();
    assert_eq!(stores[0]["uploaded"], serde_json::json!([]), "{stores}");
    assert_eq!(
        stores[0]["unchanged"],
        serde_json::json!(["linux-64/example-airlock-1.0.0-0.conda"]),
        "{stores}"
    );
    assert_eq!(stores[0]["bytes"], serde_json::json!(0), "{stores}");
}

#[test]
fn a_changed_build_under_an_occupied_name_is_refused() {
    let fixture = Fixture::new();
    fixture
        .output_of(&["publish-artifacts", "--store", "local-store"])
        .expect_success();
    // A different build of the same name, as a half-finished release would leave behind.
    std::fs::write(fixture.artifact(), b"different bytes").expect("a changed artifact");

    let output = fixture.output_of(&["publish-artifacts", "--store", "local-store"]);

    assert_eq!(output.exit_code(), 1);
    assert!(
        output.stderr().contains("publish.store.refused"),
        "{}",
        output.stderr()
    );
    assert_eq!(
        std::fs::read(fixture.artifact()).expect("the artifact is left alone"),
        b"different bytes"
    );
}

#[test]
fn force_replaces_a_changed_build() {
    let fixture = Fixture::new();
    fixture
        .output_of(&["publish-artifacts", "--store", "local-store"])
        .expect_success();
    std::fs::write(fixture.artifact(), b"different bytes").expect("a changed artifact");

    let output = fixture.output_of(&["publish-artifacts", "--store", "local-store", "--force"]);

    assert_eq!(output.exit_code(), 0, "stderr: {}", output.stderr());
    assert_eq!(
        std::fs::read(fixture.artifact()).expect("the replaced artifact"),
        b"conda bytes"
    );
}

#[test]
fn a_dry_run_plans_the_upload_and_writes_nothing() {
    let fixture = Fixture::new();

    let output = fixture.output_of(&["publish-artifacts", "--store", "local-store", "--dry-run"]);

    assert_eq!(output.exit_code(), 0, "stderr: {}", output.stderr());
    assert!(
        output.stdout().contains("would upload 1 artifact"),
        "{}",
        output.stdout()
    );
    assert!(
        !fixture.published.exists(),
        "a dry run creates no destination directory"
    );
}

#[test]
fn naming_a_branch_store_is_a_usage_error() {
    let fixture = Fixture::new();
    let manifest = std::fs::read_to_string(fixture.root().join("universe.toml")).expect("manifest")
        + "\n[[stores]]\nname = \"channel-branch\"\nkind = \"branch\"\nwritable = true\n";
    std::fs::write(fixture.root().join("universe.toml"), manifest).expect("manifest");

    let output = fixture.output_of(&["publish-artifacts", "--store", "channel-branch"]);

    assert_eq!(output.exit_code(), 2);
    assert!(
        output.stderr().contains("publish.store.kind"),
        "{}",
        output.stderr()
    );
}

#[test]
fn every_artifact_of_every_package_is_uploaded_by_default() {
    let fixture = Fixture::new();
    fixture.add_package("example-tools", b"second conda bytes");

    let output = fixture.output_of(&["publish-artifacts", "--json"]);

    assert_eq!(output.exit_code(), 0, "stderr: {}", output.stderr());
    let stores = result(&output)["stores"].clone();
    assert_eq!(
        stores[0]["uploaded"].as_array().map(Vec::len),
        Some(2),
        "{stores}"
    );
}

/// Fail the test with stderr when the run was expected to succeed.
trait ExpectSuccess {
    fn expect_success(self);
}

impl ExpectSuccess for universe_core::CommandOutput {
    fn expect_success(self) {
        assert_eq!(self.exit_code(), 0, "stderr: {}", self.stderr());
    }
}
