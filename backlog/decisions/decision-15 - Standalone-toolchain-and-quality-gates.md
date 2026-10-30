---
id: decision-15
title: Standalone toolchain and quality gates
date: '2026-10-27'
status: accepted
---

## Context

The standalone repository contains Rust crates, Markdown Backlog records,
agent skills, and Python repository-maintenance checks. A product-facing Bun
workspace is now explicitly authorized for the shared configuration, schema,
and static portal packages; that toolchain must not replace Python for repo
glue or weaken the ecosystem-neutral core boundary.

## Decision

- Pixi is the only human/CI entrypoint and owns the locked tool environment.
- Rust product code lives in `crates/`; Rust tests use each crate's external
  `tests/` directory where practical.
- Repository checks are Python modules under `scripts/`, tested by pytest
  under `TESTS/`, formatted/linted by Ruff, and type-checked by mypy.
- A root `package.json`, Bun lockfile, Biome/Turbo/TypeScript configuration, and
  `packages/{config,schema,portal}` are allowed only for product workspace packages.
  Every package has a `VERSIONS` file checked against its package manifest.
- Python remains the implementation language for repository scripts and tests; no
  TypeScript repo-glue migration is implied by the product workspace.
- Pixi remains the only outer task entrypoint. Its locked environment owns Bun as
  well as Rust/Python tools, and the package workspace is invoked through named
  Pixi tasks. `pixi run gates` covers the existing Rust/Python gates and the Bun
  product-package checks.
- Rust line coverage must be at least 90%; Python repository scripts emit
  LCOV and have their own coverage floor in the Pixi task.

## Consequences

Decision-2 remains superseded for repository automation. Local development, hooks,
and CI enter through the same named Pixi tasks. The `packages/*` and embedded-assets
scaffolds are in place; the M6 portal behavior and generated-site acceptance criteria
remain tracked by UNI-5/UNI-6 and are not complete.
