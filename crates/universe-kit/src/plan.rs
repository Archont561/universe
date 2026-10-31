//! `universe plan`: what would be built, in what order.
//!
//! The planning itself lives in `universe-core::planner`; this module is the command layer —
//! arguments in, envelope out, exit codes that mean one thing each. Keeping the two apart is
//! what lets the ordering rules be property-tested without a process and the CLI be tested
//! without a builder.

use universe_core::CommandOutput;
use universe_core::diagnostic::{Diagnostic, Severity};
use universe_core::planner::{PlanError, plan};

use crate::options::{Options, VerbSpec};
use crate::output::{FINDINGS, USAGE, failure, human, json_success};
use crate::project::Project;

/// Run `universe plan`.
pub(crate) fn run(args: &[String]) -> CommandOutput {
    let options = match Options::parse(args, VerbSpec::plan()) {
        Ok(options) => options,
        Err(message) => return usage_error(&message),
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
        Err(error) => return planning_error(error),
    };

    // A change filter narrows *what* runs; it never reorders it. A package a change can
    // affect, and everything that depends on it, stays in the plan.
    let planned = if options.changed_only {
        match affected(&project, &options.base_ref) {
            Ok(affected) => narrow_to(planned, &affected),
            Err(diagnostic) => return failure(USAGE, &[diagnostic]),
        }
    } else {
        planned
    };

    // A platform filter narrows the plan without reordering it: the levels are a property of
    // the dependency graph, not of which subdirs the caller asked about.
    let planned = match &options.platform {
        None => planned,
        Some(platform) => {
            let mut narrowed = planned.clone();
            for job in &mut narrowed.jobs {
                job.platforms.retain(|candidate| candidate == platform);
            }
            narrowed.jobs.retain(|job| !job.platforms.is_empty());
            let keep: Vec<String> = narrowed
                .jobs
                .iter()
                .map(|job| job.package.clone())
                .collect();
            narrowed.levels = narrowed
                .levels
                .iter()
                .map(|level| {
                    level
                        .iter()
                        .filter(|package| keep.contains(package))
                        .cloned()
                        .collect::<Vec<_>>()
                })
                .filter(|level| !level.is_empty())
                .collect();
            narrowed
        }
    };

    let result = planned.to_json_value();
    if options.json {
        return json_success("plan", result, Vec::new(), Vec::new());
    }
    human(describe(&planned.levels, &planned.jobs))
}

/// The packages a change between `base_ref` and the work tree can affect.
///
/// A package owns `crates/<name>/`, which is where its recipe lives; a change anywhere in that
/// subtree rebuilds it, and a change to any package rebuilds everything that depends on it.
/// Nothing here guesses: a git that cannot answer is a failure, not an empty plan, because an
/// empty plan reads as "nothing to do" and would skip a release.
fn affected(project: &Project, base_ref: &str) -> Result<Vec<String>, Diagnostic> {
    let output = std::process::Command::new("git")
        .arg("-C")
        .arg(&project.root)
        .args(["diff", "--name-only", &format!("{base_ref}...HEAD")])
        .output()
        .map_err(|error| {
            Diagnostic::new(
                Severity::Error,
                "plan.git.unavailable",
                format!("cannot run git: {error}"),
            )
        })?;
    if !output.status.success() {
        return Err(Diagnostic::new(
            Severity::Error,
            "plan.git.failed",
            format!(
                "cannot diff against {base_ref}: {}",
                String::from_utf8_lossy(&output.stderr).trim()
            ),
        ));
    }

    let changed: Vec<String> = String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(str::to_owned)
        .collect();

    let mut selected: Vec<String> = project
        .manifest
        .packages
        .iter()
        .filter(|package| {
            let owned = format!("crates/{}/", package.name);
            changed.iter().any(|path| path.starts_with(&owned))
        })
        .map(|package| package.name.clone())
        .collect();

    // A dependency that moved forwards takes its dependents with it.
    loop {
        let mut added = false;
        for package in &project.manifest.packages {
            if selected.contains(&package.name) {
                continue;
            }
            if package
                .depends_on
                .iter()
                .any(|dependency| selected.contains(dependency))
            {
                selected.push(package.name.clone());
                added = true;
            }
        }
        if !added {
            break;
        }
    }
    selected.sort();
    Ok(selected)
}

/// Keep only the selected packages, preserving each level's order.
fn narrow_to(
    planned: universe_core::planner::BuildPlan,
    selected: &[String],
) -> universe_core::planner::BuildPlan {
    let mut narrowed = planned.clone();
    narrowed.jobs.retain(|job| selected.contains(&job.package));
    narrowed.levels = narrowed
        .levels
        .iter()
        .map(|level| {
            level
                .iter()
                .filter(|package| selected.contains(package))
                .cloned()
                .collect::<Vec<_>>()
        })
        .filter(|level| !level.is_empty())
        .collect();
    narrowed
}

fn describe(levels: &[Vec<String>], jobs: &[universe_core::planner::PlanJob]) -> String {
    if levels.is_empty() {
        return "nothing to build\n".to_owned();
    }
    let mut body = String::new();
    for (index, level) in levels.iter().enumerate() {
        let members: Vec<String> = level
            .iter()
            .map(
                |package| match jobs.iter().find(|job| &job.package == package) {
                    Some(job) => format!("{package} ({})", job.platforms.join(", ")),
                    None => package.clone(),
                },
            )
            .collect();
        body.push_str(&format!("level {index}: {}\n", members.join(", ")));
    }
    body
}

fn planning_error(error: PlanError) -> CommandOutput {
    let diagnostic = match error {
        PlanError::Cycle { path } => Diagnostic::new(
            Severity::Error,
            "plan.cycle",
            format!("the dependency graph closes a cycle: {}", path.join(" -> ")),
        ),
        PlanError::UnknownDependency {
            dependent,
            dependency,
        } => Diagnostic::new(
            Severity::Error,
            "plan.dependency.unknown",
            format!("package '{dependent}' depends on unknown package '{dependency}'"),
        )
        .with_path(format!("packages[{dependent}].depends_on")),
    };
    failure(FINDINGS, &[diagnostic])
}

fn usage_error(message: &str) -> CommandOutput {
    failure(
        USAGE,
        &[Diagnostic::new(
            Severity::Error,
            "plan.usage.invalid",
            message,
        )],
    )
}
