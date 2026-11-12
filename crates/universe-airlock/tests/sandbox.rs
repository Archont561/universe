//! The sandbox: a built Conda package set, installed into a prefix, and a command run in it.
//!
//! Every test here drives the binary a user installs from the channel, because the sandbox is a
//! user-facing promise rather than a library API. What it promises is narrow on purpose: the
//! command sees the set's own `bin` first, the prefix's own interpreter as its Conda prefix, and
//! none of the host's Python configuration. It is an environment sandbox, not a machine one, and
//! the run says so.

use std::path::{Path, PathBuf};
use std::process::Command;

mod support;
use support::{Shipped, command, conda_package, data};

const BINARY: &str = env!("CARGO_BIN_EXE_universe-airlock");

/// A directory a test owns, deleted when it ends.
struct Scenario {
    root: PathBuf,
}

impl Scenario {
    fn new(label: &str) -> Self {
        let root = std::env::temp_dir().join(format!(
            "universe-airlock-sandbox-{}-{label}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).expect("create a scenario directory");
        Self { root }
    }

    /// Write one `.conda` into the scenario and return its path.
    fn package(&self, name: &str, depends: &[&str], shipped: &[Shipped]) -> PathBuf {
        let path = self.root.join(format!("{name}-0.1.0-h1234567_0.conda"));
        std::fs::write(&path, conda_package(name, depends, shipped)).expect("write a package");
        path
    }

    fn run(&self, arguments: &[&str]) -> Output {
        self.run_with(arguments, &[])
    }

    /// Run the binary with `arguments` and `environment` added to the inherited environment.
    fn run_with(&self, arguments: &[&str], environment: &[(&str, &str)]) -> Output {
        let mut invocation = Command::new(BINARY);
        invocation.args(arguments).current_dir(&self.root);
        for (name, value) in environment {
            invocation.env(name, value);
        }
        let output = invocation.output().expect("run universe-airlock");
        Output {
            code: output.status.code().unwrap_or(-1),
            stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        }
    }
}

impl Drop for Scenario {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

struct Output {
    code: i32,
    stdout: String,
    stderr: String,
}

/// The `sandbox:` line's value, which is the prefix the command ran in.
fn prefix_of(stdout: &str) -> String {
    stdout
        .lines()
        .find_map(|line| line.strip_prefix("sandbox: "))
        .unwrap_or_else(|| panic!("no `sandbox:` line in:\n{stdout}"))
        .trim()
        .to_owned()
}

/// A package whose one command prints the prefix it was installed into.
fn probe_package(scenario: &Scenario) -> PathBuf {
    scenario.package(
        "probe",
        &[],
        &[command(
            "bin/probe",
            "#!/bin/sh\nprintf 'prefix=%s\\n' \"$CONDA_PREFIX\"\n",
        )],
    )
}

#[test]
fn a_sandbox_runs_a_command_from_the_package_it_installed() {
    let scenario = Scenario::new("run");
    let package = probe_package(&scenario);

    let output = scenario.run(&[
        "sandbox",
        "--package",
        &package.display().to_string(),
        "--",
        "probe",
    ]);

    assert_eq!(output.code, 0, "{}", output.stderr);
    let prefix = prefix_of(&output.stdout);
    // The command ran out of the prefix the run reported, and it was the prefix's `CONDA_PREFIX`
    // rather than the host's — which is the whole of what "inside the sandbox" means here.
    assert!(
        output.stdout.contains(&format!("prefix={prefix}")),
        "the command did not see the sandbox as its prefix:\n{}",
        output.stdout
    );
    assert!(output.stdout.contains("artifacts: 1"), "{}", output.stdout);
}

#[test]
fn the_prefix_is_discarded_when_the_run_ends() {
    let scenario = Scenario::new("discard");
    let package = probe_package(&scenario);

    let output = scenario.run(&[
        "sandbox",
        "--package",
        &package.display().to_string(),
        "--",
        "probe",
    ]);

    let prefix = prefix_of(&output.stdout);
    assert!(
        !Path::new(&prefix).exists(),
        "the sandbox left {prefix} behind"
    );
}

#[test]
fn keeping_the_prefix_leaves_it_where_the_run_says() {
    let scenario = Scenario::new("keep");
    let package = probe_package(&scenario);

    let output = scenario.run(&[
        "sandbox",
        "--package",
        &package.display().to_string(),
        "--keep",
        "--",
        "probe",
    ]);

    assert_eq!(output.code, 0, "{}", output.stderr);
    let prefix = PathBuf::from(prefix_of(&output.stdout));
    assert!(prefix.join("bin/probe").is_file(), "{prefix:?} is empty");
}

#[test]
fn the_commands_exit_code_is_the_sandboxs_exit_code() {
    let scenario = Scenario::new("exit");
    let package = probe_package(&scenario);

    let output = scenario.run(&[
        "sandbox",
        "--package",
        &package.display().to_string(),
        "--",
        "sh",
        "-c",
        "exit 7",
    ]);

    assert_eq!(output.code, 7, "{}", output.stderr);
}

#[test]
fn the_host_python_configuration_does_not_reach_the_command() {
    // The failure this prevents: a host `PYTHONPATH` or an activated virtualenv makes a
    // `noarch: python` package import modules from outside the set it was installed with, and
    // the run passes for a reason that has nothing to do with the packages.
    let scenario = Scenario::new("pythonpath");
    let package = probe_package(&scenario);

    let output = scenario.run_with(
        &[
            "sandbox",
            "--package",
            &package.display().to_string(),
            "--",
            "sh",
            "-c",
            "printf 'PYTHONPATH=%s\\nVIRTUAL_ENV=%s\\nCONDA_PREFIX=%s\\n' \"$PYTHONPATH\" \"$VIRTUAL_ENV\" \"$CONDA_PREFIX\"",
        ],
        &[
            ("PYTHONPATH", "/host/python"),
            ("VIRTUAL_ENV", "/host/venv"),
        ],
    );

    assert_eq!(output.code, 0, "{}", output.stderr);
    assert!(
        !output.stdout.contains("/host/python"),
        "the host's PYTHONPATH reached the command:\n{}",
        output.stdout
    );
    assert!(
        !output.stdout.contains("/host/venv"),
        "an activated virtualenv reached the command:\n{}",
        output.stdout
    );
    let prefix = prefix_of(&output.stdout);
    assert!(
        output.stdout.contains(&format!("CONDA_PREFIX={prefix}")),
        "the command does not see the sandbox as its Conda prefix:\n{}",
        output.stdout
    );
}

#[test]
fn the_sets_own_commands_come_first_on_the_path() {
    let scenario = Scenario::new("path");
    // A command whose name exists on the host too: the set's version has to win, or a sandbox
    // silently runs the host's tool and the package's own is never exercised.
    let package = scenario.package(
        "sh",
        &[],
        &[command("bin/sh", "#!/bin/sh\nprintf 'the sandbox\\n'\n")],
    );

    let output = scenario.run(&[
        "sandbox",
        "--package",
        &package.display().to_string(),
        "--",
        "sh",
    ]);

    assert_eq!(output.code, 0, "{}", output.stderr);
    assert!(
        output.stdout.contains("the sandbox"),
        "the host's `sh` ran instead of the package's:\n{}",
        output.stdout
    );
}

#[test]
fn two_packages_live_in_one_sandbox() {
    let scenario = Scenario::new("two");
    let alpha = scenario.package("alpha", &[], &[command("bin/alpha", "#!/bin/sh\nexit 0\n")]);
    let beta = scenario.package("beta", &[], &[command("bin/beta", "#!/bin/sh\nexit 0\n")]);

    let output = scenario.run(&[
        "sandbox",
        "--package",
        &alpha.display().to_string(),
        "--package",
        &beta.display().to_string(),
        "--",
        "sh",
        "-c",
        "command -v alpha && command -v beta",
    ]);

    assert_eq!(output.code, 0, "{}", output.stderr);
    assert!(output.stdout.contains("artifacts: 2"), "{}", output.stdout);
    assert!(
        output.stdout.contains("order: alpha, beta"),
        "{}",
        output.stdout
    );
}

#[test]
fn a_set_that_cannot_be_installed_never_runs_the_command() {
    // A refusal that ran the command anyway would be worse than no sandbox: the user would see
    // their program's output and conclude the packages were fine.
    let scenario = Scenario::new("refuse");
    let broken = scenario.root.join("broken-0.1.0-h0.conda");
    std::fs::write(&broken, b"not an archive at all").expect("write a broken package");

    let output = scenario.run(&[
        "sandbox",
        "--package",
        &broken.display().to_string(),
        "--",
        "sh",
        "-c",
        "touch ran-anyway",
    ]);

    assert_eq!(output.code, 1, "{}", output.stderr);
    assert!(
        output.stderr.contains("airlock.prefix.archive"),
        "the refusal does not say what was wrong:\n{}",
        output.stderr
    );
    assert!(
        !scenario.root.join("ran-anyway").exists(),
        "the command ran even though the set could not be installed"
    );
}

#[test]
fn a_sandbox_needs_a_command() {
    let scenario = Scenario::new("no-command");
    let package = probe_package(&scenario);

    let output = scenario.run(&["sandbox", "--package", &package.display().to_string()]);

    assert_eq!(output.code, 2, "{}", output.stderr);
    assert!(
        output.stderr.contains("needs a command"),
        "{}",
        output.stderr
    );
}

#[test]
fn a_sandbox_with_nothing_to_install_is_refused_rather_than_empty() {
    // No `--package` and no mirror: an empty sandbox would run the command against the host and
    // look like it worked.
    let scenario = Scenario::new("no-set");

    let output = scenario.run(&["sandbox", "--", "sh", "-c", "exit 0"]);

    assert_eq!(output.code, 1, "{}", output.stderr);
    assert!(
        output.stderr.contains("airlock.replay.index"),
        "{}",
        output.stderr
    );
}

#[test]
fn a_directory_that_already_holds_something_is_refused() {
    // `--dir` is how a user keeps a sandbox around. Reusing a non-empty one would install this
    // set on top of another, and the result would be neither.
    let scenario = Scenario::new("occupied");
    let package = probe_package(&scenario);
    let directory = scenario.root.join("occupied");
    std::fs::create_dir_all(&directory).expect("create the directory");
    std::fs::write(directory.join("something"), b"here already").expect("write a file");

    let output = scenario.run(&[
        "sandbox",
        "--package",
        &package.display().to_string(),
        "--dir",
        &directory.display().to_string(),
        "--",
        "probe",
    ]);

    assert_eq!(output.code, 1, "{}", output.stderr);
    assert!(
        output.stderr.contains("airlock.sandbox.dir"),
        "{}",
        output.stderr
    );
}

#[test]
fn the_run_says_what_the_sandbox_is_not() {
    // No namespaces are created, so the command can still reach the network and the host
    // filesystem. Saying that on every run is the difference between a documented limit and a
    // false claim a user discovers later.
    let scenario = Scenario::new("note");
    let package = probe_package(&scenario);

    let output = scenario.run(&[
        "sandbox",
        "--package",
        &package.display().to_string(),
        "--",
        "probe",
    ]);

    assert!(
        output.stderr.contains("airlock.sandbox.environment"),
        "the run does not state its limits:\n{}",
        output.stderr
    );
}

#[test]
fn a_json_run_reports_the_command_and_its_output() {
    let scenario = Scenario::new("json");
    let package = probe_package(&scenario);

    let output = scenario.run(&[
        "sandbox",
        "--json",
        "--package",
        &package.display().to_string(),
        "--",
        "sh",
        "-c",
        "printf 'hello from the sandbox\\n'",
    ]);

    assert_eq!(output.code, 0, "{}", output.stderr);
    let value: serde_json::Value =
        serde_json::from_str(output.stdout.trim()).expect("one JSON envelope");
    assert_eq!(value["command"], "sandbox");
    assert_eq!(value["ok"], true);
    assert_eq!(value["result"]["exit"], 0);
    assert_eq!(value["result"]["artifacts"], 1);
    assert!(
        value["result"]["stdout"]
            .as_str()
            .unwrap_or_default()
            .contains("hello from the sandbox"),
        "{}",
        output.stdout
    );
}

#[test]
fn a_failing_command_is_still_a_reported_result() {
    // The envelope describes what happened; `ok` is about the sandbox doing its job, and the
    // exit code is the command's. A CI script needs both without parsing prose.
    let scenario = Scenario::new("json-failed");
    let package = probe_package(&scenario);

    let output = scenario.run(&[
        "sandbox",
        "--json",
        "--package",
        &package.display().to_string(),
        "--",
        "sh",
        "-c",
        "printf 'no\\n' >&2; exit 3",
    ]);

    assert_eq!(output.code, 3, "{}", output.stderr);
    let value: serde_json::Value =
        serde_json::from_str(output.stdout.trim()).expect("one JSON envelope");
    assert_eq!(value["result"]["exit"], 3);
    assert!(
        value["result"]["stderr"]
            .as_str()
            .unwrap_or_default()
            .contains("no"),
        "{}",
        output.stdout
    );
}

#[test]
fn a_package_that_ships_data_is_installed_with_it() {
    // The sandbox is a prefix like any other: what the archives carry is what lands, and an
    // entry point that reads a data file beside it has to find it.
    let scenario = Scenario::new("data");
    let package = scenario.package(
        "reader",
        &[],
        &[
            data("share/reader/value.txt", b"the value\n"),
            command(
                "bin/reader",
                "#!/bin/sh\nprintf 'read: '; cat \"$CONDA_PREFIX/share/reader/value.txt\"\n",
            ),
        ],
    );

    let output = scenario.run(&[
        "sandbox",
        "--package",
        &package.display().to_string(),
        "--",
        "reader",
    ]);

    assert_eq!(output.code, 0, "{}", output.stderr);
    assert!(
        output.stdout.contains("read: the value"),
        "{}",
        output.stdout
    );
}

#[test]
fn a_mirrored_lock_can_be_sandboxed_and_says_what_it_left_out() {
    // The Pixi user's case: the set is already a mirror. A lock usually pins wheels beside its
    // Conda packages, and a prefix holds the Conda ones — which the run has to say rather than
    // let the user assume their Python environment is complete.
    let scenario = Scenario::new("mirror");
    let package = probe_package(&scenario);
    let bytes = std::fs::read(&package).expect("read the package");
    let digest = support::digest(&bytes);
    let filename = "probe-0.1.0-h1234567_0.conda";
    let path = format!("blobs/sha256/{}/{digest}/{filename}", &digest[..2]);
    let blob = scenario.root.join("mirror").join(&path);
    std::fs::create_dir_all(blob.parent().expect("parent")).expect("mirror directory");
    std::fs::write(&blob, &bytes).expect("write the blob");
    let index = serde_json::json!({
        "schema": 1,
        "adapter": "pixi",
        "branch": "mirror/pixi",
        "lock_sha256": "1".repeat(64),
        "artifacts": [
            {
                "name": "probe",
                "version": "0.1.0",
                "kind": "conda",
                "subdir": "linux-64",
                "url": format!("https://example.invalid/linux-64/{filename}"),
                "sha256": digest,
                "size": bytes.len(),
                "path": path,
            },
            {
                "name": "requests",
                "version": "2.32.3",
                "kind": "wheel",
                "subdir": "any",
                "url": "https://example.invalid/requests-2.32.3-py3-none-any.whl",
                "sha256": "0".repeat(64),
                "size": 1,
                "path": format!("blobs/sha256/00/{}", "0".repeat(64)),
            },
        ],
    });
    std::fs::write(
        scenario.root.join("mirror/mirror.json"),
        format!("{index}\n"),
    )
    .expect("write the index");

    let output = scenario.run(&[
        "sandbox",
        "--mirror",
        &scenario.root.join("mirror").display().to_string(),
        "--",
        "probe",
    ]);

    assert_eq!(output.code, 0, "{}", output.stderr);
    assert!(
        output.stdout.contains("artifacts: 1 (1 left out)"),
        "{}",
        output.stdout
    );
    assert!(
        output.stderr.contains("requests"),
        "the note does not name the artifact it left out:\n{}",
        output.stderr
    );
}

#[test]
fn two_sources_are_a_usage_error() {
    let scenario = Scenario::new("two-sources");
    let package = probe_package(&scenario);

    let output = scenario.run(&[
        "sandbox",
        "--package",
        &package.display().to_string(),
        "--mirror",
        "elsewhere",
        "--",
        "probe",
    ]);

    assert_eq!(output.code, 2, "{}", output.stderr);
    assert!(output.stderr.contains("not both"), "{}", output.stderr);
}

#[test]
fn a_missing_command_is_a_refusal_and_leaves_no_prefix() {
    let scenario = Scenario::new("missing-command");
    let package = probe_package(&scenario);
    let directory = scenario.root.join("kept");
    let argument = directory.display().to_string();

    let output = scenario.run(&[
        "sandbox",
        "--package",
        &package.display().to_string(),
        "--dir",
        &argument,
        "--",
        "this-command-does-not-exist",
    ]);

    assert_eq!(output.code, 1, "{}", output.stderr);
    assert!(
        output.stderr.contains("airlock.sandbox.command"),
        "{}",
        output.stderr
    );
}

#[test]
fn a_named_directory_is_created_and_kept() {
    let scenario = Scenario::new("named-dir");
    let package = probe_package(&scenario);
    // Two levels that do not exist yet: `--dir` is where a caller points at a path, and a path
    // that is not there yet is what a first run has.
    let directory = scenario.root.join("output/prefix");

    let output = scenario.run(&[
        "sandbox",
        "--package",
        &package.display().to_string(),
        "--dir",
        &directory.display().to_string(),
        "--",
        "probe",
    ]);

    assert_eq!(output.code, 0, "{}", output.stderr);
    assert!(directory.join("bin/probe").is_file());
    assert!(
        output.stdout.contains("prefix kept: yes"),
        "{}",
        output.stdout
    );
}

#[test]
fn a_package_named_relative_to_the_working_directory_is_found() {
    let scenario = Scenario::new("relative");
    let _package = probe_package(&scenario);

    let output = scenario.run(&[
        "sandbox",
        "--package",
        "probe-0.1.0-h1234567_0.conda",
        "--",
        "probe",
    ]);

    assert_eq!(output.code, 0, "{}", output.stderr);
}

#[test]
fn a_json_run_kept_on_disk_says_so() {
    let scenario = Scenario::new("json-kept");
    let package = probe_package(&scenario);

    let output = scenario.run(&[
        "sandbox",
        "--json",
        "--keep",
        "--package",
        &package.display().to_string(),
        "--",
        "probe",
    ]);

    assert_eq!(output.code, 0, "{}", output.stderr);
    let value: serde_json::Value =
        serde_json::from_str(output.stdout.trim()).expect("one JSON envelope");
    let prefix = value["result"]["prefix"].as_str().expect("a prefix");
    assert!(Path::new(prefix).join("bin/probe").is_file());
}

#[test]
fn a_package_that_is_not_there_is_named_in_the_refusal() {
    let scenario = Scenario::new("absent-package");

    let output = scenario.run(&[
        "sandbox",
        "--package",
        "not-built-yet-0.1.0-h0.conda",
        "--",
        "sh",
        "-c",
        "exit 0",
    ]);

    assert_eq!(output.code, 1, "{}", output.stderr);
    assert!(
        output.stderr.contains("airlock.sandbox.package")
            && output.stderr.contains("not-built-yet-0.1.0-h0.conda"),
        "{}",
        output.stderr
    );
}

#[test]
fn an_empty_named_directory_is_used_as_it_is() {
    // A caller who made the directory themselves is not asked to remove it: empty is empty.
    let scenario = Scenario::new("empty-dir");
    let package = probe_package(&scenario);
    let directory = scenario.root.join("already-there");
    std::fs::create_dir_all(&directory).expect("create the directory");

    let output = scenario.run(&[
        "sandbox",
        "--package",
        &package.display().to_string(),
        "--dir",
        &directory.display().to_string(),
        "--",
        "probe",
    ]);

    assert_eq!(output.code, 0, "{}", output.stderr);
    assert!(directory.join("bin/probe").is_file());
}

#[test]
fn the_sandbox_does_not_need_a_host_path() {
    // The prefix's `bin` is the path the run builds; a caller with no `PATH` at all still gets
    // the package's command, because a shebang is absolute and the prefix is first anyway.
    let scenario = Scenario::new("no-path");
    let package = probe_package(&scenario);
    let output = Command::new(BINARY)
        .args([
            "sandbox",
            "--package",
            &package.display().to_string(),
            "--",
            "probe",
        ])
        .env_remove("PATH")
        .current_dir(&scenario.root)
        .output()
        .expect("run universe-airlock");

    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        String::from_utf8_lossy(&output.stdout).contains("prefix="),
        "{}",
        String::from_utf8_lossy(&output.stdout)
    );
}
