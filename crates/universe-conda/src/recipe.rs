//! What a recipe has to prove before anything is built from it.
//!
//! A recipe is the last configuration between a source and a published artifact, and every
//! serious defect in one is silent: an unpinned `sha256` downloads different bytes next week,
//! a mutable git ref builds a different commit, an invalid SPDX expression fails a consumer's
//! licence check long after release, and a negative build number sorts below the artifact it
//! replaces. Each is cheap to refuse here and expensive to discover later.
//!
//! The parser is `serde_yaml`, so a finding names the key path it concerns (`source[0].sha256`)
//! rather than a line and column. That is a correction to the task that specified
//! `marked-yaml`: the span-carrying parser is not in this repository's offline transport, and
//! a lint that cannot be run is worse than one that points at the key.

use std::path::Path;

use rattler_conda_types::Version;
use serde_yaml::{Mapping, Value};
use universe_core::diagnostic::{Diagnostic, Severity};

/// Which rules a recipe lint applies.
///
/// The defaults are the strict ones, matching `universe.toml`'s `[audit]` defaults: a rule a
/// contributor has to remember to switch on is a rule that is off.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RecipeRules {
    /// Accept a git source pinned to a branch or tag instead of a commit.
    pub allow_mutable_git: bool,
    /// Require `about.license_file` and require every file it names to exist.
    pub enforce_license_file: bool,
    /// Require `about.license` to be a valid SPDX expression.
    pub require_valid_spdx: bool,
}

impl Default for RecipeRules {
    fn default() -> Self {
        Self {
            allow_mutable_git: false,
            enforce_license_file: true,
            require_valid_spdx: true,
        }
    }
}

/// Lint one recipe.
///
/// `recipe_directory` is where the recipe lives, because `license_file` is resolved relative
/// to it. Returns every finding, in document order; an empty list means the recipe passed.
#[must_use]
pub fn lint_recipe(recipe_directory: &Path, text: &str, rules: RecipeRules) -> Vec<Diagnostic> {
    let Ok(document) = serde_yaml::from_str::<Value>(text) else {
        return vec![Diagnostic::new(
            Severity::Error,
            "recipe.parse.failed",
            "the recipe is not valid YAML".to_owned(),
        )];
    };
    let Some(root) = document.as_mapping() else {
        return vec![Diagnostic::new(
            Severity::Error,
            "recipe.parse.failed",
            "the recipe is not a mapping".to_owned(),
        )];
    };

    let mut findings = Vec::new();
    check_version(root, &mut findings);
    check_sources(root, rules, &mut findings);

    // A multi-output recipe moves `package`, `build`, and `about` into each output; a
    // single-output one keeps them at the top. Checking both shapes at the top would refuse
    // every correct multi-output recipe, so the shape decides which checks run.
    let outputs = root.get(Value::String("outputs".to_owned()));
    if let Some(Value::Sequence(outputs)) = outputs {
        for (index, output) in outputs.iter().enumerate() {
            let Some(mapping) = output.as_mapping() else {
                findings.push(
                    Diagnostic::new(
                        Severity::Error,
                        "recipe.output.invalid",
                        "an output is not a mapping".to_owned(),
                    )
                    .with_path(format!("outputs[{index}]")),
                );
                continue;
            };
            check_output_package(mapping, index, &mut findings);
            check_build(mapping, &mut findings);
            check_about(recipe_directory, mapping, rules, &mut findings);
        }
    } else {
        check_package(root, &mut findings);
        check_build(root, &mut findings);
        check_about(recipe_directory, root, rules, &mut findings);
    }
    findings
}

/// The recipe's version: `context.version`, or a literal `package.version`.
fn check_version(root: &Mapping, findings: &mut Vec<Diagnostic>) -> Option<String> {
    let literal = literal_at(root, &["context", "version"])
        .or_else(|| literal_at(root, &["package", "version"]))
        .filter(|value| !value.contains("${{"));
    let Some(version) = literal else {
        findings.push(
            Diagnostic::new(
                Severity::Error,
                "recipe.version.missing",
                "the recipe declares no literal context.version or package.version",
            )
            .with_path("context.version"),
        );
        return None;
    };
    match version.parse::<Version>() {
        Ok(_) => Some(version),
        Err(error) => {
            findings.push(
                Diagnostic::new(
                    Severity::Error,
                    "recipe.version.invalid",
                    format!("'{version}' is not a Conda version: {error}"),
                )
                .with_path("context.version"),
            );
            None
        }
    }
}

fn check_package(root: &Mapping, findings: &mut Vec<Diagnostic>) {
    let name = literal_at(root, &["package", "name"]).unwrap_or_default();
    if name.trim().is_empty() {
        findings.push(
            Diagnostic::new(
                Severity::Error,
                "recipe.package.name.missing",
                "the recipe declares no package.name",
            )
            .with_path("package.name"),
        );
    }
    check_literal_version(root, "package.version", findings);
}

/// One output's package identity, reported under the output's own path.
fn check_output_package(output: &Mapping, index: usize, findings: &mut Vec<Diagnostic>) {
    let name = literal_at(output, &["package", "name"]).unwrap_or_default();
    if name.trim().is_empty() {
        findings.push(
            Diagnostic::new(
                Severity::Error,
                "recipe.output.name.missing",
                "an output declares no package.name".to_owned(),
            )
            .with_path(format!("outputs[{index}].package.name")),
        );
    }
    if let Some(declared) = literal_at(output, &["package", "version"])
        && !declared.contains("${{")
        && declared.parse::<Version>().is_err()
    {
        findings.push(
            Diagnostic::new(
                Severity::Error,
                "recipe.output.version.invalid",
                format!("output version '{declared}' is not a Conda version"),
            )
            .with_path(format!("outputs[{index}].package.version")),
        );
    }
}

fn check_literal_version(root: &Mapping, path: &str, findings: &mut Vec<Diagnostic>) {
    if let Some(declared) = literal_at(root, &["package", "version"])
        && !declared.contains("${{")
        && declared.parse::<Version>().is_err()
    {
        findings.push(
            Diagnostic::new(
                Severity::Error,
                "recipe.version.invalid",
                format!("package.version '{declared}' is not a Conda version"),
            )
            .with_path(path),
        );
    }
}

fn check_build(root: &Mapping, findings: &mut Vec<Diagnostic>) {
    let Some(build) = root
        .get(Value::String("build".to_owned()))
        .and_then(Value::as_mapping)
    else {
        return;
    };
    let Some(number) = build.get(Value::String("number".to_owned())) else {
        return;
    };
    match number.as_i64() {
        Some(value) if value >= 0 => {}
        Some(value) => findings.push(
            Diagnostic::new(
                Severity::Error,
                "recipe.build.number.negative",
                format!("build.number is {value}; a negative number sorts below the artifact it replaces"),
            )
            .with_path("build.number"),
        ),
        None => findings.push(
            Diagnostic::new(
                Severity::Error,
                "recipe.build.number.invalid",
                "build.number is not an integer".to_owned(),
            )
            .with_path("build.number"),
        ),
    }
}

/// Every source has to pin the bytes it fetches.
fn check_sources(root: &Mapping, rules: RecipeRules, findings: &mut Vec<Diagnostic>) {
    let Some(sources) = root.get(Value::String("source".to_owned())) else {
        return;
    };
    let entries: Vec<(String, &Value)> = match sources {
        Value::Sequence(sequence) => sequence
            .iter()
            .enumerate()
            .map(|(index, entry)| (format!("source[{index}]"), entry))
            .collect(),
        mapping @ Value::Mapping(_) => vec![("source".to_owned(), mapping)],
        _ => {
            findings.push(
                Diagnostic::new(
                    Severity::Error,
                    "recipe.source.invalid",
                    "source is neither a mapping nor a list of mappings".to_owned(),
                )
                .with_path("source"),
            );
            return;
        }
    };

    for (path, entry) in entries {
        let Some(mapping) = entry.as_mapping() else {
            findings.push(
                Diagnostic::new(
                    Severity::Error,
                    "recipe.source.invalid",
                    "a source entry is not a mapping".to_owned(),
                )
                .with_path(path),
            );
            continue;
        };

        if mapping.contains_key(Value::String("url".to_owned())) {
            check_digest(mapping, &path, findings);
        }
        if mapping.contains_key(Value::String("git".to_owned())) {
            check_git(mapping, &path, rules, findings);
        }
        // A `path` source is this repository's own working tree: there is nothing to pin.
    }
}

fn check_digest(mapping: &Mapping, path: &str, findings: &mut Vec<Diagnostic>) {
    let digest = mapping
        .get(Value::String("sha256".to_owned()))
        .and_then(Value::as_str)
        .map(str::trim);
    match digest {
        None => findings.push(
            Diagnostic::new(
                Severity::Error,
                "recipe.source.sha256.missing",
                "a url source declares no sha256, so the bytes are not pinned".to_owned(),
            )
            .with_path(format!("{path}.sha256")),
        ),
        Some(value) if !is_sha256(value) => findings.push(
            Diagnostic::new(
                Severity::Error,
                "recipe.source.sha256.invalid",
                format!("sha256 '{value}' is not 64 lowercase hexadecimal characters"),
            )
            .with_path(format!("{path}.sha256")),
        ),
        Some(_) => {}
    }
}

fn check_git(mapping: &Mapping, path: &str, rules: RecipeRules, findings: &mut Vec<Diagnostic>) {
    if rules.allow_mutable_git {
        return;
    }
    let rev = mapping
        .get(Value::String("rev".to_owned()))
        .and_then(Value::as_str)
        .map(str::trim);
    match rev {
        Some(value) if is_commit(value) => {}
        Some(value) => findings.push(
            Diagnostic::new(
                Severity::Error,
                "recipe.source.git.mutable",
                format!("git rev '{value}' is not a 40-character commit"),
            )
            .with_path(format!("{path}.rev")),
        ),
        None => findings.push(
            Diagnostic::new(
                Severity::Error,
                "recipe.source.git.mutable",
                "a git source declares no rev, so it builds whatever the branch points at"
                    .to_owned(),
            )
            .with_path(format!("{path}.rev")),
        ),
    }
}

fn check_about(
    recipe_directory: &Path,
    root: &Mapping,
    rules: RecipeRules,
    findings: &mut Vec<Diagnostic>,
) {
    let Some(about) = root
        .get(Value::String("about".to_owned()))
        .and_then(Value::as_mapping)
    else {
        findings.push(
            Diagnostic::new(
                Severity::Error,
                "recipe.about.missing",
                "the recipe has no about section, so the package carries no licence".to_owned(),
            )
            .with_path("about"),
        );
        return;
    };

    if rules.require_valid_spdx {
        match literal(about, "license") {
            None => findings.push(
                Diagnostic::new(
                    Severity::Error,
                    "recipe.license.missing",
                    "about.license is absent".to_owned(),
                )
                .with_path("about.license"),
            ),
            Some(value) => {
                if let Err(error) = value.parse::<spdx::Expression>() {
                    findings.push(
                        Diagnostic::new(
                            Severity::Error,
                            "recipe.license.invalid",
                            format!("'{value}' is not an SPDX expression: {error}"),
                        )
                        .with_path("about.license"),
                    );
                }
            }
        }
    }

    if !rules.enforce_license_file {
        return;
    }
    let Some(declared) = literal(about, "license_file") else {
        findings.push(
            Diagnostic::new(
                Severity::Error,
                "recipe.license_file.missing",
                "about.license_file is absent, so the licence text ships nowhere".to_owned(),
            )
            .with_path("about.license_file"),
        );
        return;
    };
    // The licence text may sit beside the recipe or inside one of the recipe's own local
    // sources; `rattler-build` copies a path source into the work directory before it reads
    // `license_file`, so a file that is reachable that way is reachable for the build too.
    let reachable = licence_roots(recipe_directory, root)
        .iter()
        .any(|root| root.join(&declared).is_file());
    if !reachable {
        findings.push(
            Diagnostic::new(
                Severity::Error,
                "recipe.license_file.absent",
                format!(
                    "about.license_file names '{declared}', which is not beside the recipe or in any of its local sources"
                ),
            )
            .with_path("about.license_file"),
        );
    }
}

/// Where a recipe can legitimately find a file it names: its own directory, and every local
/// `path` source it declares.
fn licence_roots(recipe_directory: &Path, root: &Mapping) -> Vec<std::path::PathBuf> {
    let mut roots = vec![recipe_directory.to_path_buf()];
    let sources = match root.get(Value::String("source".to_owned())) {
        Some(Value::Sequence(sequence)) => sequence.iter().collect::<Vec<_>>(),
        Some(other) => vec![other],
        None => Vec::new(),
    };
    for source in sources {
        if let Some(path) = source
            .as_mapping()
            .and_then(|mapping| literal(mapping, "path"))
        {
            roots.push(recipe_directory.join(path));
        }
    }
    roots
}

fn literal(mapping: &Mapping, key: &str) -> Option<String> {
    mapping
        .get(Value::String(key.to_owned()))
        .and_then(Value::as_str)
        .map(str::trim)
        .map(str::to_owned)
}

fn literal_at(root: &Mapping, path: &[&str]) -> Option<String> {
    let mut current = root;
    for key in &path[..path.len() - 1] {
        current = current
            .get(Value::String((*key).to_owned()))?
            .as_mapping()?;
    }
    literal(current, path[path.len() - 1])
}

fn is_sha256(text: &str) -> bool {
    text.len() == 64
        && text
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn is_commit(text: &str) -> bool {
    text.len() == 40
        && text
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}
