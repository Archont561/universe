---
id: doc-204
title: Testing and regression artifacts
type: specification
created_date: '2026-10-27'
updated_date: '2026-10-27'
---
# Testing and regression artifacts

## Test layout

- Rust integration tests live below each crate's `tests/` directory where
  practical; product behaviour is proven through public library surfaces and
  real binaries.
- Python tests for repository automation live in `tests/scripts/`.
- Tests never mutate the live repository or a developer's real home
  directory. Use fixtures, temporary directories, and isolated environment
  variables.
- Offline behaviour is tested with local fixtures and no accidental network
  dependency.

## Suites in the scaffold gate

| Suite | Command surface | Proves |
| --- | --- | --- |
| Rust nextest | `pixi run test` | Core parsing/output logic and all three binaries |
| Rust doc tests | `pixi run test` | Public Rust examples continue to compile |
| pytest | `pixi run test` | Backlog, repository, filesystem, and skill-hash checkers |
| Rust coverage | `pixi run coverage` | LCOV plus a 90% Rust line threshold |
| Python coverage | `pixi run coverage` | LCOV for repository scripts |

## Property-test regressions

When property tests are introduced, configure failure persistence to a
committed `proptest-regressions/` directory beside the owning test suite. If
a shrunk failure creates or changes a seed file, commit that file with the fix
and the regression test. Do not delete a seed merely to make CI pass; a seed
may be removed only when a generator/contract change makes it meaningless
and the task records why.

## Other regression artifacts

Keep failing fixtures minimal and named by behaviour rather than by incident
date. Binary fixtures belong beside the adapter that owns them and should be
small enough to review. Generated coverage, snapshots of live network
responses, and machine-local caches are not committed.
