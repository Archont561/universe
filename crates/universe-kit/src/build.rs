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
use universe_core::planner::plan;

use crate::options::Options;
use crate::output::{FINDINGS, USAGE, failure, human, json_success};
use crate::project::Project;

/// One package that ran, and what it left behind.
#[derive(Clone, Debug, Eq, PartialEq)]
struct Built {
    package: String,
    command: String,
    outputs: Vec<String>,
}

/// Run `universe build`.
pub(crate) fn run(args: &[String]) -> CommandOutput {
    let options = match Options::parse(args, "build", true) {
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

        match execute(&options.root, package) {
            Ok(outputs) => built.push(Built {
                package: package.name.clone(),
                command: package.command.join(" "),
                outputs,
            }),
            Err(diagnostic) => return failure(FINDINGS, &[diagnostic]),
        }
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
            entry.command,
            entry.outputs.join(", ")
        ));
    }
    human(body)
}

/// Run one package's declared command and collect its declared outputs.
fn execute(
    root: &Path,
    package: &universe_core::manifest::Package,
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

    let completed = command.output().map_err(|error| {
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
