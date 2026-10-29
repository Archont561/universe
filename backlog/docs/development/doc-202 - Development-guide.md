---
id: doc-202
title: Development guide
type: specification
created_date: '2026-10-27'
updated_date: '2026-10-27'
---
# Development guide

This is the durable development guide for the standalone Universe repository.
The short operational version is the root `CONTRIBUTING.md`; mandatory agent
rules are in `AGENTS.md`.

## Toolchain

| Concern | Choice |
| --- | --- |
| Environment | Pixi 0.83+ with the committed `pixi.lock` |
| Product language | Rust 2024 workspace under `crates/` |
| Repository scripts | Python 3.13 modules under `scripts/` |
| Python tests | pytest suites under `tests/scripts/` |
| Python format/lint/types | Ruff formatter, Ruff lint, strict mypy |
| Rust tests | External `tests/`, nextest, and doc tests |
| Coverage | `cargo llvm-cov` LCOV for Rust; pytest-cov LCOV for Python scripts |
| Workflow lint | Actionlint plus full-SHA action refs |
| Commits | Convco Conventional Commits |
| Hooks | Lefthook delegating to Pixi tasks |

There is no root TypeScript configuration, Bun lockfile, Biome config, or
TypeScript repository scripting in the scaffold. A future TypeScript product
surface must arrive as an explicit package and extend the same gate rather
than adding side-channel commands.

## Setup

```bash
pixi install --locked
pixi run setup
pixi run gates
```

## Gate semantics

- `fmt` — `cargo fmt`, Taplo, Ruff format, and Ruff autofixes.
- `fmt-check` — non-writing format checks.
- `lint` — Clippy over all targets/features with warnings denied; Ruff lint.
- `typecheck` — `cargo check`; strict mypy over `scripts/` and script tests.
- `test` — nextest, Rust doc tests, and pytest.
- `coverage` — instrumented Rust tests with a 90% line threshold and Python
  script LCOV with its Pixi-task coverage floor.
- `backlog-check` — Backlog IDs, statuses, filenames, frontmatter, and links.
- `repo-check` — required files, Cargo/Pixi version sync, clean standalone
  layout, and immutable workflow action refs.
- `skills-check` — skill frontmatter and content hashes in `skills-lock.json`.
- `audit` — the offline-first runner: `cargo deny --offline check` plus
  `cargo audit --no-fetch` against a bounded advisory cache (decision-120).
  `audit-sync` is the one networked task that seeds that cache.
- `alint-check` — the declarative repository layer from `.alint.yml`:
  required/forbidden paths, backlog filename and frontmatter conventions,
  relative-link resolution, manifest and gate assertions, and the
  skill-to-`skills-lock.json` registration relation (decision-121). It runs
  inside `check`, so it also runs in `offline-gates`: `alint` is pinned in
  `pixi.lock` and reaches a restored checkout with the next transport publish.
- `gates` — all checks plus workflow lint and build.

## Testing

- Follow TDD at parser, planner, adapter, auditor, CAS, restore, and output
  boundaries.
- Tests use fixtures and temporary directories and isolate process/home state.
- Offline claims require tests that run without network assumptions.
- A test-count regression, coverage drop below a threshold, or a new ignored
  advisory requires an explicit Backlog decision.

See doc-204 for regression artifacts and property-test seed policy.
