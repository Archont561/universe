//! Embedded, compile-time assets: the portal bundle and the workflow templates.
//!
//! Both are embedded rather than read from disk, so rendering a workflow needs no
//! checkout of a template directory and no network. A template that only exists in the
//! repository would make every consumer's render depend on a path that is not theirs.

use include_dir::{Dir, include_dir};

/// Crate version synchronized with the package `VERSIONS` file.
pub const VERSION: &str = "0.1.0";

static PORTAL_DIST: Dir<'_> = include_dir!("$OUT_DIR/portal-dist");

/// Return the embedded static portal directory tree.
#[must_use]
pub fn portal_assets() -> &'static Dir<'static> {
    &PORTAL_DIST
}

/// The workflow file template every generated file is rendered from.
pub const WORKFLOW_TEMPLATE: &str = include_str!("../templates/workflow.yml.j2");

/// Actions this repository has already pinned, by the ref a template needs.
///
/// A generated workflow that used a mutable tag would fail the repository's own
/// `gha-pin-actions-to-sha` rule, so the pin is part of the template's input rather than
/// something a renderer guesses at.
pub const CHECKOUT: &str = "actions/checkout@3d3c42e5aac5ba805825da76410c181273ba90b1 # v7.0.1";

/// `prefix-dev/setup-pixi`, pinned.
pub const SETUP_PIXI: &str =
    "prefix-dev/setup-pixi@ad1da6449d9e506fd127fd67ab2d054a769f6a94 # v0.11.2";

/// `actions/upload-artifact`, pinned.
///
/// A build that produces an artifact on one runner and audits it on another needs the bytes
/// to travel between them; Runner filesystems are not shared.
/// Pinned to the version the rest of this repository's workflows already use.
pub const UPLOAD_ARTIFACT: &str =
    "actions/upload-artifact@cf430e030ddbb5b0abf93d22962f4752f3646cd9 # v7.0.2";

/// `actions/download-artifact`, pinned.
pub const DOWNLOAD_ARTIFACT: &str =
    "actions/download-artifact@9000827ccba6bdab643e8b6fd33ac0654aef8333 # v8.0.2";

#[cfg(test)]
mod tests {
    use super::{
        CHECKOUT, DOWNLOAD_ARTIFACT, SETUP_PIXI, UPLOAD_ARTIFACT, VERSION, WORKFLOW_TEMPLATE,
        portal_assets,
    };

    #[test]
    fn version_matches_the_package_manifest() {
        assert_eq!(VERSION, env!("CARGO_PKG_VERSION"));
    }

    #[test]
    fn version_matches_the_versions_file() {
        assert_eq!(VERSION, include_str!("../VERSIONS").trim());
    }

    #[test]
    fn a_fresh_build_embeds_an_index_page() {
        assert!(portal_assets().get_file("index.html").is_some());
    }

    // The template uses `<<`/`>>` for its own variables because GitHub expressions are
    // `${{ }}`: a template in the default syntax could not contain a workflow expression
    // at all. So every `{{` here must belong to a GitHub expression.
    #[test]
    fn the_template_uses_only_github_expressions_in_double_braces() {
        assert!(WORKFLOW_TEMPLATE.contains("<< name >>"));
        for (index, _) in WORKFLOW_TEMPLATE.match_indices("{{") {
            assert!(
                WORKFLOW_TEMPLATE[..index].ends_with('$'),
                "a bare {{{{ at byte {index} would be parsed as a template variable"
            );
        }
        assert!(WORKFLOW_TEMPLATE.contains("${{ github.ref }}"));
    }

    #[test]
    fn action_pins_are_commit_shas() {
        for action in [CHECKOUT, SETUP_PIXI, UPLOAD_ARTIFACT, DOWNLOAD_ARTIFACT] {
            let reference = action.split('@').nth(1).expect("ref");
            let sha = reference.split_whitespace().next().expect("sha");
            assert_eq!(sha.len(), 40, "{action} is not pinned to a commit");
            assert!(sha.bytes().all(|byte| byte.is_ascii_hexdigit()), "{action}");
        }
    }
}
