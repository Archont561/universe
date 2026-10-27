# Changelog

All notable changes to Universe are recorded here.

## Unreleased

- Created the standalone Universe monorepo scaffold with `universe-core`,
  `universe-kit`, `universe-airlock`, and `universe-airlock-bootstrap`.
- Added the Backlog design, decision, milestone, and task store.
- Added Pixi quality gates covering Rust and Python formatting, linting,
  type checking, tests, coverage, Backlog/repository/skills validation,
  workflow lint, dependency audit, and builds.
- Kept repository automation in Python; there is no root TypeScript
  configuration or TypeScript scripting surface.
