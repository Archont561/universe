//! Deterministic manifest-to-workflow renderer for Universe.

// The renderer implementation lands separately. This crate exists so the
// dependency graph and `Cargo.lock` carry the rendering stack.

/// Version of this component, equal to the workspace version the whole artifact set is
/// released under.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(test)]
mod tests {
    use super::VERSION;

    // A prerelease attempt publishes `0.Y.Z-rc.N`, so only the numeric core is pinned.
    #[test]
    fn version_has_a_three_part_numeric_core() {
        assert_eq!(VERSION, env!("CARGO_PKG_VERSION"));
        let core = VERSION.split('-').next().unwrap_or_default();
        let parts: Vec<&str> = core.split('.').collect();
        assert_eq!(parts.len(), 3, "expected three version parts in {VERSION}");
        assert!(
            parts.iter().all(|part| part.parse::<u32>().is_ok()),
            "expected numeric version parts in {VERSION}"
        );
    }
}
