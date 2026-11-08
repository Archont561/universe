---
id: UNI-63
title: Give every crate and package a features directory and enforce the layout
status: Done
assignee: []
created_date: '2026-11-08'
updated_date: '2026-11-08'
labels:
  - tooling
  - bdd
  - gates
milestone: m-0
documentation:
  - backlog/decisions/decision-137 - Every-workspace-unit-puts-its-Gherkin-specification-in-features-at-the-unit-root.md
  - backlog/decisions/decision-133 - Python-tests-live-under-scripts-tests,-beside-the-automation-they-cover.md
priority: medium
type: chore
ordinal: 63000
---

## Description
<!-- SECTION:DESCRIPTION.BEGIN -->
Two crates had behaviour tests and eight units had none, so "does this unit have scenarios?"
was answered by looking. That is not a gate.

These three directories are now the shape of a workspace unit: `src/` for the implementation
and its unit tests, `tests/` for the harness that binds the steps, and `features/` for the
Gherkin documents — at the unit's root, not under `tests/`. `decision-137` records the layout
and the reversal it makes; `AGENTS.md` house rule 12 states it for agents.

`universe-kit` and `universe-workflow` moved from `tests/features/` to `features/`, which
rewrote the 22 `#[scenario(path = …)]` attributes that pointed at them. Every other crate
gained a `.feature` document and a `tests/*_bdd.rs` harness that runs it, `universe-assets` and
`universe-cli` gained a `tests/` directory they never had, and the three Bun packages gained a
`@aboviq/bun-test-cucumber` harness: `bunfig.toml`, a preload, a loader, and `tests/*.steps.ts`.

`universe-cli`'s routing rule moved from `src/main.rs` into `src/lib.rs` so the facade's one
decision is a behaviour a scenario can drive; `main.rs` now does nothing but hand its arguments
to it.
<!-- SECTION:DESCRIPTION.END -->

## Acceptance Criteria
<!-- AC.BEGIN -->
- [x] #1 Every crate under `crates/` and every package under `packages/` has `src/`, `tests/` and `features/`. — 2026-11-08: `crates/universe-assets/tests/` and `crates/universe-cli/tests/` created; `features/` created in the eight units that had none.
- [x] #2 `features/` sits at the unit root, and the two units that had `tests/features/` were moved with `git mv` and their scenario paths rewritten. — 2026-11-08: 22 `#[scenario(path = "tests/features/…")]` attributes now read `path = "features/…"`; `cargo test -p universe-workflow --test render_bdd` and `-p universe-kit` pass.
- [x] #3 Every unit's `features/` holds a `.feature` document and its `tests/` holds the harness that runs it. — 2026-11-08: 11 Rust crates drive `rstest-bdd`; `packages/{config,portal,schema}` run `@aboviq/bun-test-cucumber` over their own `features/`.
- [x] #4 The convention is a house rule in `AGENTS.md`. — 2026-11-08: house rule 12, with the two harness shapes and the commands that check them.
- [x] #5 `.alint.yml` enforces the layout repo-wide, and the rule passes on the whole repository. — 2026-11-08: `workspace-unit-layout` plus three `dir_contains` rules; `pixi run alint-check` reports 74 rules passing.
- [x] #6 The new rules are covered by versioned policy tests, so a rule that stops firing is caught. — 2026-11-08: five documents under `alint-tests/behaviour-layout/`; `pixi run alint-test` runs them.
- [x] #7 The new checks are gate members rather than commands run by hand. — 2026-11-08: `alint-test` is a Pixi task and a dependency of `check-fast`.
<!-- AC.END -->

## Definition of Done
<!-- DOD.BEGIN -->
- [x] #1 The Backlog record and the decision are written, and the decision is recorded as a reversal rather than a preference.
- [x] #2 `pixi run gates` passes. — 2026-11-08: exit 0; 572 Rust tests, 39 Bun tests, alint 74/74 plus 5 policy tests, 13 feature files clean.
<!-- DOD.END -->
