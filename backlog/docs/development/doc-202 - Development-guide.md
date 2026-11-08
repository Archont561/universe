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
| Python tests | pytest suites under `scripts/tests/` |
| Python format/lint/types | Ruff formatter, Ruff lint, strict mypy |
| Rust tests | External `tests/`, nextest, and doc tests |
| Coverage | `cargo llvm-cov` LCOV for Rust; pytest-cov LCOV for Python scripts |
| Workflow lint | Actionlint plus full-SHA action refs |
| Gherkin lint | `gherlint` over every `features/*.feature`, failing closed when there are none |
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
- `test` — nextest, Rust doc tests, and pytest. Not part of `check`: `coverage` runs the
  same tests under instrumentation, so including both executed the suite twice. `test` is
  what pre-push runs, and what to reach for when nextest's per-test output is what you want.
- `coverage` — instrumented Rust tests with a 90% line threshold and Python
  script LCOV with its Pixi-task coverage floor.
- `backlog-check` — Backlog identity and cross-record integrity: filename-to-`id`
  equality, duplicate ids, status enums, and milestone/dependency resolution.
  Filename shape and relative-link resolution are `alint-check` rules.
- `repo-check` — what a declarative rule cannot express: the legacy-token scan,
  Cargo/Pixi version sync through workspace inheritance, recipe drift, and the
  offline-transport and audit-gate contracts.
- `skills-check` — skill frontmatter and content hashes in `skills-lock.json`,
  including the provenance of the skills vendored from upstream.
- `lint-gherkin` — `gherlint` over every `features/*.feature` file. Discovery fails
  closed, and a finding is printed as `file:line:column: message (name)`; the
  linter's own CLI exits zero on findings, so the exit code is this gate's job.
- `crate-check` — the dependency boundary of `universe-core`: `cargo metadata --locked`
  resolves the normal/build closure and every crate in it must be on the neutral
  allowlist, so the shared vocabulary cannot silently acquire an ecosystem, UI,
  network, or package-resolution dependency (UNI-30).
- `audit` — the offline-first runner: `cargo deny --offline check` plus
  `cargo audit --no-fetch` against a bounded advisory cache (decision-120).
  `audit-sync` is the one networked task that seeds that cache. **Not part of
  `gates`**: its input changes without a commit, so it runs daily in
  `.github/workflows/security.yml` instead (decision-134), and `repo-check`
  refuses both a return to `gates` and the loss of that schedule. Run it by hand
  with `pixi run audit`.
- `alint-check` — the declarative repository layer from `.alint.yml` (71 rules):
  required/forbidden paths and basenames, backlog filename and frontmatter
  conventions, relative-link resolution including links that leave `backlog/`,
  manifest and gate assertions, immutable workflow action refs, and the
  skill-to-`skills-lock.json` registration relation (decision-121). It runs
  inside `check`, so it also runs in `offline-gates`: `alint` is pinned in
  `pixi.lock` and reaches a restored checkout with the transport publish. Rules
  inherited from a bundled ruleset at a weaker level are overridden explicitly —
  `gha-pin-actions-to-sha` is `error` here, not the bundled `warning` — so
  retiring a Python check never downgrades a gate.
- `check-fast` — the checks that need no compile (`fmt-check`, `backlog-check`,
  `repo-check`, `skills-check`, `alint-check`, `crate-check`, `lint-gherkin`). Pixi runs
  `depends-on` serially and stops at the first failure, so this set runs **first** inside
  `check`: together they cost under two seconds, while the tasks that follow compile for
  minutes. `repo-check` refuses a `check` that orders them otherwise (decision-135).
- `gates` — all checks plus coverage, workflow lint, the web workspace, and build. This is
  the per-diff contract: everything in it is a statement about the change under review. The
  advisory audit is the one gate that is not, and it lives on a schedule (decision-134).

CI runs the same tasks in three lanes — `fast` (`check-fast`, workflow lint, commit
messages), `heavy` (the compile-based remainder), and `web` — with `heavy` and `web`
selected from the diff by `scripts/detect_changes.py`. A `gates` job aggregates them and
keeps the name branch protection requires, so a skipped lane reports as a pass only when the
diff says it had nothing to check (decision-135).

## Testing

- Follow TDD at parser, planner, adapter, auditor, CAS, restore, and output
  boundaries.
- Tests use fixtures and temporary directories and isolate process/home state.
- Offline claims require tests that run without network assumptions.
- A test-count regression, coverage drop below a threshold, or a new ignored
  advisory requires an explicit Backlog decision.

See doc-204 for regression artifacts and property-test seed policy.
