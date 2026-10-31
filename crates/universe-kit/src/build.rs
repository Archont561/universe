//! `universe build`: run what the manifest declares, collect what it says it produced.
//!
//! Kit owns no builder. A package names a command; this module runs exactly that command, in
//! the project root, with only the environment the package declares, and then collects the
//! outputs the package promised. Nothing here inspects the command to guess what it does,
//! because a tool that special-cases its builders cannot be audited as a thin runner.

use std::path::Path;
use std::process::Command;

use serde_json::json;
use universe_core::CommandOutput;
use universe_core::diagnostic::{Diagnostic, Severity};
use universe_core::manifest::Package;
use universe_core::planner::plan;

use crate::options::{Options, VerbSpec};
use crate::output::{FINDINGS, USAGE, failure, human, json_success};
use crate::project::Project;

/// One package that ran, and what it left behind.
#[derive(Clone, Debug, Eq, PartialEq)]
struct Built {
    package: String,
    command: Vec<String>,
    duration_ms: u128,
    outputs: Vec<String>,
}

/// Run `universe build`.
pub(crate) fn run(args: &[String]) -> CommandOutput {
    let options = match Options::parse(args, VerbSpec::build()) {
        Ok(options) => options,
        Err(message) => {
            return failure(
                USAGE,
                &[Diagnostic::new(
                    Severity::Error,
                    "build.usage.invalid",
                    message,
                )],
            );
        }
    };

    let project = match Project::load(&options.root) {
        Ok(project) => project,
        Err(output) => return output,
    };
    if let Err(output) = project.require_valid() {
        return output;
    }

    let planned = match plan(&project.manifest) {
        Ok(planned) => planned,
        Err(error) => {
            return failure(
                FINDINGS,
                &[Diagnostic::new(
                    Severity::Error,
                    "build.plan.failed",
                    format!("the manifest cannot be ordered: {error:?}"),
                )],
            );
        }
    };

    let mut built = Vec::new();
    for job in &planned.jobs {
        if let Some(platform) = &options.platform
            && !job.platforms.iter().any(|candidate| candidate == platform)
        {
            continue;
        }
        let Some(package) = project
            .manifest
            .packages
            .iter()
            .find(|package| package.name == job.package)
        else {
            continue;
        };

        let started = std::time::Instant::now();
        match execute(&options.root, package, options.timeout_seconds) {
            Ok(outputs) => built.push(Built {
                package: package.name.clone(),
                command: package.command.clone(),
                duration_ms: started.elapsed().as_millis(),
                outputs,
            }),
            Err(diagnostic) => return failure(FINDINGS, &[diagnostic]),
        }
    }

    if let Some(directory) = &options.receipts
        && let Err(error) = write_receipts(&options.root, directory, &built)
    {
        return failure(
            USAGE,
            &[Diagnostic::new(
                Severity::Error,
                "build.receipt.write_failed",
                error,
            )],
        );
    }

    if options.json {
        let result = json!({
            "built": built
                .iter()
                .map(|entry| json!({
                    "package": entry.package,
                    "command": entry.command,
                    "outputs": entry.outputs,
                }))
                .collect::<Vec<_>>(),
        });
        return json_success("build", result, Vec::new(), Vec::new());
    }
    if built.is_empty() {
        return human("nothing to build\n".to_owned());
    }
    let mut body = String::new();
    for entry in &built {
        body.push_str(&format!(
            "{}: {} -> {}\n",
            entry.package,
            entry.command.join(" "),
            entry.outputs.join(", ")
        ));
    }
    human(body)
}

/// Write one receipt per package: what ran, where, and the bytes that came out.
///
/// The receipt is what a later publish lane consumes, so it records the artifacts by path,
/// size, and digest rather than trusting whoever built them. Only the duration varies between
/// two identical runs, and it is recorded rather than omitted because a build that suddenly
/// takes ten times as long is a finding of its own.
fn write_receipts(root: &Path, directory: &Path, built: &[Built]) -> Result<(), String> {
    for entry in built {
        let outputs = entry
            .outputs
            .iter()
            .map(|output| {
                let bytes = std::fs::read(root.join(output))
                    .map_err(|error| format!("cannot read {output}: {error}"))?;
                Ok(json!({
                    "path": output,
                    "size_bytes": bytes.len(),
                    "digest": universe_workflow::sha256::sha256_hex(&bytes),
                }))
            })
            .collect::<Result<Vec<_>, String>>()?;
        let receipt = json!({
            "package": entry.package,
            "command": entry.command,
            "cwd": ".",
            "duration_ms": entry.duration_ms,
            "outputs": outputs,
        });
        let document = serde_json::to_string_pretty(&receipt)
            .map_err(|error| format!("cannot serialize a receipt: {error}"))?;
        let path = directory.join(format!("{}.json", entry.package));
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|error| format!("cannot create {}: {error}", parent.display()))?;
        }
        std::fs::write(&path, format!("{document}\n"))
            .map_err(|error| format!("cannot write {}: {error}", path.display()))?;
    }
    Ok(())
}

/// Run one package's declared command and collect its declared outputs.
fn execute(
    root: &Path,
    package: &universe_core::manifest::Package,
    timeout_seconds: u64,
) -> Result<Vec<String>, Diagnostic> {
    let (program, arguments) = package
        .command
        .split_first()
        .ok_or_else(|| empty_command(&package.name))?;

    let mut command = Command::new(program);
    command.args(arguments).current_dir(root);
    // The child sees the declared environment and a named set of ambient variables, and
    // nothing else. Inheriting the parent's environment would hand a build every token this
    // session holds — the thing `UNI-9` #4 forbids — while clearing it outright leaves a
    // toolchain without a home directory or a temporary directory to work in.
    command.env_clear();
    for (name, value) in std::env::vars_os() {
        if ambient(&name.to_string_lossy()) {
            command.env(name, value);
        }
    }
    for (name, value) in &package.env {
        command.env(name, value);
    }

    let completed = run_bounded(command, timeout_seconds).map_err(|error| {
        Diagnostic::new(
            Severity::Error,
            "build.command.unrunnable",
            format!("cannot run `{program}`: {error}"),
        )
        .with_path(format!("packages[{}].command", package.name))
    })?;

    if !completed.status.success() {
        let detail = stderr_tail(&String::from_utf8_lossy(&completed.stderr));
        return Err(Diagnostic::new(
            Severity::Error,
            "build.command.failed",
            format!(
                "`{}` exited with {}: {detail}",
                package.command.join(" "),
                completed.status
            ),
        )
        .with_path(format!("packages[{}].command", package.name)));
    }

    let mut outputs = Vec::new();
    for pattern in &package.outputs {
        for path in crate::paths::collect(root, pattern) {
            let relative = path
                .strip_prefix(root)
                .unwrap_or(&path)
                .to_string_lossy()
                .into_owned();
            if !outputs.contains(&relative) {
                outputs.push(relative);
            }
        }
    }
    // The collected files have to be the package they claim to be, before anything downstream
    // audits or publishes them: a build that produced a differently-identified archive is a
    // build whose output directory was stale.
    for output in &outputs {
        verify_identity(root, output, package)?;
    }
    if outputs.is_empty() {
        return Err(Diagnostic::new(
            Severity::Error,
            "build.output.missing",
            format!(
                "package '{}' built without producing any of {}",
                package.name,
                package.outputs.join(", ")
            ),
        )
        .with_path(format!("packages[{}].outputs", package.name)));
    }
    Ok(outputs)
}

/// What a bounded run produced.
struct Completed {
    status: std::process::ExitStatus,
    stderr: Vec<u8>,
}

/// Run a command to completion, or kill it at `timeout_seconds`.
///
/// A builder that hangs is the failure mode a runner notices last: the job sits until the
/// platform's six-hour ceiling and reports nothing useful. The limit is per command, so it
/// bounds the thing that can hang rather than the whole job.
fn run_bounded(mut command: Command, timeout_seconds: u64) -> std::io::Result<Completed> {
    use std::io::Read;

    let started = std::time::Instant::now();
    let mut child = command
        .stderr(std::process::Stdio::piped())
        .stdout(std::process::Stdio::null())
        .spawn()?;
    loop {
        if let Some(status) = child.try_wait()? {
            let mut stderr = Vec::new();
            if let Some(mut pipe) = child.stderr.take() {
                let _ = pipe.read_to_end(&mut stderr);
            }
            return Ok(Completed { status, stderr });
        }
        if started.elapsed().as_secs() >= timeout_seconds {
            let _ = child.kill();
            let status = child.wait()?;
            return Ok(Completed {
                status,
                stderr: format!("timed out after {timeout_seconds}s").into_bytes(),
            });
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
}

/// Check a collected artifact against the package that claims it.
///
/// A Conda archive answers for itself: `info/index.json` names the package, so the check is a
/// read rather than an inference from the file name. Anything else is checked by name only.
fn verify_identity(root: &Path, output: &str, package: &Package) -> Result<(), Diagnostic> {
    if package.kind != universe_core::manifest::PackageKind::Conda || !output.ends_with(".conda") {
        return Ok(());
    }
    let path = root.join(output);
    let opened = universe_conda::open_package(&path).map_err(|error| {
        Diagnostic::new(
            Severity::Error,
            "build.output.unreadable",
            format!("{output}: {error}"),
        )
        .with_path(format!("packages[{}].outputs", package.name))
    })?;
    if opened.index().name != package.name {
        return Err(Diagnostic::new(
            Severity::Error,
            "build.output.identity",
            format!(
                "{output} is package '{}' inside the archive but '{}' in the manifest",
                opened.index().name,
                package.name
            ),
        )
        .with_path(format!("packages[{}].outputs", package.name)));
    }
    Ok(())
}

/// The last few lines a failed command wrote, on one line and bounded.
///
/// A build tool's final line is usually the one that does not say what broke, and the whole
/// log lives behind a URL this job cannot publish. So the tail travels in the diagnostic,
/// which is what reaches an annotation a reader can actually fetch.
fn stderr_tail(stderr: &str) -> String {
    const LINES: usize = 6;
    const CHARS: usize = 800;
    let lines: Vec<&str> = stderr
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect();
    let tail = lines[lines.len().saturating_sub(LINES)..].join(" | ");
    match tail.char_indices().nth(CHARS) {
        Some((cut, _)) => format!("{}…", &tail[..cut]),
        None if tail.is_empty() => "no stderr".to_owned(),
        None => tail,
    }
}

/// Ambient variables a build toolchain needs and that carry no credential.
///
/// The list is an allowlist on purpose: a new variable has to be argued for here, and a
/// secret that happens to be in the environment is excluded by default rather than by luck.
/// `PATH` is how `pixi` is found; `HOME` and the temp variables are where compilers and
/// package managers keep their caches; the locale and certificate variables keep output
/// parsing and TLS working; the proxy variables are the network's routing, not its identity.
fn ambient(name: &str) -> bool {
    const ALLOWED: [&str; 15] = [
        "PATH",
        "HOME",
        "TMPDIR",
        "TEMP",
        "TMP",
        "LANG",
        "LANGUAGE",
        "LC_ALL",
        "SSL_CERT_FILE",
        "SSL_CERT_DIR",
        "HTTP_PROXY",
        "HTTPS_PROXY",
        "NO_PROXY",
        "http_proxy",
        "https_proxy",
    ];
    ALLOWED.contains(&name) || name == "SYSTEMROOT" || name == "no_proxy"
}

fn empty_command(package: &str) -> Diagnostic {
    Diagnostic::new(
        Severity::Error,
        "build.command.empty",
        format!("package '{package}' has no command to run"),
    )
    .with_path(format!("packages[{package}].command"))
}

#[cfg(test)]
mod tests {
    use super::{ambient, stderr_tail};

    #[test]
    fn the_tail_keeps_the_lines_that_explain_the_failure() {
        let stderr = "resolving\nwarning: slow\nerror: recipe not found\n";
        assert_eq!(
            stderr_tail(stderr),
            "resolving | warning: slow | error: recipe not found"
        );
    }

    #[test]
    fn the_tail_is_bounded_and_says_so_when_a_command_said_nothing() {
        assert_eq!(stderr_tail("  \n\n"), "no stderr");
        let long = "x".repeat(2_000);
        assert!(stderr_tail(&long).chars().count() <= 801);
    }

    #[test]
    fn the_ambient_allowlist_carries_what_a_toolchain_needs() {
        for name in [
            "PATH",
            "HOME",
            "TMPDIR",
            "LANG",
            "SSL_CERT_FILE",
            "HTTPS_PROXY",
        ] {
            assert!(ambient(name), "{name} must reach a build");
        }
    }

    #[test]
    fn the_ambient_allowlist_carries_no_credential() {
        // Named for the failure that matters: a token in the environment must not become a
        // token in a child process, whatever it is called.
        for name in [
            "UNIVERSE_CHANNEL_TOKEN",
            "CONDA_PREFIX_CONDA_TOKEN",
            "GITHUB_TOKEN",
            "AWS_SECRET_ACCESS_KEY",
            "PIXI_INSTALL_TOKEN",
        ] {
            assert!(!ambient(name), "{name} must not reach a build");
        }
    }
}
