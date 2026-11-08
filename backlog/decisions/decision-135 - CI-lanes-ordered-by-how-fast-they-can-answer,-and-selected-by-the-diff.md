---
id: DECISION-135
title: CI lanes ordered by how fast they can answer, and selected by the diff
status: accepted
created_date: '2026-11-08'
updated_date: '2026-11-08'
---

## Context

The gate suite was correct and slow to be *wrong* in it. `pixi run gates` is one Pixi task
graph, Pixi runs `depends-on` strictly sequentially, and it stops at the first failure — so
the order of that graph is the feedback loop. Measured on 2026-11-08, cold, in the
developer sandbox:

| task | cost |
| --- | --- |
| `lint` (clippy, all targets) | 108s |
| `test` (nextest, doctests, pytest) | 100s |
| `coverage` (llvm-cov, pytest again) | ~120–160s |
| `fmt-check` | 8s |
| `backlog-check`, `repo-check`, `alint-check`, `crate-check`, `lint-gherkin` | **under 1.5s together** |
| `ci-check`, `airlock-check` | 1s warm, ~104s cold |

`check` ran `fmt-check, lint, typecheck, test, backlog-check, repo-check, skills-check,
alint-check, crate-check, ci-check, airlock-check, lint-gherkin`. So **five checks worth 1.5
seconds sat behind ~225 seconds of compilation**: a typo in a Backlog record, an `alint` rule
violation, or a missing crate boundary took four minutes to report.

Two smaller costs, both measured in the same runs: `universe / validate` declared
`needs: [view]`, so the critical path was the sum of two independent jobs rather than the
slower of them; and every generated job compiled the same workspace from cold (71s, 55s,
68s, 63s in one run) because nothing cached it.

## Decision

- `check-fast` holds the checks that need no compile: `fmt-check`, `backlog-check`,
  `repo-check`, `skills-check`, `alint-check`, `crate-check`, `lint-gherkin`. It is the
  **first** dependency of `check`, and `repo-check` refuses a `check` that puts it anywhere
  else or drops a member.
- `check` = `check-fast`, then `lint`, `typecheck`, `ci-check`, `airlock-check`, `coverage`.
  `test` is no longer part of it: `coverage` runs the whole suite under instrumentation, so
  `test` was a second full execution of both languages plus a second instrumented compile.
  `test` stays a task for the per-test reporting nextest gives, and pre-push still runs it.
- CI splits into three lanes plus a verdict. `fast` is `check-fast` + workflow lint + commit
  validation, unbounded by the diff because it is seconds. `heavy` is the compile-based
  remainder. `web` is the Bun workspace.
- `heavy` and `web` are selected by `scripts/detect_changes.py` from the diff. The
  classifier answers two questions (`code`, `web`) and treats an unknown path as code: a
  lane that runs too often costs minutes, a lane that silently skips costs the point of the
  gate. An unusable comparison revision exports both as true rather than false.
- The `gates` job name is kept, as an aggregator that always runs and decides what it owes.
  Branch protection requires a check by that name; a lane split without an aggregator leaves
  a pull request blocked on `Expected`, which is worse than a slow one. `repo-check` refuses
  a `ci.yml` that declares no `gates` job.
- Generated workflows get `timeout-minutes` on every job (a hang reads as `failed`, not as
  six hours of `pending`), a pinned `Swatinem/rust-cache` sharing one key across jobs, and
  `validate` no longer waits for `view`.

## Consequences

- A formatting, Backlog, or repository-invariant error reports in seconds instead of being
  queued behind a compile. That is the whole point; everything else here is secondary.
- A docs-only change runs only `fast`. A change that touches only `backlog/` still runs
  `fast`, because `alint-check` and `backlog-check` read the Backlog.
- The test suite executes once per run instead of twice. A failing test is still reported,
  by `coverage`, with less per-test detail than nextest; `pixi run test` remains available
  and is what pre-push runs.
- Jobs are cheaper: one shared Rust cache instead of four cold builds.
- Splitting lanes costs runner-minutes (`fast` and `heavy` run in parallel), paid for by the
  cache and by not running `heavy` on prose changes.
- This does not change what any single gate *checks* — only the order, the parallelism, and
  which lanes a diff selects. `pixi run gates` locally is still the whole suite.

## Alternatives

- **Reorder `check` and stop there.** Half the benefit, zero risk. Rejected as a stopping
  point because the lanes also remove the four cold compiles and the duplicate test run; but
  it is exactly the first commit's worth of this change and can be reviewed as such.
- **Make the Pixi graph parallel** (`pixi` has no parallel `depends-on`). Rejected: not a
  feature of the tool, and a shell-parallel wrapper would put the same work in one job where
  a failure hides the others' results.
- **Skip lanes with `paths-ignore`** on the workflow trigger. Rejected: a required check
  that never runs blocks the merge, because GitHub reports `Expected` forever. Selection has
  to happen inside a workflow that always starts.
- **Share one built binary between the `universe` jobs instead of trusting the cache.**
  Rejected: the rendered commands are `pixi run --frozen universe -- …`, which is `cargo run`,
  and cargo re-verifies fingerprints — a downloaded binary would be rebuilt anyway unless the
  commands stopped going through the task, which would break the dogfooding rule. A shared
  cache key gets the same effect without changing a single rendered command.
