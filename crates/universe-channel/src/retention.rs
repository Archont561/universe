//! How many builds of one version a snapshot keeps.
//!
//! A branch channel is a single tree, so retention here is not garbage collection: nothing is
//! deleted from history, and nothing shrinks on the server. What retention decides is which
//! fragments take part in the snapshot that replaces the branch, which bounds what the channel
//! *serves*. The publisher says so in its own output rather than implying it reclaimed space,
//! because a force-push followed by a server-side `gc` is the only thing that would, and this
//! command does not run one.
//!
//! The rule is deliberately small and total: group by `(package, version)`, order by build
//! number, keep the highest `keep`. Every input fragment ends up either kept or pruned, and the
//! order of the input cannot change which is which — a snapshot that depends on the order of a
//! directory listing is a snapshot two machines cannot agree on.

use crate::fragment::Fragment;

/// Why a fragment set could not be retained.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum RetentionError {
    /// Keeping nothing would empty the channel.
    #[error(
        "retention keeps 0 builds of each version, which would leave the channel with no artifact at all"
    )]
    Zero,

    /// A build that retention would drop is the only one satisfying a surviving fragment's pin.
    #[error(
        "'{artifact}' is the build '{pinned_by}' pins exactly, so pruning it would publish a \
         channel that cannot satisfy its own metadata; raise --keep, or widen that dependency"
    )]
    Pinned {
        /// The artifact retention would prune.
        artifact: String,
        /// The surviving fragment whose dependency names it.
        pinned_by: String,
    },
}

/// What retention decided, in the order it decided it.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Retained {
    /// Fragments the snapshot keeps.
    pub kept: Vec<Fragment>,
    /// Fragments the snapshot leaves out.
    pub pruned: Vec<Fragment>,
}

impl Retained {
    /// Whether retention changed anything.
    #[must_use]
    pub fn is_unchanged(&self) -> bool {
        self.pruned.is_empty()
    }

    /// The pruned artifacts' channel paths, sorted, for a message that names them.
    #[must_use]
    pub fn pruned_paths(&self) -> Vec<String> {
        let mut paths: Vec<String> = self.pruned.iter().map(Fragment::channel_path).collect();
        paths.sort();
        paths
    }
}

/// Keep the highest `keep` builds of each `(package, version)`.
///
/// # Errors
///
/// Returns [`RetentionError::Zero`] when asked to keep nothing, and
/// [`RetentionError::Pinned`] when the build it would drop is the one a surviving fragment
/// pins by name, version and build — the one case where pruning would make the published
/// metadata unanswerable.
pub fn prune(fragments: &[Fragment], keep: usize) -> Result<Retained, RetentionError> {
    if keep == 0 {
        return Err(RetentionError::Zero);
    }

    let mut grouped: std::collections::BTreeMap<(String, String), Vec<&Fragment>> =
        std::collections::BTreeMap::new();
    for fragment in fragments {
        grouped
            .entry((fragment.package.clone(), fragment.version.clone()))
            .or_default()
            .push(fragment);
    }

    let mut retained = Retained::default();
    for (_, mut builds) in grouped {
        // Highest build first; the build string breaks a tie so two artifacts that share a
        // number order the same way on every machine.
        builds.sort_by(|left, right| {
            right
                .build_number
                .cmp(&left.build_number)
                .then_with(|| right.build.cmp(&left.build))
                .then_with(|| left.channel_path().cmp(&right.channel_path()))
        });
        let (keep_now, drop_now) = builds.split_at(builds.len().min(keep));
        retained
            .kept
            .extend(keep_now.iter().map(|fragment| (*fragment).clone()));
        retained
            .pruned
            .extend(drop_now.iter().map(|fragment| (*fragment).clone()));
    }

    // A pin that nothing else satisfies is a channel that contradicts itself, so it is refused
    // rather than published and discovered by a solver.
    for doomed in &retained.pruned {
        if let Some(pinner) = retained
            .kept
            .iter()
            .find(|kept| kept.depends.iter().any(|dep| pins_build(dep, doomed)))
        {
            return Err(RetentionError::Pinned {
                artifact: doomed.channel_path(),
                pinned_by: format!("{} {} {}", pinner.package, pinner.version, pinner.build),
            });
        }
    }

    retained.kept.sort_by_key(Fragment::channel_path);
    retained.pruned.sort_by_key(Fragment::channel_path);
    Ok(retained)
}

/// Whether one Conda dependency specifier pins `fragment` by name, version *and* build.
///
/// Two spellings pin a build — `name ==version=build` and the older `name version build` — and
/// both are read here. A range (`lib >=1.0`) or a version with no build (`lib 1.0.0`) is
/// satisfied by whichever build survives, so pruning against it is exactly what retention is
/// for; treating those as pins would make retention unusable on any channel whose packages
/// depend on each other at all.
fn exact_pin(dep: &str) -> Option<(&str, &str, &str)> {
    let parts: Vec<&str> = dep.split_whitespace().collect();
    match parts.as_slice() {
        [name, specifier] => {
            let (version, build) = specifier.strip_prefix("==")?.split_once('=')?;
            Some((name, version, build))
        }
        [name, version, build] => Some((name, version, build)),
        _ => None,
    }
}

fn pins_build(dep: &str, fragment: &Fragment) -> bool {
    exact_pin(dep).is_some_and(|(name, version, build)| {
        name == fragment.package && version == fragment.version && build == fragment.build
    })
}

#[cfg(test)]
mod tests {
    use super::{RetentionError, prune};
    use crate::fragment::Fragment;

    fn fragment(package: &str, version: &str, build_number: u64) -> Fragment {
        Fragment {
            package: package.to_owned(),
            version: version.to_owned(),
            build: format!("h{build_number:07}"),
            build_number,
            subdir: "linux-64".to_owned(),
            digest: format!("{build_number:064x}"),
            size_bytes: 10,
            depends: Vec::new(),
            license: Some("MIT".to_owned()),
            license_files: Vec::new(),
            path: format!("output/{package}-{version}-{build_number}.conda"),
        }
    }

    fn paths(fragments: &[Fragment]) -> Vec<String> {
        fragments.iter().map(Fragment::channel_path).collect()
    }

    #[test]
    fn the_highest_build_numbers_of_each_version_survive() {
        let fragments = [
            fragment("a", "1.0.0", 1),
            fragment("a", "1.0.0", 3),
            fragment("a", "1.0.0", 2),
        ];
        let retained = prune(&fragments, 2).expect("prunes");
        // Kept is ordered by channel path, so a snapshot's file list does not depend on the
        // order retention happened to visit the groups in.
        assert_eq!(
            paths(&retained.kept),
            [
                "linux-64/a-1.0.0-h0000002.conda",
                "linux-64/a-1.0.0-h0000003.conda"
            ]
        );
        assert_eq!(paths(&retained.pruned), ["linux-64/a-1.0.0-h0000001.conda"]);
    }

    #[test]
    fn versions_are_counted_apart_so_an_old_version_is_not_starved_by_a_new_one() {
        let fragments = [
            fragment("a", "1.0.0", 0),
            fragment("a", "2.0.0", 0),
            fragment("a", "2.0.0", 1),
        ];
        let retained = prune(&fragments, 1).expect("prunes");
        assert_eq!(
            paths(&retained.kept),
            [
                "linux-64/a-1.0.0-h0000000.conda",
                "linux-64/a-2.0.0-h0000001.conda"
            ]
        );
    }

    #[test]
    fn the_input_order_cannot_change_what_survives() {
        let mut fragments = vec![
            fragment("a", "1.0.0", 0),
            fragment("a", "1.0.0", 1),
            fragment("a", "1.0.0", 2),
        ];
        let once = prune(&fragments, 2).expect("prunes");
        fragments.reverse();
        let again = prune(&fragments, 2).expect("prunes");
        assert_eq!(once, again);
    }

    #[test]
    fn keeping_nothing_is_refused_rather_than_emptying_the_channel() {
        assert_eq!(
            prune(&[fragment("a", "1.0.0", 0)], 0),
            Err(RetentionError::Zero)
        );
    }

    #[test]
    fn a_build_a_surviving_package_pins_exactly_is_not_pruned() {
        let lib_old = {
            let mut fragment = fragment("lib", "1.0.0", 0);
            fragment.build = "h0000000".to_owned();
            fragment
        };
        let lib_new = {
            let mut fragment = fragment("lib", "1.0.0", 1);
            fragment.build = "h0000001".to_owned();
            fragment
        };
        let mut consumer = fragment("consumer", "1.0.0", 0);
        consumer.depends = vec!["lib ==1.0.0=h0000000".to_owned()];

        let error = prune(&[lib_old, lib_new, consumer], 1).expect_err("refused");
        assert_eq!(
            error,
            RetentionError::Pinned {
                artifact: "linux-64/lib-1.0.0-h0000000.conda".to_owned(),
                pinned_by: "consumer 1.0.0 h0000000".to_owned(),
            }
        );
        assert!(
            error
                .to_string()
                .contains("cannot satisfy its own metadata")
        );
    }

    #[test]
    fn a_range_is_not_a_pin_because_any_surviving_build_satisfies_it() {
        let lib_old = {
            let mut fragment = fragment("lib", "1.0.0", 0);
            fragment.build = "h0000000".to_owned();
            fragment
        };
        let lib_new = {
            let mut fragment = fragment("lib", "1.0.0", 1);
            fragment.build = "h0000001".to_owned();
            fragment
        };
        let mut consumer = fragment("consumer", "1.0.0", 0);
        consumer.depends = vec!["lib >=1.0".to_owned(), "lib 1.0.0".to_owned()];

        let retained = prune(&[lib_old, lib_new, consumer], 1).expect("prunes");
        assert!(
            retained
                .pruned
                .iter()
                .any(|fragment| fragment.package == "lib")
        );
    }

    #[test]
    fn the_older_name_version_build_spelling_is_a_pin_too() {
        let lib_old = {
            let mut fragment = fragment("lib", "1.0.0", 0);
            fragment.build = "h0000000".to_owned();
            fragment
        };
        let lib_new = {
            let mut fragment = fragment("lib", "1.0.0", 1);
            fragment.build = "h0000001".to_owned();
            fragment
        };
        let mut consumer = fragment("consumer", "1.0.0", 0);
        consumer.depends = vec!["lib 1.0.0 h0000000".to_owned()];

        assert!(matches!(
            prune(&[lib_old, lib_new, consumer], 1),
            Err(RetentionError::Pinned { .. })
        ));
    }
}
