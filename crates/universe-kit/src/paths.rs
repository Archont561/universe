//! Declared-output collection.
//!
//! A manifest names its outputs as globs, because a recipe's build string is not known
//! before it builds. The matcher here is deliberately small — `*` within one path segment
//! and `**` across segments — rather than a general glob engine: the manifest schema already
//! refuses a pattern that escapes the workspace, so what is left to match is exactly these
//! two forms.

use std::path::{Path, PathBuf};

/// Whether `path` matches `pattern`, both relative and slash-separated.
#[must_use]
pub(crate) fn matches(pattern: &str, path: &str) -> bool {
    let pattern: Vec<&str> = pattern.split('/').collect();
    let path: Vec<&str> = path.split('/').collect();
    match_segments(&pattern, &path)
}

fn match_segments(pattern: &[&str], path: &[&str]) -> bool {
    match pattern.split_first() {
        None => path.is_empty(),
        // `**` consumes zero or more segments, so every split point has to be tried.
        Some((segment, rest)) if *segment == "**" => {
            (0..=path.len()).any(|consumed| match_segments(rest, &path[consumed..]))
        }
        Some((segment, rest)) => match path.split_first() {
            Some((first, tail)) => match_segment(segment, first) && match_segments(rest, tail),
            None => false,
        },
    }
}

fn match_segment(pattern: &str, text: &str) -> bool {
    match pattern.split_once('*') {
        None => pattern == text,
        Some((prefix, suffix)) => {
            text.len() >= prefix.len() + suffix.len()
                && text.starts_with(prefix)
                && text.ends_with(suffix)
        }
    }
}

/// The directory a pattern can reach into, so collection never walks the whole repository.
///
/// The longest wildcard-free prefix is the base: a pattern that starts with a wildcard falls
/// back to the root itself.
#[must_use]
pub(crate) fn base_directory(pattern: &str) -> PathBuf {
    let mut base = PathBuf::new();
    for segment in pattern.split('/') {
        if segment.contains('*') {
            break;
        }
        base.push(segment);
    }
    base
}

/// Every file under `root` that matches `pattern`, sorted so a run is deterministic.
#[must_use]
pub(crate) fn collect(root: &Path, pattern: &str) -> Vec<PathBuf> {
    let base = root.join(base_directory(pattern));
    let mut found = Vec::new();
    walk(&base, root, pattern, &mut found);
    found.sort();
    found
}

fn walk(directory: &Path, root: &Path, pattern: &str, found: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(directory) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let Ok(kind) = entry.file_type() else {
            continue;
        };
        if kind.is_dir() {
            walk(&path, root, pattern, found);
        } else if let Ok(relative) = path.strip_prefix(root)
            && matches(pattern, &relative.to_string_lossy())
        {
            found.push(path);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{base_directory, matches};

    #[test]
    fn a_star_matches_inside_one_segment_only() {
        assert!(matches("out/*.conda", "out/a.conda"));
        assert!(!matches("out/*.conda", "out/nested/a.conda"));
    }

    #[test]
    fn a_double_star_matches_any_depth() {
        assert!(matches("out/**/*.conda", "out/a.conda"));
        assert!(matches("out/**/*.conda", "out/x/y/a.conda"));
        assert!(!matches("out/**/*.conda", "other/a.conda"));
    }

    #[test]
    fn a_pattern_without_wildcards_matches_one_path() {
        assert!(matches("out/a.conda", "out/a.conda"));
        assert!(!matches("out/a.conda", "out/b.conda"));
    }

    #[test]
    fn the_base_directory_stops_at_the_first_wildcard() {
        assert_eq!(
            base_directory("output/pkg/linux-64/*.conda"),
            std::path::PathBuf::from("output/pkg/linux-64")
        );
        assert_eq!(base_directory("**/*.conda"), std::path::PathBuf::new());
    }
}
