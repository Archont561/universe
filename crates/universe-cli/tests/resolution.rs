//! The facade as a user meets it: one binary, invoked by absolute path.
//!
//! GitHub issue 125 was filed from a pixi prefix where the facade was installed by absolute
//! path and its bin directory was not on `PATH`. The unit tests pin the search order against a
//! `Search` a test can build; these run the real binary and let it build its own, because the
//! part that was wrong — `Command::new` with a bare name — only exists in `main`.
//!
//! Unix only: every component here is a shell script, which is also how the reported failure
//! was observed.
#![cfg(unix)]

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

/// A directory of this test's own, removed when it goes out of scope.
struct Scratch(PathBuf);

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

impl Scratch {
    fn path(&self) -> &Path {
        &self.0
    }
}

fn scratch(label: &str) -> Scratch {
    use std::sync::atomic::{AtomicUsize, Ordering};
    static COUNTER: AtomicUsize = AtomicUsize::new(0);
    let unique = format!(
        "universe-cli-e2e-{label}-{}-{}",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::Relaxed)
    );
    let path = std::env::temp_dir().join(unique);
    std::fs::create_dir_all(&path).expect("scratch directory");
    Scratch(path)
}

/// A component that says which copy of it ran.
fn component_in(directory: &Path, name: &str, says: &str) -> PathBuf {
    use std::os::unix::fs::PermissionsExt;
    std::fs::create_dir_all(directory).expect("component directory");
    let path = directory.join(name);
    std::fs::write(&path, format!("#!/bin/sh\nprintf '%s\\n' {says}\n")).expect("component");
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).expect("executable");
    path
}

/// The facade this test built, copied somewhere it can be run from.
fn facade_in(directory: &Path) -> PathBuf {
    let source = env!("CARGO_BIN_EXE_universe");
    let destination = directory.join("universe");
    std::fs::copy(source, &destination).expect("copy the facade");
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(&destination, std::fs::Permissions::from_mode(0o755))
        .expect("executable");
    destination
}

/// Run a facade with nowhere to look but what the arguments say.
fn run(facade: &Path, args: &[&str], path: &Path, override_dir: Option<&Path>) -> Output {
    let mut command = Command::new(facade);
    command
        .args(args)
        .env("PATH", path)
        .env_remove("UNIVERSE_COMPONENT_DIR");
    if let Some(directory) = override_dir {
        command.env("UNIVERSE_COMPONENT_DIR", directory);
    }
    command.output().expect("the facade runs")
}

#[test]
fn a_facade_run_by_absolute_path_finds_the_component_installed_beside_it() {
    // The issue, exactly: absolute-path invocation, nothing useful on PATH, the component in
    // the same directory as the facade.
    let prefix = scratch("prefix");
    let facade = facade_in(prefix.path());
    component_in(prefix.path(), "universe-kit", "beside");

    let output = run(&facade, &["--version"], Path::new("/nonexistent"), None);

    assert_eq!(
        output.status.code(),
        Some(0),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "beside");
}

#[test]
fn a_stray_component_earlier_on_path_does_not_shadow_the_installed_one() {
    let prefix = scratch("shadow-prefix");
    let facade = facade_in(prefix.path());
    component_in(prefix.path(), "universe-kit", "installed");
    let stray = scratch("shadow-stray");
    component_in(stray.path(), "universe-kit", "stray");

    let output = run(&facade, &["--version"], stray.path(), None);

    assert_eq!(output.status.code(), Some(0));
    assert_eq!(
        String::from_utf8_lossy(&output.stdout).trim(),
        "installed",
        "the facade ran a component that is not the one installed with it"
    );
}

#[test]
fn an_override_directory_outranks_the_component_beside_the_facade() {
    let prefix = scratch("override-prefix");
    let facade = facade_in(prefix.path());
    component_in(prefix.path(), "universe-kit", "installed");
    let staged = scratch("override-staged");
    component_in(staged.path(), "universe-kit", "staged");

    let output = run(
        &facade,
        &["--version"],
        Path::new("/nonexistent"),
        Some(staged.path()),
    );

    assert_eq!(output.status.code(), Some(0));
    assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "staged");
}

#[test]
fn a_missing_component_exits_127_with_a_remedy_rather_than_an_os_error() {
    let prefix = scratch("missing-prefix");
    let facade = facade_in(prefix.path());

    let output = run(&facade, &["--version"], Path::new("/nonexistent"), None);

    assert_eq!(output.status.code(), Some(127));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("universe-kit"), "{stderr}");
    assert!(stderr.contains("UNIVERSE_COMPONENT_DIR"), "{stderr}");
    assert!(
        !stderr.contains("os error"),
        "the diagnostic is a bare OS error: {stderr}"
    );
    assert!(
        output.stdout.is_empty(),
        "a failure wrote to stdout: {}",
        String::from_utf8_lossy(&output.stdout)
    );
}
