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
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Cycle detection refuses circular intra-universe recipes (`A -> B -> A`) with a clear cycle path diagnostic.
- [ ] #2 `proptest` proves that for arbitrary random DAGs of up to 50 packages, every edge `(u -> v)` satisfies `level(u) < level(v)` and output ordering is 100% deterministic regardless of filesystem discovery order.
- [ ] #3 `wiremock` + `rstest-bdd` scenarios verify `--changed-or-missing` skips already-published tuples and marks missing `(pkg, subdir)` builds as runnable.
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 Unit, `proptest`, and `rstest-bdd` (`plan.feature`) tests pass under `cargo nextest` with zero network access.
<!-- DOD:END -->
