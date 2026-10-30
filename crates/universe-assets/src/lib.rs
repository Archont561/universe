//! Compile-time embedded static portal assets for Universe.
//!
//! The committed placeholder keeps a fresh checkout buildable until the static portal has
//! been generated. A generated `packages/portal/build` directory takes precedence in build.rs.

use include_dir::{Dir, include_dir};

/// Crate version synchronized with the package `VERSIONS` file.
pub const VERSION: &str = "0.1.0";

static PORTAL_DIST: Dir<'_> = include_dir!("$OUT_DIR/portal-dist");

/// Return the embedded static portal directory tree.
#[must_use]
pub fn portal_assets() -> &'static Dir<'static> {
    &PORTAL_DIST
}

#[cfg(test)]
mod tests {
    use super::{VERSION, portal_assets};

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
}
