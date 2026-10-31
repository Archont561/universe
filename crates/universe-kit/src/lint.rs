//! `universe lint`: everything a manifest can get wrong without running anything.
//!
//! Two families live here. The first is the manifest's own validation, which the schema
//! already owns — lint reports it rather than re-deciding it. The second is the one rule that
//! spans files: the workspace version, each recipe's `context.version`, and the changelog
//! section that has to exist for a version that is released.
//!
//! The recipe rules that need a YAML parser with spans (sources, digests, SPDX licences)
//! arrive with `UNI-7` proper; what is here reads the two literals parity depends on.

use std::path::{Path, PathBuf};

use serde_json::json;
use universe_core::CommandOutput;
use universe_core::diagnostic::{Diagnostic, Severity};

use universe_conda::{RecipeRules, lint_recipe};

use crate::options::{Options, VerbSpec};
use crate::output::{FINDINGS, USAGE, failure, human, json_success};
use crate::project::Project;

/// Run `universe lint`.
pub(crate) fn run(args: &[String]) -> CommandOutput {
    let options = match Options::parse(args, VerbSpec::lint()) {
        Ok(options) => options,
        Err(message) => {
            return failure(
                USAGE,
                &[Diagnostic::new(
                    Severity::Error,
                    "lint.usage.invalid",
                    message,
                )],
            );
        }
    };

    let project = match Project::load(&options.root) {
        Ok(project) => project,
        // An unparsable manifest is the loudest lint failure there is, and it is reported by
        // the parser's own diagnostics rather than by a rule invented here.
        Err(output) => return output,
    };

    let mut diagnostics = project.manifest.validate();
    diagnostics.extend(version_parity(&project.root));
    diagnostics.extend(recipe_findings(&project));

    let has_errors = diagnostics.iter().any(Diagnostic::is_error);
    if options.json {
        if has_errors {
            return failure(FINDINGS, &diagnostics);
        }
        let warnings = diagnostics
            .iter()
            .map(|diagnostic| json!({ "code": diagnostic.code(), "message": diagnostic.message() }))
            .collect::<Vec<_>>();
        return json_success(
            "lint",
            json!({ "packages": project.manifest.packages.len(), "warnings": warnings }),
            diagnostics,
            Vec::new(),
        );
    }

    if has_errors {
        return failure(FINDINGS, &diagnostics);
    }
    let mut body = String::new();
    for diagnostic in &diagnostics {
        body.push_str(&format!(
            "warning: {}: {}\n",
            diagnostic.code(),
            diagnostic.message()
        ));
    }
    body.push_str("ok\n");
    human(body)
}

/// Every recipe, linted with the rules the manifest's `[audit]` section declares.
///
/// The rules come from the manifest rather than from a flag so that turning a rule off is a
/// reviewable change to a committed file, not something a contributor can do for one run.
fn recipe_findings(project: &Project) -> Vec<Diagnostic> {
    let rules = RecipeRules {
        allow_mutable_git: false,
        enforce_license_file: project.manifest.audit.enforce_license_file,
        require_valid_spdx: project.manifest.audit.require_valid_spdx,
    };
    let mut findings = Vec::new();
    for recipe in recipes(&project.root.join("crates")) {
        let Ok(text) = std::fs::read_to_string(&recipe) else {
            continue;
        };
        let directory = recipe.parent().unwrap_or(&project.root);
        for finding in lint_recipe(directory, &text, rules) {
            findings.push(finding.with_path(relative(&project.root, &recipe)));
        }
    }
    findings
}

fn relative(root: &std::path::Path, path: &std::path::Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .into_owned()
}

/// The workspace version, every recipe's `context.version`, and the changelog.
fn version_parity(root: &Path) -> Vec<Diagnostic> {
    let Some(workspace) = workspace_version(root) else {
        return vec![
            Diagnostic::new(
                Severity::Error,
                "lint.version.unknown",
                "the workspace declares no [workspace.package] version",
            )
            .with_path("Cargo.toml"),
        ];
    };

    let mut diagnostics = Vec::new();
    let recipe_root = root.join("crates");
    for recipe in recipes(&recipe_root) {
        let Ok(text) = std::fs::read_to_string(&recipe) else {
            continue;
        };
        match context_version(&text) {
            Some(version) if version == workspace => {}
            Some(version) => diagnostics.push(
                Diagnostic::new(
                    Severity::Error,
                    "lint.version.recipe",
                    format!(
                        "recipe declares version {version}, the workspace declares {workspace}"
                    ),
                )
                .with_path(recipe.display().to_string()),
            ),
            None => diagnostics.push(
                Diagnostic::new(
                    Severity::Error,
                    "lint.version.recipe",
                    "recipe declares no context.version".to_owned(),
                )
                .with_path(recipe.display().to_string()),
            ),
        }
    }

    if let Some(section) = changelog_problem(root, &workspace) {
        diagnostics.push(section);
    }
    diagnostics
}

fn workspace_version(root: &Path) -> Option<String> {
    let text = std::fs::read_to_string(root.join("Cargo.toml")).ok()?;
    let document = text.parse::<toml_edit::DocumentMut>().ok()?;
    document
        .get("workspace")?
        .get("package")?
        .get("version")?
        .as_str()
        .map(str::to_owned)
}

/// Every `recipe.yaml` reachable from `crates/`, sorted, so findings are stable.
fn recipes(root: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    let Ok(entries) = std::fs::read_dir(root) else {
        return found;
    };
    for entry in entries.flatten() {
        let recipe = entry.path().join("recipe/recipe.yaml");
        if recipe.is_file() {
            found.push(recipe);
        }
    }
    found.sort();
    found
}

/// The recipe's `context.version` literal, read without a YAML parser.
///
/// Two literal forms matter here and both are line-anchored, so a scan is honest about what
/// it covers: a full YAML parse with spans is `UNI-7`'s job, and until it lands this must
/// not pretend to more than it checks.
fn context_version(text: &str) -> Option<String> {
    let mut in_context = false;
    for line in text.lines() {
        let trimmed = line.trim_end();
        if trimmed == "context:" {
            in_context = true;
            continue;
        }
        if in_context {
            let indented = line.starts_with(' ') || line.starts_with('\t');
            if !indented && !trimmed.is_empty() {
                return None;
            }
            if let Some(value) = line.trim().strip_prefix("version:") {
                return Some(value.trim().trim_matches(['"', '\'']).to_owned());
            }
        }
    }
    None
}

/// A repository that has begun cutting releases must have a section for the version it
/// claims. A changelog that is still one `Unreleased` block has not started, and is allowed:
/// the release that gives it a heading is the release that would otherwise be unannounced.
fn changelog_problem(root: &Path, workspace: &str) -> Option<Diagnostic> {
    let text = std::fs::read_to_string(root.join("CHANGELOG.md")).ok()?;
    let released: Vec<String> = text
        .lines()
        .filter_map(|line| line.strip_prefix("## "))
        .map(|heading| {
            heading
                .trim()
                .trim_start_matches('[')
                .trim_end_matches(']')
                .to_owned()
        })
        .filter(|heading| {
            heading
                .chars()
                .next()
                .is_some_and(|first| first.is_ascii_digit())
        })
        .collect();
    if released.is_empty() || released.iter().any(|heading| heading == workspace) {
        return None;
    }
    Some(
        Diagnostic::new(
            Severity::Error,
            "lint.version.changelog",
            format!("no CHANGELOG.md section for version {workspace}"),
        )
        .with_path("CHANGELOG.md"),
    )
}

#[cfg(test)]
mod tests {
    use super::{changelog_problem, context_version};
    use std::path::Path;

    #[test]
    fn a_quoted_context_version_is_read_without_its_quotes() {
        let recipe = "context:\n  name: example\n  version: \"1.2.3\"\n";
        assert_eq!(context_version(recipe).as_deref(), Some("1.2.3"));
    }

    #[test]
    fn a_version_outside_the_context_block_is_not_the_recipe_version() {
        let recipe = "context:\n  name: example\npackage:\n  version: \"1.2.3\"\n";
        assert_eq!(context_version(recipe), None);
    }

    #[test]
    fn an_unreleased_only_changelog_is_allowed() {
        let directory = tempfile::tempdir().expect("a temporary directory");
        std::fs::write(
            directory.path().join("CHANGELOG.md"),
            "## Unreleased\n\n- x\n",
        )
        .expect("writes");
        assert!(changelog_problem(directory.path(), "1.0.0").is_none());
    }

    #[test]
    fn a_released_changelog_without_the_current_version_is_refused() {
        let directory = tempfile::tempdir().expect("a temporary directory");
        std::fs::write(directory.path().join("CHANGELOG.md"), "## 0.9.0\n\n- x\n").expect("writes");
        let diagnostic = changelog_problem(directory.path(), "1.0.0").expect("a finding");
        assert_eq!(diagnostic.code(), "lint.version.changelog");
    }

    #[test]
    fn a_missing_changelog_is_not_a_version_problem() {
        assert!(changelog_problem(Path::new("/nonexistent-root"), "1.0.0").is_none());
    }
}
