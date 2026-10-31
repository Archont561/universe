//! The planner decides build order, so its failures are the ones that reach a runner.
//!
//! Two properties are worth more than any feature here: a dependency that does not exist is
//! refused rather than dropped, because a silently dropped edge builds a package before the
//! thing it needs; and the output ordering does not depend on the order packages happen to
//! appear in the manifest, because a matrix that reshuffles between two identical manifests
//! is a diff nobody asked for.

use serde_json::json;
use universe_core::manifest::Manifest;
use universe_core::planner::{PlanError, plan};

/// A manifest that validates: these fixtures publish nowhere, so they owe no smoke command.
fn manifest(packages: &str) -> Manifest {
    let text = format!(
        "schema = 1\n\n[project]\nname = \"example\"\nplatforms = [\"linux-64\"]\n\n{packages}"
    );
    Manifest::from_toml(&text).expect("the fixture manifest parses")
}

/// A manifest built without validation, so the planner's own guards are what is under test.
/// Validation refuses these shapes first, and a planner that leaned on that would be a
/// planner that trusts every caller to have validated.
fn unvalidated(packages: serde_json::Value) -> Manifest {
    let text = json!({
        "schema": 1,
        "project": { "name": "example", "platforms": ["linux-64"] },
        "packages": packages,
    })
    .to_string();
    serde_json::from_str::<Manifest>(&text).expect("the fixture parses structurally")
}

const ONE: &str = r#"
[[packages]]
name = "one"
kind = "conda"
command = ["build", "one"]
release = false
outputs = ["out/one.conda"]
"#;

#[test]
fn a_single_package_is_one_level() {
    let plan = plan(&manifest(ONE)).expect("plans");
    assert_eq!(plan.levels, [vec!["one".to_owned()]]);
    assert!(plan.has_work());
}

#[test]
fn independent_packages_share_a_level() {
    let packages = r#"
[[packages]]
name = "b"
kind = "conda"
command = ["build", "b"]
release = false

[[packages]]
name = "a"
kind = "conda"
command = ["build", "a"]
release = false
"#;
    // Declared out of order on purpose: the plan must not inherit declaration order.
    assert_eq!(
        plan(&manifest(packages)).expect("plans").levels,
        [vec!["a".to_owned(), "b".to_owned()]]
    );
}

#[test]
fn a_dependent_package_lands_in_a_later_level() {
    let packages = r#"
[[packages]]
name = "app"
kind = "conda"
command = ["build", "app"]
release = false
depends_on = ["lib"]

[[packages]]
name = "lib"
kind = "conda"
command = ["build", "lib"]
release = false

[[packages]]
name = "tool"
kind = "conda"
command = ["build", "tool"]
release = false
depends_on = ["lib"]
"#;
    assert_eq!(
        plan(&manifest(packages)).expect("plans").levels,
        [
            vec!["lib".to_owned()],
            vec!["app".to_owned(), "tool".to_owned()]
        ]
    );
}

#[test]
fn a_cycle_is_refused_with_the_path_that_closed_it() {
    let manifest = unvalidated(json!([
        { "name": "a", "kind": "conda", "command": ["build", "a"], "depends_on": ["b"] },
        { "name": "b", "kind": "conda", "command": ["build", "b"], "depends_on": ["a"] },
    ]));
    match plan(&manifest) {
        Err(PlanError::Cycle { path }) => {
            assert_eq!(path.first(), path.last(), "the path returns to its start");
            assert_eq!(
                path.len(),
                3,
                "a two-package cycle names three nodes: {path:?}"
            );
        }
        other => panic!("expected a cycle, got {other:?}"),
    }
}

#[test]
fn a_dependency_that_does_not_exist_is_refused_by_name() {
    let manifest = unvalidated(json!([
        { "name": "app", "kind": "conda", "command": ["build", "app"], "depends_on": ["ghost"] },
    ]));
    assert_eq!(
        plan(&manifest),
        Err(PlanError::UnknownDependency {
            dependent: "app".to_owned(),
            dependency: "ghost".to_owned(),
        })
    );
}

#[test]
fn a_package_with_no_command_is_not_scheduled_but_still_satisfies_a_dependency() {
    // An externally supplied artifact has no build step of its own. Refusing it would make
    // every consumer of a vendor-provided package unplannable.
    let packages = r#"
[[packages]]
name = "app"
kind = "conda"
command = ["build", "app"]
release = false
depends_on = ["vendored"]

[[packages]]
name = "vendored"
kind = "conda"
release = false
"#;
    assert_eq!(
        plan(&manifest(packages)).expect("plans").levels,
        [vec!["app".to_owned()]]
    );
}

#[test]
fn a_manifest_with_nothing_to_build_reports_no_work() {
    let plan = plan(&manifest("")).expect("plans");
    assert!(plan.levels.is_empty());
    assert!(!plan.has_work());
}

#[test]
fn the_plan_serializes_as_levels_of_jobs() {
    let plan = plan(&manifest(ONE)).expect("plans");
    let json = serde_json::to_value(plan.to_json_value()).expect("serializes");
    assert_eq!(json["has_work"], true);
    assert_eq!(json["levels"][0]["level"], 0);
    assert_eq!(json["levels"][0]["jobs"][0]["package"], "one");
    assert_eq!(json["levels"][0]["jobs"][0]["platforms"][0], "linux-64");
}

// The property the matrix depends on: an edge always points forward in time. A planner that
// merely happens to order this repository's packages correctly is a planner that will break
// on someone else's; a random DAG is the cheapest way to stop trusting the happy path.
mod properties {
    use super::unvalidated;
    use proptest::prelude::*;
    use serde_json::json;
    use std::collections::BTreeSet;
    use universe_core::planner::plan;

    fn any_dag(bound: usize) -> impl Strategy<Value = Vec<Vec<bool>>> {
        // An adjacency matrix restricted to edges that point from a lower index to a higher
        // one: every such graph is acyclic by construction, so the generator cannot waste
        // cases on cycles the planner is expected to refuse.
        (0..=bound).prop_flat_map(|size| {
            proptest::collection::vec(any::<bool>(), size * size).prop_map(move |bits| {
                let mut matrix = vec![vec![false; size]; size];
                for (from, row) in matrix.iter_mut().enumerate().take(size) {
                    for (to, edge) in row.iter_mut().enumerate().skip(from + 1) {
                        *edge = bits[from * size + to];
                    }
                }
                matrix
            })
        })
    }

    fn manifest_for(matrix: &[Vec<bool>]) -> universe_core::manifest::Manifest {
        let packages: Vec<serde_json::Value> = matrix
            .iter()
            .enumerate()
            .map(|(from, row)| {
                let dependencies: Vec<String> = row
                    .iter()
                    .enumerate()
                    .filter(|(_, edge)| **edge)
                    .map(|(to, _)| format!("p{to}"))
                    .collect();
                json!({
                    "name": format!("p{from}"),
                    "kind": "conda",
                    "command": ["build"],
                    "depends_on": dependencies,
                    // Built without validation: the generator produces shapes the validator
                    // would refuse for reasons that have nothing to do with ordering.
                    "release": false,
                })
            })
            .collect();
        unvalidated(serde_json::Value::Array(packages))
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(64))]

        #[test]
        fn every_edge_points_forward_in_time(matrix in any_dag(12)) {
            let manifest = manifest_for(&matrix);
            let plan = plan(&manifest).expect("an acyclic graph always plans");
            let mut level_of = std::collections::BTreeMap::new();
            for (index, level) in plan.levels.iter().enumerate() {
                for package in level {
                    level_of.insert(package.clone(), index);
                }
            }
            for (from, row) in matrix.iter().enumerate() {
                for (to, edge) in row.iter().enumerate() {
                    if *edge {
                        let dependent = level_of[&format!("p{from}")];
                        let dependency = level_of[&format!("p{to}")];
                        prop_assert!(
                            dependency < dependent,
                            "p{} depends on p{} but sits at level {} against {}",
                            from, to, dependent, dependency
                        );
                    }
                }
            }
        }

        #[test]
        fn the_same_graph_always_plans_the_same_way(matrix in any_dag(12)) {
            let first = plan(&manifest_for(&matrix)).expect("plans");
            let second = plan(&manifest_for(&matrix)).expect("plans again");
            let total = first.levels.iter().flatten().count();
            prop_assert_eq!(first.clone(), second);
            let names: BTreeSet<String> = first.levels.iter().flatten().cloned().collect();
            prop_assert_eq!(names.len(), total);
        }
    }
}
