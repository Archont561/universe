---
id: DECISION-137
title: Every workspace unit puts its Gherkin specification in features/ at the unit root
status: accepted
created_date: '2026-11-08'
updated_date: '2026-11-08'
---

## Context

`universe-kit` and `universe-workflow` were the only units with behaviour tests, and both put
them under `tests/features/`. That put the specification inside the harness that runs it: a
reader who wanted to know what the unit promised had to know how the tests were wired, and a
`.feature` file sat one level deeper than every other spec-shaped document in the repository.

Deciding this once, for every unit, is what makes the alint rule expressible at all — a rule
that has to accept two layouts enforces neither.

## Decision

Every unit under `crates/` or `packages/` that declares itself with a manifest carries three
directories: `src/`, `tests/`, and `features/` — **`features/` at the unit's root**.

The consequences, stated so they are not rediscovered later:

- `universe-kit` and `universe-workflow` moved from `tests/features/` to `features/`. The
  `#[scenario(path = …)]` attribute is relative to the crate root, so the move rewrote every
  path in the two harnesses; there is no second location a feature file can be read from.
- `scripts/gherkin_lint.py` discovers any directory named `features`, so the move took no edit
  to it and `pixi run lint-gherkin` covers the new documents as soon as they exist.
- Bun packages state behaviour the same way, with `@aboviq/bun-test-cucumber`: `bunfig.toml`
  preloads a plugin from `tests/`, `tests/features.test.ts` loads `features/**/*.feature`, and
  step definitions live in `tests/*.steps.ts`. The specification stays in `features/`; only
  the harness is under `tests/`.
- `.alint.yml` enforces the shape (`workspace-unit-layout` and three `dir_contains` rules),
  each rule is covered by a policy test under `alint-tests/behaviour-layout/`, and
  `AGENTS.md` house rule 12 states it for agents.

## Alternatives

- **`<unit>/tests/features/`.** Rejected by the same reasoning, and it is the layout this
  decision reverses: the specification is not a test asset, it is what the tests assert.
- **A repository-level `features/` tree.** Rejected: it separates a unit's specification from
  the unit, and it gives every crate's scenarios one namespace to collide in.
- **In-line `#[cfg(test)]` scenarios only, with no `.feature` files.** Rejected: a scenario
  written in Rust is a test with a comment, and the cucumber-expression grammar is what makes
  a scenario readable by someone who does not read Rust.
