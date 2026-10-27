---
id: decision-15
title: Standalone toolchain and quality gates
date: '2026-10-27'
status: accepted
---

## Context

The standalone repository contains Rust crates, Markdown Backlog records,
agent skills, and small repository-maintenance checks. It has no browser
portal package, schema package, release lane, or TypeScript application yet.
The scaffold should therefore stay small and reproducible without carrying a
second JavaScript toolchain only to run repository glue.

## Decision

- Pixi is the only human/CI entrypoint and owns the locked tool environment.
- Rust product code lives in `crates/`; Rust tests use each crate's external
  `tests/` directory where practical.
- Repository checks are Python modules under `scripts/`, tested by pytest
  under `tests/scripts/`, formatted/linted by Ruff, and type-checked by mypy.
- There is no root `tsconfig.json`, root `package.json`, Biome config, Bun
  lockfile, or TypeScript repository scripting.
- Future TypeScript product packages may be introduced only as explicit
  packages with their own Backlog decision, task wiring in Pixi, and tests;
  they must not resurrect an implicit root JavaScript workspace for glue.
- `pixi run gates` is non-negotiable and covers formatting, lint, typecheck,
  Rust and Python tests, Rust/Python coverage, Backlog/repo/skills
  validation, workflow lint, dependency audit, and a full build.
- Rust line coverage must be at least 90%; Python repository scripts emit
  LCOV and have their own coverage floor in the Pixi task.

## Consequences

Decision-2 is superseded for repository automation. Local development, hooks,
and CI all enter through the same named Pixi tasks, and the current tree can
be checked without installing host Rust, Python, Node, or Bun toolchains.
