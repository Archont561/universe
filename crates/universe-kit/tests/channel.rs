//! `index` and `publish`: the two ways a rendered channel leaves the machine.
//!
//! Every test builds its own project and its own Git remote in a temporary directory. Nothing
//! here reads this repository's manifest or touches its branches — a test that publishes into
//! the repository it runs in is a test that can corrupt a developer's work, so the remote is
//! always a bare repository created by the test itself.

use std::path::{Path, PathBuf};
use std::process::Command;

use tempfile::TempDir;
use universe_kit::run;

const MANIFEST: &str = r#"
schema = 1

[project]
name = "fixture-channel"
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
url = "https://example.invalid/universe.git"
writable = true
"#;

/// A fragment, with the digest and size filled in by the fixture.
fn fragment_json(package: &str, subdir: &str, digest: &str, size: u64, path: &str) -> String {
    format!(
        r#"{{
  "package": "{package}",
  "version": "1.0.0",
  "build": "0",
  "build_number": 0,
  "subdir": "{subdir}",
  "digest": "{digest}",
  "size_bytes": {size},
  "depends": [],
  "license": "MIT",
  "license_files": [],
  "path": "{path}"
}}
"#
    )
}

/// A project with one package and one fragment, ready to index.
struct Fixture {
    directory: TempDir,
}

impl Fixture {
    fn new() -> Self {
        Self::with_manifest(MANIFEST)
    }

    fn with_manifest(manifest: &str) -> Self {
        let directory = tempfile::tempdir().expect("temp dir");
        std::fs::write(directory.path().join("universe.toml"), manifest).expect("manifest");
        let fixture = Self { directory };
        fixture.add_package("example-airlock", "linux-64", b"conda bytes");
        fixture
    }

    fn root(&self) -> &Path {
        self.directory.path()
    }

    /// Write a package under `output/` and the fragment that describes it.
    fn add_package(&self, package: &str, subdir: &str, bytes: &[u8]) {
        let relative = format!("output/{package}-1.0.0-0.conda");
        let path = self.root().join(&relative);
        std::fs::create_dir_all(path.parent().expect("parent")).expect("output dir");
        std::fs::write(&path, bytes).expect("artifact");
        let digest = universe_workflow::sha256::sha256_hex(bytes);
        let fragments = self.root().join("output/fragments");
        std::fs::create_dir_all(&fragments).expect("fragment dir");
        std::fs::write(
            fragments.join(format!("{package}-{subdir}.json")),
            fragment_json(package, subdir, &digest, bytes.len() as u64, &relative),
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

    fn index(&self) -> universe_core::CommandOutput {
        let output = self.root().join("channel");
        self.output_of(&["index", "--output", output.to_str().expect("utf-8")])
    }

    fn read(&self, relative: &str) -> String {
        std::fs::read_to_string(self.root().join("channel").join(relative)).expect("rendered file")
    }

    fn exists(&self, relative: &str) -> bool {
        self.root().join("channel").join(relative).exists()
    }
}

/// A bare repository the test owns, standing in for the channel's remote.
fn remote(directory: &Path) -> PathBuf {
    let path = directory.join("remote.git");
    let status = Command::new("git")
        .args(["init", "--bare", "--quiet"])
        .arg(&path)
        .status()
        .expect("git is installed");
    assert!(status.success(), "bare repository initialized");
    path
}

/// Whether the remote holds the named branch at all.
fn branch_exists(remote: &Path, branch: &str) -> bool {
    Command::new("git")
        .args([
            "-C",
            remote.to_str().expect("utf-8"),
            "rev-parse",
            "--quiet",
            "--verify",
        ])
        .arg(format!("refs/heads/{branch}"))
        .output()
        .expect("git runs")
        .status
        .success()
}

/// Every file under `directory`, relative and content-hashed, to prove a verb wrote nothing.
fn snapshot(directory: &Path) -> Vec<(String, Vec<u8>)> {
    let mut found = Vec::new();
    fn walk(root: &Path, directory: &Path, found: &mut Vec<(String, Vec<u8>)>) {
        let Ok(entries) = std::fs::read_dir(directory) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                walk(root, &path, found);
            } else {
                found.push((
                    path.strip_prefix(root)
                        .expect("inside root")
                        .display()
                        .to_string(),
                    std::fs::read(&path).expect("readable"),
                ));
            }
        }
    }
    walk(directory, directory, &mut found);
    found.sort();
    found
}

#[test]
fn index_renders_a_channel_a_solver_can_read() {
    let fixture = Fixture::new();
    let output = fixture.index();
    assert_eq!(output.exit_code(), 0, "stderr: {}", output.stderr());

    // Every configured subdir, plus `noarch`, gets metadata even when it holds nothing.
    assert!(fixture.exists("linux-64/repodata.json"));
    assert!(fixture.exists("noarch/repodata.json"));
    assert!(fixture.exists("channeldata.json"));
    assert!(fixture.exists("run_exports.json"));
    assert!(fixture.exists(".nojekyll"));
    assert!(fixture.exists("README.md"));
    // The artifact itself, at the path its own metadata advertises.
    assert!(fixture.exists("linux-64/example-airlock-1.0.0-0.conda"));

    let repodata = fixture.read("linux-64/repodata.json");
    assert!(repodata.contains("packages.conda"), "{repodata}");
    assert!(
        repodata.contains("example-airlock-1.0.0-0.conda"),
        "{repodata}"
    );
    // Two fields a channel must not invent: a timestamp would make every render differ, and
    // an md5 would be a digest nobody measured.
    assert!(!repodata.contains("timestamp"), "{repodata}");
    assert!(!repodata.contains("md5"), "{repodata}");

    let empty = fixture.read("noarch/repodata.json");
    assert!(empty.contains("\"packages.conda\": {}"), "{empty}");
}

#[test]
fn rendering_the_same_fragments_twice_produces_identical_bytes() {
    let fixture = Fixture::new();
    assert_eq!(fixture.index().exit_code(), 0);
    let first = snapshot(&fixture.root().join("channel"));

    assert_eq!(fixture.index().exit_code(), 0);
    let second = snapshot(&fixture.root().join("channel"));

    assert_eq!(
        first, second,
        "a render must be a pure function of the fragments"
    );
}

#[test]
fn a_digest_that_disagrees_with_the_bytes_stops_the_index_and_names_the_file() {
    let fixture = Fixture::new();
    let fragment = fixture
        .root()
        .join("output/fragments/example-airlock-linux-64.json");
    let text = std::fs::read_to_string(&fragment).expect("fragment");
    let digest = universe_workflow::sha256::sha256_hex(b"conda bytes");
    std::fs::write(
        &fragment,
        text.replace(
            &digest,
            &universe_workflow::sha256::sha256_hex(b"other bytes"),
        ),
    )
    .expect("rewritten fragment");

    let output = fixture.index();
    assert_eq!(output.exit_code(), 1);
    assert!(
        output.stderr().contains("example-airlock-1.0.0-0.conda"),
        "the failure must name the artifact: {}",
        output.stderr()
    );
}

#[test]
fn a_missing_artifact_is_reported_by_path() {
    let fixture = Fixture::new();
    std::fs::remove_file(fixture.root().join("output/example-airlock-1.0.0-0.conda"))
        .expect("artifact removed");

    let output = fixture.index();
    assert_eq!(output.exit_code(), 1);
    assert!(
        output
            .stderr()
            .contains("output/example-airlock-1.0.0-0.conda")
    );
}

#[test]
fn a_dry_run_describes_the_publish_and_writes_nothing() {
    let fixture = Fixture::new();
    let remote = remote(fixture.root());
    let before = snapshot(&fixture.root().join("output"));
    let remote_before = snapshot(&remote);

    let output = fixture.output_of(&[
        "publish",
        "--dry-run",
        "--remote",
        remote.to_str().expect("utf-8"),
        "--branch",
        "channel",
    ]);

    assert_eq!(output.exit_code(), 0, "stderr: {}", output.stderr());
    assert!(
        output.stdout().contains("linux-64/repodata.json"),
        "{}",
        output.stdout()
    );
    assert_eq!(before, snapshot(&fixture.root().join("output")));
    assert_eq!(remote_before, snapshot(&remote));
    assert!(!fixture.root().join("output/publish").exists());
}

#[test]
fn publish_writes_one_orphan_snapshot_and_a_second_publish_pushes_nothing() {
    let fixture = Fixture::new();
    let remote = remote(fixture.root());
    let remote_path = remote.to_str().expect("utf-8").to_owned();

    let first = fixture.output_of(&["publish", "--remote", &remote_path, "--branch", "channel"]);
    assert_eq!(first.exit_code(), 0, "stderr: {}", first.stderr());

    // The branch is a snapshot, not a changelog: exactly one commit, and it has no parent.
    let count = Command::new("git")
        .args(["-C", &remote_path, "rev-list", "--count", "channel"])
        .output()
        .expect("git runs");
    assert_eq!(String::from_utf8_lossy(&count.stdout).trim(), "1");
    let parents = Command::new("git")
        .args([
            "-C",
            &remote_path,
            "rev-list",
            "--parents",
            "--count",
            "channel",
        ])
        .output()
        .expect("git runs");
    assert_eq!(String::from_utf8_lossy(&parents.stdout).trim(), "1");

    // The published tree is exactly what the fragments render to, which is what `--check`
    // answers, and what makes a republish a no-op rather than a second empty commit.
    let check = fixture.output_of(&[
        "publish",
        "--check",
        "--remote",
        &remote_path,
        "--branch",
        "channel",
    ]);
    assert_eq!(check.exit_code(), 0, "stderr: {}", check.stderr());

    let second = fixture.output_of(&["publish", "--remote", &remote_path, "--branch", "channel"]);
    assert_eq!(second.exit_code(), 0, "stderr: {}", second.stderr());
    assert!(
        second.stdout().contains("nothing pushed"),
        "{}",
        second.stdout()
    );
    let after = Command::new("git")
        .args(["-C", &remote_path, "rev-list", "--count", "channel"])
        .output()
        .expect("git runs");
    assert_eq!(String::from_utf8_lossy(&after.stdout).trim(), "1");
}

#[test]
fn check_fails_on_a_branch_that_was_never_published() {
    let fixture = Fixture::new();
    let remote = remote(fixture.root());

    let output = fixture.output_of(&[
        "publish",
        "--check",
        "--remote",
        remote.to_str().expect("utf-8"),
        "--branch",
        "channel",
    ]);

    assert_eq!(output.exit_code(), 1);
    assert!(
        output.stderr().contains("publish.check.absent"),
        "{}",
        output.stderr()
    );
}

#[test]
fn a_store_budget_that_the_snapshot_exceeds_refuses_before_anything_is_pushed() {
    let manifest = MANIFEST.replace(
        "writable = true",
        "writable = true\n[stores.budgets]\ntransport = 4",
    );
    let fixture = Fixture::with_manifest(&manifest);
    let remote = remote(fixture.root());
    let remote_before = snapshot(&remote);

    let output = fixture.output_of(&[
        "publish",
        "--remote",
        remote.to_str().expect("utf-8"),
        "--branch",
        "channel",
        "--store",
        "channel-branch",
    ]);

    assert_eq!(output.exit_code(), 1);
    assert!(
        output.stderr().contains("transport ceiling"),
        "{}",
        output.stderr()
    );
    assert!(output.stderr().contains("publish.budget.exceeded"));
    // Nothing left the machine, and nothing was staged.
    assert_eq!(remote_before, snapshot(&remote));
    assert!(!fixture.root().join("output/publish").exists());
}

#[test]
fn a_publish_that_names_no_store_still_applies_the_store_budget() {
    // The trap this guards: the budget used to be read only when `--store` was passed, so the
    // same manifest produced a refusal for one invocation and a push for another.
    let manifest = MANIFEST.replace(
        "writable = true",
        "writable = true\n[stores.budgets]\ntransport = 4",
    );
    let fixture = Fixture::with_manifest(&manifest);
    let remote = remote(fixture.root());
    let remote_before = snapshot(&remote);

    let output = fixture.output_of(&[
        "publish",
        "--remote",
        remote.to_str().expect("utf-8"),
        "--branch",
        "channel",
    ]);

    assert_eq!(output.exit_code(), 1);
    assert!(
        output.stderr().contains("publish.budget.exceeded"),
        "{}",
        output.stderr()
    );
    assert_eq!(remote_before, snapshot(&remote));
}

#[test]
fn a_publish_that_names_no_store_is_refused_when_two_branch_stores_are_writable() {
    let manifest = format!(
        "{MANIFEST}\n[[stores]]\nname = \"mirror-branch\"\nkind = \"branch\"\nwritable = true\n"
    );
    let fixture = Fixture::with_manifest(&manifest);
    let remote = remote(fixture.root());

    let output = fixture.output_of(&[
        "publish",
        "--remote",
        remote.to_str().expect("utf-8"),
        "--branch",
        "channel",
    ]);

    assert_eq!(output.exit_code(), 2);
    assert!(
        output.stderr().contains("publish.store.ambiguous"),
        "{}",
        output.stderr()
    );
    assert!(
        output.stderr().contains("mirror-branch"),
        "the refusal names both candidates: {}",
        output.stderr()
    );
}

#[test]
fn a_publish_measured_against_a_registry_store_is_refused() {
    let manifest = format!(
        "{MANIFEST}\n[[stores]]\nname = \"hosted\"\nkind = \"prefix-dev\"\nchannel = \"acme/acme\"\nwritable = true\n"
    );
    let fixture = Fixture::with_manifest(&manifest);
    let remote = remote(fixture.root());

    let output = fixture.output_of(&[
        "publish",
        "--remote",
        remote.to_str().expect("utf-8"),
        "--branch",
        "channel",
        "--store",
        "hosted",
    ]);

    assert_eq!(output.exit_code(), 2);
    assert!(
        output.stderr().contains("publish.store.kind"),
        "{}",
        output.stderr()
    );
}

#[test]
fn a_package_filter_narrows_the_channel_to_what_was_named() {
    let fixture = Fixture::new();
    fixture.add_package("other-airlock", "linux-64", b"other conda bytes");
    let output = fixture.root().join("channel");

    let result = fixture.output_of(&[
        "index",
        "--output",
        output.to_str().expect("utf-8"),
        "--package",
        "other-airlock",
    ]);

    assert_eq!(result.exit_code(), 0, "stderr: {}", result.stderr());
    let repodata = fixture.read("linux-64/repodata.json");
    assert!(repodata.contains("other-airlock"), "{repodata}");
    assert!(!repodata.contains("example-airlock"), "{repodata}");
}

/// The blob ids of every path in a branch's tree, as git records them.
fn tree_blobs(remote: &Path, revision: &str) -> Vec<(String, String)> {
    let output = Command::new("git")
        .args([
            "-C",
            remote.to_str().expect("utf-8"),
            "ls-tree",
            "-r",
            revision,
        ])
        .output()
        .expect("git runs");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(|line| {
            let (meta, path) = line.split_once('\t')?;
            let mut fields = meta.split_whitespace();
            Some((fields.nth(2)?.to_owned(), path.to_owned()))
        })
        .collect()
}

#[test]
fn a_consumer_restores_the_channel_with_git_alone() {
    let fixture = Fixture::new();
    let remote = remote(fixture.root());
    let remote_path = remote.to_str().expect("utf-8").to_owned();
    assert_eq!(
        fixture
            .output_of(&["publish", "--remote", &remote_path, "--branch", "channel"])
            .exit_code(),
        0
    );

    // No Universe on the other side: a clone of the branch, and the tree is complete.
    let checkout = tempfile::tempdir().expect("temp dir");
    let clone = Command::new("git")
        .args(["clone", "--quiet", "--depth", "1", "--branch", "channel"])
        .arg(&remote)
        .arg(checkout.path())
        .output()
        .expect("git runs");
    assert!(
        clone.status.success(),
        "{}",
        String::from_utf8_lossy(&clone.stderr)
    );

    let restored = checkout.path();
    for file in [
        "README.md",
        "channeldata.json",
        "linux-64/repodata.json",
        "noarch/repodata.json",
        "linux-64/example-airlock-1.0.0-0.conda",
    ] {
        assert!(
            restored.join(file).is_file(),
            "{file} is missing from the clone"
        );
    }
    // What was cloned hashes to what the fragment claimed, which is the whole point of
    // publishing a digest beside the bytes.
    let bytes = std::fs::read(restored.join("linux-64/example-airlock-1.0.0-0.conda"))
        .expect("published artifact");
    assert_eq!(bytes, b"conda bytes");
    let readme = std::fs::read_to_string(restored.join("README.md")).expect("README");
    assert!(readme.contains("channels"), "{readme}");
}

#[test]
fn changing_one_artifact_reuses_every_blob_that_did_not_change() {
    let fixture = Fixture::new();
    let remote = remote(fixture.root());
    let remote_path = remote.to_str().expect("utf-8").to_owned();
    assert_eq!(
        fixture
            .output_of(&["publish", "--remote", &remote_path, "--branch", "channel"])
            .exit_code(),
        0
    );
    let before = tree_blobs(&remote, "channel");

    // A second package, so the first publish's other files must survive unchanged.
    fixture.add_package("other-airlock", "linux-64", b"other conda bytes");
    let published =
        fixture.output_of(&["publish", "--remote", &remote_path, "--branch", "channel"]);
    assert_eq!(published.exit_code(), 0, "stderr: {}", published.stderr());
    let after = tree_blobs(&remote, "channel");

    // Whole files are what makes this true; fixed-size chunking would rewrite every piece on
    // every publish, because the boundaries would not follow the artifact.
    let shared: Vec<&String> = after
        .iter()
        .filter(|(blob, _)| before.iter().any(|(earlier, _)| earlier == blob))
        .map(|(blob, _)| blob)
        .collect();
    assert!(
        shared.len() >= 3,
        "expected the unchanged metadata and artifact to be reused: {after:?}"
    );
    assert!(
        after
            .iter()
            .any(|(_, path)| path == "linux-64/example-airlock-1.0.0-0.conda"),
        "{after:?}"
    );
}

#[test]
fn a_file_above_the_blob_ceiling_is_published_as_numbered_parts() {
    let manifest = MANIFEST.replace(
        "writable = true",
        "writable = true\n[stores.budgets]\nblob = 2000",
    );
    let fixture = Fixture::with_manifest(&manifest);
    // Larger than the ceiling the store declares, so the split is exercised at a size a
    // test can afford; the ceiling that matters in production is the 95 MiB constant.
    fixture.add_package("example-airlock", "linux-64", &[b'x'; 4_000]);
    let remote = remote(fixture.root());

    let output = fixture.output_of(&[
        "publish",
        "--dry-run",
        "--remote",
        remote.to_str().expect("utf-8"),
        "--branch",
        "channel",
        "--store",
        "channel-branch",
    ]);

    assert_eq!(output.exit_code(), 0, "stderr: {}", output.stderr());
    assert!(output.stdout().contains(".part001"), "{}", output.stdout());
    assert!(
        !output
            .stdout()
            .contains("linux-64/example-airlock-1.0.0-0.conda\n"),
        "the whole file must not be published beside its parts: {}",
        output.stdout()
    );
}

#[test]
fn index_without_an_output_directory_is_a_usage_error() {
    let fixture = Fixture::new();
    let output = fixture.output_of(&["index"]);
    assert_eq!(output.exit_code(), 2);
    assert!(
        output.stderr().contains("index.output.required"),
        "{}",
        output.stderr()
    );
}

#[test]
fn index_reports_the_rendered_file_set_as_json() {
    let fixture = Fixture::new();
    let directory = fixture.root().join("channel");
    let output = fixture.output_of(&[
        "index",
        "--json",
        "--output",
        directory.to_str().expect("utf-8"),
    ]);

    assert_eq!(output.exit_code(), 0, "stderr: {}", output.stderr());
    let envelope: serde_json::Value =
        serde_json::from_str(output.stdout()).expect("one JSON document on stdout");
    assert_eq!(envelope["command"], "index");
    assert_eq!(envelope["result"]["packages"], 1);
    let files = envelope["result"]["files"].as_array().expect("files");
    assert!(files.iter().any(|file| file == "linux-64/repodata.json"));
}

#[test]
fn publish_reports_the_pushed_snapshot_as_json_and_nothing_on_a_republish() {
    let fixture = Fixture::new();
    let remote = remote(fixture.root());
    let remote_path = remote.to_str().expect("utf-8").to_owned();

    let published = fixture.output_of(&[
        "publish",
        "--json",
        "--remote",
        &remote_path,
        "--branch",
        "channel",
    ]);
    assert_eq!(published.exit_code(), 0, "stderr: {}", published.stderr());
    let envelope: serde_json::Value =
        serde_json::from_str(published.stdout()).expect("one JSON document on stdout");
    assert_eq!(envelope["result"]["published"], true);
    assert!(
        envelope["result"]["commit"]
            .as_str()
            .is_some_and(|commit| !commit.is_empty())
    );

    let again = fixture.output_of(&[
        "publish",
        "--json",
        "--remote",
        &remote_path,
        "--branch",
        "channel",
    ]);
    assert_eq!(again.exit_code(), 0, "stderr: {}", again.stderr());
    let envelope: serde_json::Value =
        serde_json::from_str(again.stdout()).expect("one JSON document on stdout");
    assert_eq!(envelope["result"]["published"], false);
    assert_eq!(envelope["result"]["reason"], "unchanged");

    let check = fixture.output_of(&[
        "publish",
        "--json",
        "--check",
        "--remote",
        &remote_path,
        "--branch",
        "channel",
    ]);
    assert_eq!(check.exit_code(), 0, "stderr: {}", check.stderr());
    let envelope: serde_json::Value =
        serde_json::from_str(check.stdout()).expect("one JSON document on stdout");
    assert_eq!(envelope["result"]["check"], "clean");
}

#[test]
fn a_remote_that_does_not_exist_fails_the_publish_without_a_claim_that_it_worked() {
    let fixture = Fixture::new();
    let missing = fixture.root().join("nowhere.git");
    let output = fixture.output_of(&[
        "publish",
        "--remote",
        missing.to_str().expect("utf-8"),
        "--branch",
        "channel",
    ]);

    assert_eq!(output.exit_code(), 1);
    assert!(
        output.stderr().contains("publish.transport.failed"),
        "{}",
        output.stderr()
    );
    assert!(
        output.stdout().is_empty(),
        "stdout stays data-only: {}",
        output.stdout()
    );
}

#[test]
fn an_unknown_store_is_a_usage_error_because_no_budget_could_apply() {
    let fixture = Fixture::new();
    let remote = remote(fixture.root());
    let output = fixture.output_of(&[
        "publish",
        "--remote",
        remote.to_str().expect("utf-8"),
        "--store",
        "ghost",
    ]);

    assert_eq!(output.exit_code(), 2);
    assert!(
        output.stderr().contains("publish.store.unknown"),
        "{}",
        output.stderr()
    );
}

#[test]
fn a_publish_without_a_remote_says_what_is_missing() {
    let fixture = Fixture::new();
    let output = fixture.output_of(&["publish"]);
    assert_eq!(output.exit_code(), 2);
    assert!(output.stderr().contains("--remote"), "{}", output.stderr());
}

#[test]
fn the_human_forms_of_publish_say_what_happened() {
    let fixture = Fixture::new();
    let remote = remote(fixture.root());
    let remote_path = remote.to_str().expect("utf-8").to_owned();

    let published =
        fixture.output_of(&["publish", "--remote", &remote_path, "--branch", "channel"]);
    assert!(
        published.stdout().contains("published"),
        "{}",
        published.stdout()
    );
    let checked = fixture.output_of(&[
        "publish",
        "--check",
        "--remote",
        &remote_path,
        "--branch",
        "channel",
    ]);
    assert!(
        checked.stdout().contains("already holds"),
        "{}",
        checked.stdout()
    );
}

#[test]
fn check_reports_drift_after_an_artifact_changes() {
    let fixture = Fixture::new();
    let remote = remote(fixture.root());
    let remote_path = remote.to_str().expect("utf-8").to_owned();
    assert_eq!(
        fixture
            .output_of(&["publish", "--remote", &remote_path, "--branch", "channel"])
            .exit_code(),
        0
    );

    // The bytes moved, so the branch no longer describes what would be published today.
    fixture.add_package("example-airlock", "linux-64", b"rebuilt bytes");
    let output = fixture.output_of(&[
        "publish",
        "--check",
        "--remote",
        &remote_path,
        "--branch",
        "channel",
    ]);

    assert_eq!(output.exit_code(), 1);
    assert!(
        output.stderr().contains("publish.check.drift"),
        "{}",
        output.stderr()
    );
    assert!(
        output.stderr().contains("universe publish"),
        "{}",
        output.stderr()
    );
}

#[test]
fn a_staged_ceiling_that_the_snapshot_exceeds_is_reported_with_its_own_field() {
    let manifest = MANIFEST.replace(
        "writable = true",
        "writable = true\n[stores.budgets]\nstaged = 4",
    );
    let fixture = Fixture::with_manifest(&manifest);
    let remote = remote(fixture.root());

    let output = fixture.output_of(&[
        "publish",
        "--dry-run",
        "--remote",
        remote.to_str().expect("utf-8"),
        "--store",
        "channel-branch",
    ]);

    assert_eq!(output.exit_code(), 1);
    assert!(
        output.stderr().contains("staged push"),
        "{}",
        output.stderr()
    );
    assert!(
        output.stderr().contains("[stores.budgets].staged"),
        "{}",
        output.stderr()
    );
}

#[test]
fn an_output_path_that_is_a_file_fails_the_index_instead_of_dropping_files() {
    let fixture = Fixture::new();
    let output = fixture.root().join("channel");
    std::fs::write(&output, "not a directory").expect("blocking file");

    let result = fixture.output_of(&["index", "--output", output.to_str().expect("utf-8")]);

    assert_eq!(result.exit_code(), 2);
    assert!(
        result.stderr().contains("index.write.failed"),
        "{}",
        result.stderr()
    );
}

#[test]
fn a_manifest_that_does_not_validate_is_refused_before_any_fragment_is_read() {
    let manifest = MANIFEST.replace("platforms = [\"linux-64\"]", "platforms = [\" \"]");
    let fixture = Fixture::with_manifest(&manifest);
    let output = fixture.root().join("channel");

    let result = fixture.output_of(&["index", "--output", output.to_str().expect("utf-8")]);

    assert_eq!(result.exit_code(), 2);
    assert!(!output.exists());
}

#[test]
fn a_dry_run_in_json_form_carries_the_budget_it_was_measured_against() {
    let fixture = Fixture::new();
    let remote = remote(fixture.root());
    let output = fixture.output_of(&[
        "publish",
        "--dry-run",
        "--json",
        "--remote",
        remote.to_str().expect("utf-8"),
        "--branch",
        "channel",
    ]);

    assert_eq!(output.exit_code(), 0, "stderr: {}", output.stderr());
    let envelope: serde_json::Value =
        serde_json::from_str(output.stdout()).expect("one JSON document on stdout");
    assert_eq!(envelope["result"]["dry_run"], true);
    assert_eq!(envelope["result"]["budgets"]["blob"], 99_614_720);
}

#[test]
fn every_channel_verb_refuses_an_argument_it_does_not_know() {
    let fixture = Fixture::new();
    for verb in ["index", "publish"] {
        let output = fixture.output_of(&[verb, "--nonsense"]);
        assert_eq!(output.exit_code(), 2, "{verb}");
        assert!(
            output.stderr().contains("unknown argument"),
            "{verb}: {}",
            output.stderr()
        );
    }
}

#[test]
fn a_verb_run_outside_a_project_says_which_manifest_is_missing() {
    let directory = tempfile::tempdir().expect("temp dir");
    let root = directory.path().to_str().expect("utf-8").to_owned();
    let output = run(vec!["index".to_owned(), "--root".to_owned(), root]);

    assert_eq!(output.exit_code(), 2);
    assert!(
        output.stderr().contains("universe.toml"),
        "{}",
        output.stderr()
    );
}

#[test]
fn publish_refuses_a_manifest_that_does_not_validate() {
    let manifest = MANIFEST.replace("platforms = [\"linux-64\"]", "platforms = [\" \"]");
    let fixture = Fixture::with_manifest(&manifest);
    let remote = remote(fixture.root());

    let output = fixture.output_of(&["publish", "--remote", remote.to_str().expect("utf-8")]);

    assert_eq!(output.exit_code(), 2);
    assert!(
        !branch_exists(&remote, "channel"),
        "an invalid manifest publishes nothing"
    );
}

#[test]
fn publish_refuses_a_fragment_that_could_not_describe_a_package() {
    let fixture = Fixture::new();
    let remote = remote(fixture.root());
    let fragment = fixture
        .root()
        .join("output/fragments/example-airlock-linux-64.json");
    let text = std::fs::read_to_string(&fragment).expect("fragment");
    std::fs::write(
        &fragment,
        text.replace("\"size_bytes\": 11", "\"size_bytes\": 0"),
    )
    .expect("rewritten fragment");

    let output = fixture.output_of(&["publish", "--remote", remote.to_str().expect("utf-8")]);

    assert_eq!(output.exit_code(), 1);
    assert!(
        output.stderr().contains("channel.fragment.unusable"),
        "{}",
        output.stderr()
    );
}

#[test]
fn a_digest_that_disagrees_stops_a_publish_as_well_as_an_index() {
    let fixture = Fixture::new();
    let remote = remote(fixture.root());
    std::fs::write(
        fixture.root().join("output/example-airlock-1.0.0-0.conda"),
        b"tampered bytes",
    )
    .expect("tampered artifact");

    let output = fixture.output_of(&["publish", "--remote", remote.to_str().expect("utf-8")]);

    assert_eq!(output.exit_code(), 1);
    assert!(
        output.stderr().contains("channel.artifact.size"),
        "{}",
        output.stderr()
    );
    assert!(!branch_exists(&remote, "channel"));
}

// ---------------------------------------------------------------------------------------
// A payload a transport can carry: no symlinks, and staging outside it.
// ---------------------------------------------------------------------------------------

#[test]
fn an_artifact_that_is_a_symlink_is_refused_before_anything_is_staged() {
    let fixture = Fixture::new();
    let remote = remote(fixture.root());
    let artifact = fixture.root().join("output/example-airlock-1.0.0-0.conda");
    // The link points at real bytes, so only a refusal that looks at the *entry* catches it:
    // a read would follow the link and hash a file from outside the payload.
    let elsewhere = fixture.root().join("elsewhere.conda");
    std::fs::write(&elsewhere, b"conda bytes").expect("the target");
    std::fs::remove_file(&artifact).expect("the original artifact");
    std::os::unix::fs::symlink(&elsewhere, &artifact).expect("a symlink in the payload");

    let output = fixture.output_of(&[
        "publish",
        "--remote",
        remote.to_str().expect("utf-8"),
        "--branch",
        "channel",
    ]);

    assert_eq!(output.exit_code(), 1, "stderr: {}", output.stderr());
    assert!(
        output.stderr().contains("channel.payload.symlink"),
        "{}",
        output.stderr()
    );
    assert!(
        output.stderr().contains("example-airlock-1.0.0-0.conda"),
        "the refusal must name the link: {}",
        output.stderr()
    );
    assert!(!branch_exists(&remote, "channel"));
}

#[test]
fn a_fragment_reached_through_a_symlink_is_refused() {
    let fixture = Fixture::new();
    let remote = remote(fixture.root());
    let fragment = fixture
        .root()
        .join("output/fragments/example-airlock-linux-64.json");
    let target = fixture.root().join("elsewhere.json");
    std::fs::rename(&fragment, &target).expect("the fragment moved out of the way");
    std::os::unix::fs::symlink(&target, &fragment).expect("a symlinked fragment");

    let output = fixture.output_of(&[
        "publish",
        "--remote",
        remote.to_str().expect("utf-8"),
        "--branch",
        "channel",
    ]);

    assert_eq!(output.exit_code(), 1, "stderr: {}", output.stderr());
    assert!(
        output.stderr().contains("channel.payload.symlink"),
        "{}",
        output.stderr()
    );
    assert!(!branch_exists(&remote, "channel"));
}

#[test]
fn staging_inside_the_directory_that_holds_the_artifacts_is_refused() {
    let fixture = Fixture::new();
    let remote = remote(fixture.root());
    let inside = fixture.root().join("output/publish");

    let output = fixture.output_of(&[
        "publish",
        "--remote",
        remote.to_str().expect("utf-8"),
        "--branch",
        "channel",
        "--output",
        inside.to_str().expect("utf-8"),
    ]);

    assert_eq!(output.exit_code(), 2, "stderr: {}", output.stderr());
    assert!(
        output.stderr().contains("publish.scratch.inside-payload"),
        "{}",
        output.stderr()
    );
    assert!(!branch_exists(&remote, "channel"));
}

#[test]
fn a_publish_stages_outside_the_project_and_leaves_nothing_behind() {
    let fixture = Fixture::new();
    let remote = remote(fixture.root());
    let scratch = fixture.root().join("staging");

    let output = fixture.output_of(&[
        "publish",
        "--remote",
        remote.to_str().expect("utf-8"),
        "--branch",
        "channel",
        "--output",
        scratch.to_str().expect("utf-8"),
    ]);

    assert_eq!(output.exit_code(), 0, "stderr: {}", output.stderr());
    // The directory was created by this run, so it is this run's to remove.
    assert!(
        !scratch.exists(),
        "a publish that succeeded left its staging tree behind"
    );
    assert!(
        !fixture.root().join("output/publish").exists(),
        "the default scratch must not land inside the payload"
    );
}

// ---------------------------------------------------------------------------------------
// Retention: which builds a snapshot keeps, and what that claim does not mean.
// ---------------------------------------------------------------------------------------

/// A fixture holding `builds` builds of one package and version.
///
/// Every build has its own bytes, so the artifacts are genuinely distinct rather than three
/// names for one file.
fn with_builds(builds: &[(&str, u64)]) -> Fixture {
    let fixture = Fixture::new();
    // The default fixture's artifact and fragment are replaced by the build set.
    std::fs::remove_file(fixture.root().join("output/example-airlock-1.0.0-0.conda"))
        .expect("the default artifact");
    std::fs::remove_file(
        fixture
            .root()
            .join("output/fragments/example-airlock-linux-64.json"),
    )
    .expect("the default fragment");

    for (build, build_number) in builds {
        let relative = format!("output/example-airlock-1.0.0-{build}.conda");
        let bytes = format!("conda bytes for {build}").into_bytes();
        std::fs::write(fixture.root().join(&relative), &bytes).expect("artifact");
        let digest = universe_workflow::sha256::sha256_hex(&bytes);
        let fragment = format!(
            r#"{{
  "package": "example-airlock",
  "version": "1.0.0",
  "build": "{build}",
  "build_number": {build_number},
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
        );
        std::fs::write(
            fixture
                .root()
                .join(format!("output/fragments/example-airlock-{build}.json")),
            fragment,
        )
        .expect("fragment");
    }
    fixture
}

/// The file names a branch holds under `linux-64`, without their directory.
fn branch_packages(remote: &Path, branch: &str) -> Vec<String> {
    let listing = Command::new("git")
        .args([
            "-C",
            remote.to_str().expect("utf-8"),
            "ls-tree",
            "-r",
            "--name-only",
        ])
        .arg(branch)
        .output()
        .expect("git runs");
    String::from_utf8_lossy(&listing.stdout)
        .lines()
        .filter_map(|line| line.strip_prefix("linux-64/"))
        .filter(|name| name.ends_with(".conda"))
        .map(str::to_owned)
        .collect()
}

#[test]
fn keep_n_publishes_the_newest_builds_and_says_what_that_did_not_reclaim() {
    let fixture = with_builds(&[("h0000001", 1), ("h0000002", 2), ("h0000003", 3)]);
    let remote = remote(fixture.root());

    let output = fixture.output_of(&[
        "publish",
        "--remote",
        remote.to_str().expect("utf-8"),
        "--branch",
        "channel",
        "--keep",
        "2",
    ]);

    assert_eq!(output.exit_code(), 0, "stderr: {}", output.stderr());
    assert_eq!(
        branch_packages(&remote, "channel"),
        [
            "example-airlock-1.0.0-h0000002.conda",
            "example-airlock-1.0.0-h0000003.conda"
        ]
    );
    // What the flag means, in the flag's own output: a force-push bounds what is served.
    assert!(
        output.stdout().contains("bounds what it serves"),
        "the report must not imply it reclaimed storage: {}",
        output.stdout()
    );
    assert!(
        output.stdout().contains("does not delete"),
        "the report must say what it did not do: {}",
        output.stdout()
    );
    assert!(
        output
            .stdout()
            .contains("example-airlock-1.0.0-h0000001.conda"),
        "the pruned artifact must be named: {}",
        output.stdout()
    );
}

#[test]
fn the_dry_run_reports_retention_and_denies_reclaiming_storage() {
    let fixture = with_builds(&[("h0000001", 1), ("h0000002", 2)]);
    let remote = remote(fixture.root());

    let output = fixture.output_of(&[
        "publish",
        "--dry-run",
        "--json",
        "--remote",
        remote.to_str().expect("utf-8"),
        "--branch",
        "channel",
        "--keep",
        "1",
    ]);

    assert_eq!(output.exit_code(), 0, "stderr: {}", output.stderr());
    let envelope: serde_json::Value =
        serde_json::from_str(output.stdout()).expect("one JSON document on stdout");
    assert_eq!(envelope["result"]["retention"]["keep"], 1);
    assert_eq!(envelope["result"]["retention"]["reclaims_storage"], false);
    assert_eq!(
        envelope["result"]["retention"]["pruned"],
        serde_json::json!(["linux-64/example-airlock-1.0.0-h0000001.conda"])
    );
}

#[test]
fn a_store_that_declares_retention_applies_it_without_the_flag() {
    let manifest = MANIFEST.replace(
        "writable = true",
        "writable = true\nmax_builds_per_version = 1",
    );
    let fixture = Fixture::with_manifest(&manifest);
    std::fs::remove_file(fixture.root().join("output/example-airlock-1.0.0-0.conda"))
        .expect("the default artifact");
    std::fs::remove_file(
        fixture
            .root()
            .join("output/fragments/example-airlock-linux-64.json"),
    )
    .expect("the default fragment");
    for (build, build_number) in [("h0000001", 1), ("h0000002", 2)] {
        let relative = format!("output/example-airlock-1.0.0-{build}.conda");
        let bytes = format!("conda bytes for {build}").into_bytes();
        std::fs::write(fixture.root().join(&relative), &bytes).expect("artifact");
        let digest = universe_workflow::sha256::sha256_hex(&bytes);
        std::fs::write(
            fixture
                .root()
                .join(format!("output/fragments/example-airlock-{build}.json")),
            format!(
                r#"{{"package":"example-airlock","version":"1.0.0","build":"{build}",
  "build_number":{build_number},"subdir":"linux-64","digest":"{digest}",
  "size_bytes":{},"depends":[],"license":"MIT","license_files":[],"path":"{relative}"}}"#,
                bytes.len()
            ),
        )
        .expect("fragment");
    }
    let remote = remote(fixture.root());

    let output = fixture.output_of(&[
        "publish",
        "--remote",
        remote.to_str().expect("utf-8"),
        "--branch",
        "channel",
    ]);

    assert_eq!(output.exit_code(), 0, "stderr: {}", output.stderr());
    assert_eq!(
        branch_packages(&remote, "channel"),
        ["example-airlock-1.0.0-h0000002.conda"]
    );
}

#[test]
fn keeping_nothing_is_refused_rather_than_publishing_an_empty_channel() {
    let fixture = Fixture::new();
    let remote = remote(fixture.root());

    let output = fixture.output_of(&[
        "publish",
        "--remote",
        remote.to_str().expect("utf-8"),
        "--branch",
        "channel",
        "--keep",
        "0",
    ]);

    assert_eq!(output.exit_code(), 2, "stderr: {}", output.stderr());
    assert!(
        output.stderr().contains("publish.retention.zero"),
        "{}",
        output.stderr()
    );
    assert!(!branch_exists(&remote, "channel"));
}
