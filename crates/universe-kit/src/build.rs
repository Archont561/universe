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
    // Only the declared environment is passed. Inheriting the parent's would hand a build
    // every token this session holds, which is exactly what `UNI-9` #4 forbids.
    command.env_clear();
    for (name, value) in &package.env {
        command.env(name, value);
    }
    // Enough of an environment to be runnable at all: a cleared PATH cannot find `pixi`.
    if let Some(path) = std::env::var_os("PATH") {
        command.env("PATH", path);
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
        let stderr = String::from_utf8_lossy(&completed.stderr);
        let detail = stderr
            .lines()
            .last()
            .unwrap_or("no stderr")
            .trim()
            .to_owned();
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

fn empty_command(package: &str) -> Diagnostic {
    Diagnostic::new(
        Severity::Error,
        "build.command.empty",
        format!("package '{package}' has no command to run"),
    )
    .with_path(format!("packages[{package}].command"))
}
