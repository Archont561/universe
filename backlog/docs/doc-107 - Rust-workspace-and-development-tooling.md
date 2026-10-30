---
id: doc-107
title: Rust workspace and development tooling
type: guide
created_date: '2026-10-25T23:49:00Z'
updated_date: '2026-10-30'
---

# Rust workspace and development tooling

**Status: current repository setup.** Application behavior is VERSION `0.1.0`; future product architecture is documented separately.

## Source layout and dependency discipline

- `crates/universe-airlock`: Clap-based command-line entrypoint and VERSION re-export.
- `crates/universe-core`: headless library exposing the shared Cargo-derived VERSION.
- Each crate has separate `src/` and `tests/`; unit-test harnesses for src targets are disabled, and the layout validator rejects inline Rust tests.
- `tests/automation`: Python regression tests for tooling; these are not offline-product acceptance tests.
- `scripts/*.py`: Python automation only, dispatched as `pixi run scripts <script name>`.
- Root Cargo/Pixi/npm locks and config files are source inputs. Builds, coverage, tool reports, exports, and temporary fixtures are disposable outputs, not repository deliverables.

Only four Pixi features remain: **rust, utils, python, typescript**. The default environment combines them; focused environments share a solve group and include the Python/shared utilities needed by their tasks. Python/Node tooling does not turn the Universe Airlock application into a multi-language implementation.

## Workspace-managed tools

Rust provides rustfmt/Clippy and is accompanied by nextest, llvm-cov, cargo-deny, rattler-build, and license bundling. Shared utilities include Lefthook, Convco, actionlint, and Git. Python and TypeScript features supply the selected automation/tooling stacks.

Use npm for root package.json/package-lock.json, Backlog.md, and skills. Do not switch the root package manager to pnpm/yarn/bun just because their executables are available. npm installation uses the lock and disables lifecycle scripts.

cargo-dist is installed from a checksum/size-pinned official asset into ignored workspace-local tooling state, not through a global cargo install. `config/actions.lock.json`, `config/dist-bootstrap.json`, `config/pixi-bootstrap.json`, and `config/sandbox-bootstrap.json` are reproducibility inputs, not loose generated artifacts to delete.

## Entry points

```text
pixi run setup
pixi run ci
pixi run scripts --help
pixi run test
pixi run coverage
pixi run backlog doc list
pixi run backlog decision list
```

If Pixi is absent, host Python 3.11+ can run `python scripts/run.py bootstrap-pixi`; this is the pre-Pixi exception. It does not change shell profiles. Keep the selected Pixi executable on PATH for Git clients/hooks.

## Git hooks

All active workflow run steps and Lefthook commands invoke named Pixi tasks. Convco reads `.versionrc` at the repository root, the path it discovers without a flag and is scoped to commit-message linting and changelog drafting; it is **not** the release authority and computes no version that ships (`decision-123`). Version literals are declared and asserted for parity, never rewritten by a tool.

- **commit-msg:** check the message for that commit.
- **pre-commit:** format staged Rust via stdin and staged configuration; avoid recursive edits of unstaged modules. Auto-update only fully staged blobs/modes. If a partially staged file needs formatting, fail without changing earlier files. Clippy reads an exported Git-index snapshot, not dirty worktree source.
- **pre-push:** read all ref updates, validate individual outgoing commits, union affected scopes, and run nextest/cargo-deny against actual pushed-tip snapshots. Handle new refs conservatively, deletions, merge/root commits, rewinds, missing bases, and multiple refs sharing a tip.

Affected scope means owning crates plus transitive reverse dependents; shared Cargo/lock/tooling configuration selects the workspace. Cargo-deny checks transitive dependency closures. Full CI remains authoritative. Temporary alternate indexes do not change real HEAD, branches, or the real index.

## Backlog and source-only maintenance

Backlog is filesystem-only: auto-commit, remote operations, automatic browser launch, and AI integrations are disabled. Design docs/ADRs are now populated; no implementation tasks or skill content were seeded.

Keep design under backlog/docs and backlog/decisions, with README as the root entrypoint. Do not recreate a separate design workspace, top-level docs tree, ZIP exports, old ecosystem workarounds, or checked-in build/report bundles. Normal commands may generate ignored build/cache outputs when needed; they are not source deliverables.

The local Git repository has no initial commit. No push, release publication, or external account configuration has been performed by this consolidation.

## Interim pixi-sandbox developer tooling

Pinned pixi-sandbox 0.9.0 is in utils without adding another Pixi feature or changing npm/Python ownership. Native generated templates are inactive under config/pixi-sandbox; root restore.sh is generated consumer glue. The manual named-task developer publisher is separate from CI/application release. See the [integration guide](doc-110%20-%20Interim-development-transport-with-pixi-sandbox.md) for regeneration/check commands, exact scope, and missing offline inputs. No Universe Airlock product init/exec/restore implementation is implied.
