//! Build order: which packages can build at once, and which must wait.
//!
//! The planner is deliberately the smallest thing that can be wrong loudly. It reads the
//! dependency edges the manifest already declares and produces levels, because a workflow
//! matrix needs a total order it can run without a second scheduler. Nothing here touches
//! the filesystem, the network, or a builder: those arrive as observed facts from `UNI-8`'s
//! `--changed-only` and `--changed-or-missing` queries, which are not part of this pass.
//!
//! Two decisions are load-bearing:
//!
//! - **A dependency that does not exist is refused.** Dropping the edge would schedule a
//!   package before the thing it needs, and the failure would surface as a build error
//!   somewhere unrelated.
//! - **Ordering never follows declaration order.** Levels and their members are sorted by
//!   name, so two identical manifests produce identical plans and an unrelated edit higher
//!   in the file cannot reshuffle a matrix.

use std::collections::{BTreeMap, BTreeSet};

use serde_json::{Value, json};

use crate::manifest::Manifest;

/// Why a manifest cannot be turned into a build order.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PlanError {
    /// A package depends on a name no package in the manifest carries.
    UnknownDependency {
        /// The package that declared the edge.
        dependent: String,
        /// The name it named.
        dependency: String,
    },
    /// The dependency edges contain a cycle, traced from one package back to itself.
    Cycle {
        /// The closed path, first element repeated last.
        path: Vec<String>,
    },
}

/// One package scheduled in the plan.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PlanJob {
    /// Package name, exactly as the manifest spells it.
    pub package: String,
    /// Platforms this package is built for, in project order.
    pub platforms: Vec<String>,
}

/// The build order, as levels of packages that may run concurrently.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct BuildPlan {
    /// Level `n` may start once every level below it has finished.
    pub levels: Vec<Vec<String>>,
    /// The same order, with each package's platforms attached.
    pub jobs: Vec<PlanJob>,
}

impl BuildPlan {
    /// Whether the plan has anything to build.
    #[must_use]
    pub fn has_work(&self) -> bool {
        !self.levels.is_empty()
    }

    /// The plan as the Actions matrix the workflow consumes.
    ///
    /// `levels` is indexed (a reader never has to count), and `jobs` is a flat fast path for
    /// a single-level plan, where a matrix of one dimension is just a list.
    #[must_use]
    pub fn to_json_value(&self) -> Value {
        let levels = self
            .levels
            .iter()
            .enumerate()
            .map(|(level, packages)| {
                let jobs = packages
                    .iter()
                    .filter_map(|package| self.jobs.iter().find(|job| &job.package == package))
                    .map(|job| json!({ "package": job.package, "platforms": job.platforms }))
                    .collect::<Vec<_>>();
                json!({ "level": level, "jobs": jobs })
            })
            .collect::<Vec<_>>();
        let flat = self
            .jobs
            .iter()
            .map(|job| json!({ "package": job.package, "platforms": job.platforms }))
            .collect::<Vec<_>>();
        json!({ "has_work": self.has_work(), "levels": levels, "jobs": flat })
    }
}

/// Order the manifest's packages into build levels.
///
/// Packages without a `command` are not scheduled: they are supplied from outside this
/// repository, so there is nothing to run. They remain valid dependency targets.
///
/// # Errors
///
/// Returns [`PlanError::UnknownDependency`] for an edge naming no package, and
/// [`PlanError::Cycle`] when the edges close a loop.
pub fn plan(manifest: &Manifest) -> Result<BuildPlan, PlanError> {
    let buildable: BTreeMap<&str, &crate::manifest::Package> = manifest
        .packages
        .iter()
        .filter(|package| !package.command.is_empty())
        .map(|package| (package.name.as_str(), package))
        .collect();
    let declared: BTreeSet<&str> = manifest
        .packages
        .iter()
        .map(|package| package.name.as_str())
        .collect();

    for package in &manifest.packages {
        for dependency in &package.depends_on {
            if !declared.contains(dependency.as_str()) {
                return Err(PlanError::UnknownDependency {
                    dependent: package.name.clone(),
                    dependency: dependency.clone(),
                });
            }
        }
    }

    let mut remaining: BTreeSet<String> = buildable.keys().map(|name| (*name).to_owned()).collect();
    let mut levels = Vec::new();

    while !remaining.is_empty() {
        // A package is ready when every one of its buildable dependencies already has a
        // level. Dependencies supplied from outside contribute no edge.
        let mut ready: Vec<String> = remaining
            .iter()
            .filter(|name| {
                buildable
                    .get(name.as_str())
                    .map(|package| &package.depends_on)
                    .into_iter()
                    .flatten()
                    .all(|dependency| !remaining.contains(dependency))
            })
            .cloned()
            .collect();

        if ready.is_empty() {
            return Err(PlanError::Cycle {
                path: cycle_path(&remaining, &buildable),
            });
        }
        ready.sort();
        for name in &ready {
            remaining.remove(name);
        }
        levels.push(ready);
    }

    let jobs = levels
        .iter()
        .flatten()
        .filter_map(|name| buildable.get(name.as_str()))
        .map(|package| PlanJob {
            package: package.name.clone(),
            platforms: manifest
                .platforms_for(package)
                .iter()
                .map(|platform| (*platform).to_owned())
                .collect(),
        })
        .collect();

    Ok(BuildPlan { levels, jobs })
}

/// Trace one cycle among the packages that never became ready.
///
/// Every remaining package has a dependency that is also remaining, so walking dependencies
/// from any of them must revisit a node; the first revisit closes the path.
fn cycle_path(
    remaining: &BTreeSet<String>,
    buildable: &BTreeMap<&str, &crate::manifest::Package>,
) -> Vec<String> {
    let Some(start) = remaining.iter().next() else {
        return Vec::new();
    };
    let mut path = Vec::new();
    let mut current = start.clone();
    loop {
        if let Some(position) = path.iter().position(|node| node == &current) {
            let mut cycle: Vec<String> = path[position..].to_vec();
            cycle.push(current);
            return cycle;
        }
        path.push(current.clone());
        let next = buildable
            .get(current.as_str())
            .and_then(|package| {
                package.depends_on.iter().find(|dependency| {
                    remaining.contains(*dependency) || path.contains(*dependency)
                })
            })
            .cloned();
        match next {
            Some(next) => current = next,
            None => return path,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::cycle_path;
    use std::collections::{BTreeMap, BTreeSet};

    #[test]
    fn an_empty_remainder_has_no_cycle_to_trace() {
        assert!(cycle_path(&BTreeSet::new(), &BTreeMap::new()).is_empty());
    }
}
