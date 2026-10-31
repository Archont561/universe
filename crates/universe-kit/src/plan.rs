//! `universe plan`: what would be built, in what order.
//!
//! The planning itself lives in `universe-core::planner`; this module is the command layer —
//! arguments in, envelope out, exit codes that mean one thing each. Keeping the two apart is
//! what lets the ordering rules be property-tested without a process and the CLI be tested
//! without a builder.

use universe_core::CommandOutput;
use universe_core::diagnostic::{Diagnostic, Severity};
use universe_core::planner::{PlanError, plan};

use crate::options::Options;
use crate::output::{FINDINGS, USAGE, failure, human, json_success};
use crate::project::Project;

/// Run `universe plan`.
pub(crate) fn run(args: &[String]) -> CommandOutput {
    let options = match Options::parse(args, "plan", true) {
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
