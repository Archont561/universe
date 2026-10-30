//! Library surface for the `universe` command facade.
//!
//! The binary in `src/main.rs` owns argument routing. This library exists so the facade's
//! version is a real export rather than a value private to `main`.

/// Facade version, tracked in the package `VERSIONS` file like every versioned package.
pub const VERSION: &str = "0.1.0";

#[cfg(test)]
mod tests {
    use super::VERSION;

    #[test]
    fn version_matches_the_package_manifest() {
        assert_eq!(VERSION, env!("CARGO_PKG_VERSION"));
    }

    #[test]
    fn version_matches_the_versions_file() {
        assert_eq!(VERSION, include_str!("../VERSIONS").trim());
    }
}
