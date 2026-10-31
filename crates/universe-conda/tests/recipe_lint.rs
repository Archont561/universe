//! Recipe lint, one fixture per rule.
//!
//! The fixtures are named for the defect they carry, so a failure names the rule that broke
//! as well as the file. Every case asserts the *code*, not the message: the codes are what a
//! CI annotation and a future `--json` consumer key on, and a reworded message must not
//! silently change the contract.

use std::path::{Path, PathBuf};

use universe_conda::{RecipeRules, lint_recipe};
use universe_core::diagnostic::Diagnostic;

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/recipes")
}

fn lint(name: &str, rules: RecipeRules) -> Vec<Diagnostic> {
    let directory = fixtures();
    let text = std::fs::read_to_string(directory.join(name))
        .unwrap_or_else(|error| panic!("cannot read fixture {name}: {error}"));
    lint_recipe(&directory, &text, rules)
}

fn codes(findings: &[Diagnostic]) -> Vec<&str> {
    findings.iter().map(Diagnostic::code).collect()
}

#[test]
fn a_recipe_that_pins_everything_passes() {
    assert_eq!(lint("pinned.yaml", RecipeRules::default()), Vec::new());
}

#[test]
fn a_url_source_without_a_digest_is_refused() {
    let findings = lint("unpinned-url.yaml", RecipeRules::default());
    assert_eq!(codes(&findings), ["recipe.source.sha256.missing"]);
    assert_eq!(findings[0].path(), Some("source[0].sha256"));
}

#[test]
fn a_git_source_pinned_to_a_branch_is_refused_without_an_override() {
    let findings = lint("mutable-git.yaml", RecipeRules::default());
    assert_eq!(codes(&findings), ["recipe.source.git.mutable"]);

    let permissive = RecipeRules {
        allow_mutable_git: true,
        ..RecipeRules::default()
    };
    assert!(lint("mutable-git.yaml", permissive).is_empty());
}

#[test]
fn an_invalid_spdx_expression_is_refused() {
    let findings = lint("bad-license.yaml", RecipeRules::default());
    assert_eq!(codes(&findings), ["recipe.license.invalid"]);
}

#[test]
fn a_negative_build_number_is_refused() {
    let findings = lint("negative-build-number.yaml", RecipeRules::default());
    assert_eq!(codes(&findings), ["recipe.build.number.negative"]);
}

#[test]
fn a_version_that_is_not_a_conda_version_is_refused() {
    let findings = lint("bad-version.yaml", RecipeRules::default());
    assert_eq!(codes(&findings), ["recipe.version.invalid"]);
}

#[test]
fn a_license_file_that_is_not_there_is_refused_by_name() {
    let findings = lint("missing-license-file.yaml", RecipeRules::default());
    assert_eq!(codes(&findings), ["recipe.license_file.absent"]);
    assert!(findings[0].message().contains("LICENCE.md"));

    let permissive = RecipeRules {
        enforce_license_file: false,
        ..RecipeRules::default()
    };
    assert!(lint("missing-license-file.yaml", permissive).is_empty());
}

#[test]
fn every_output_of_a_multi_output_recipe_names_its_package() {
    let findings = lint("multi-output.yaml", RecipeRules::default());
    assert_eq!(codes(&findings), ["recipe.output.name.missing"]);
    assert_eq!(findings[0].path(), Some("outputs[1].package.name"));
}

#[test]
fn text_that_is_not_yaml_is_one_finding_rather_than_a_panic() {
    let findings = lint_recipe(
        &fixtures(),
        "\t this: [is not: yaml",
        RecipeRules::default(),
    );
    assert_eq!(codes(&findings), ["recipe.parse.failed"]);
}

#[test]
fn a_recipe_with_no_about_section_is_refused() {
    let text = "context:\n  version: \"1.0.0\"\npackage:\n  name: example\n";
    let findings = lint_recipe(&fixtures(), text, RecipeRules::default());
    assert!(codes(&findings).contains(&"recipe.about.missing"));
}
