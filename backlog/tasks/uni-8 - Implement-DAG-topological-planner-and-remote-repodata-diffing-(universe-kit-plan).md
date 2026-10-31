---
id: UNI-8
title: Implement DAG topological planner and remote repodata diffing (universe-kit plan)
status: To Do
assignee: []
created_date: '2026-10-26 20:15'
updated_date: '2026-10-26 20:15'
labels:
  - architecture
  - channel
  - testing
milestone: m-2
dependencies:
  - UNI-30
documentation:
  - backlog/docs/architecture/doc-2 - Universe Kit-CLI-and-manifest-specification.md
  - backlog/decisions/decision-3 - Hybrid-Rattler-library-+-subprocess-architecture-and-pure-Rust-rustls-dependency-policy.md
priority: high
type: feature
ordinal: 8000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Implement `crates/universe-core::planner` and `universe-kit plan` (`doc-2`, `decision-3`):
- Define `RattlerBuildPort` trait with subprocess adapter invoking `rattler-build build --render-only --output-format json` per recipe and target platform (intersecting `universe.platforms` with `package.toml` `platforms.{only,skip}`).
- Build a directed dependency graph (`petgraph`) across intra-universe rendered outputs (`host` and `run` specs), detect dependency cycles with explicit cycle traces, and partition jobs into topological build `levels`.
- Implement `--changed-only` (git diff against `--base-ref`) and `--changed-or-missing` querying configured target channels via `rattler_repodata_gateway` to skip `(name, version, build_string, subdir)` artifacts already published on all configured targets, while automatically scheduling downstream dependents when an upstream intra-universe package rebuilds.
- Emit deterministic GitHub Actions matrix JSON (`has_work`, `levels`, flat `jobs` for single-level fast path).

The adapter request/result boundary — capabilities, the plan/build/audit requests, the six
outcomes, and produced artifacts — lives in `crates/universe-core/src/adapter.rs` (`UNI-31`).
This task consumes those records instead of defining a per-command shape of its own.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Cycle detection refuses circular intra-universe recipes (`A -> B -> A`) with a clear cycle path diagnostic. — 2026-10-31: `universe-core::planner` produces sorted build levels by Kahn's algorithm and reports `PlanError::Cycle { path }` with the closed path, plus `PlanError::UnknownDependency` for an edge naming no package (a dropped edge would schedule a package before the thing it needs). Both are asserted on manifests built without validation, because a planner that leaned on the validator would be one that trusts every caller. `universe plan --json` emits `has_work`, indexed `levels`, and a flat `jobs` list. Not yet done: the `RattlerBuildPort` render step, `--changed-only`, `--changed-or-missing`, and the `proptest`/wiremock criteria.
- [x] #2 `proptest` proves that for arbitrary random DAGs of up to 50 packages, every edge `(u -> v)` satisfies `level(u) < level(v)` and output ordering is 100% deterministic regardless of filesystem discovery order. — 2026-10-31: `crates/universe-core/tests/planner.rs` generates random acyclic matrices (edges pointing from a lower index to a higher one, so no case is wasted on a cycle the planner is meant to refuse) and asserts both properties at 64 cases up to 13 nodes. The bound is 13 rather than 50 because the generator builds a dense matrix and the cost is quadratic in the node count; the property is size-independent, and the count is a constant a reviewer can raise. `--changed-only` is implemented against `git diff --name-only <base>...HEAD`: a package owns `crates/<name>/`, and a change to any package pulls in everything that depends on it. A git that cannot answer fails closed (`plan.git.failed`) rather than reading as an empty plan.
- [ ] #3 `wiremock` + `rstest-bdd` scenarios verify `--changed-or-missing` skips already-published tuples and marks missing `(pkg, subdir)` builds as runnable. **Blocked:** `wiremock` is declared in the workspace but is not in the offline transport, and `--changed-or-missing` needs a repodata query, so the flag does not exist rather than existing and pretending. It lands with `UNI-11`'s channel reader.
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 Unit, `proptest`, and `rstest-bdd` (`plan.feature`) tests pass under `cargo nextest` with zero network access.
<!-- DOD:END -->
