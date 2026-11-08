---
id: UNI-62
title: Order the gates by feedback latency and split CI into diff-selected lanes
status: In Progress
assignee: []
created_date: '2026-11-08'
updated_date: '2026-11-08'
labels:
  - infrastructure
  - gates
  - ci
milestone: m-0
documentation:
  - backlog/decisions/decision-135 - CI-lanes-ordered-by-how-fast-they-can-answer,-and-selected-by-the-diff.md
  - backlog/decisions/decision-134 - Run-the-advisory-audit-on-a-daily-schedule,-not-on-every-commit.md
priority: medium
type: chore
ordinal: 62000
---

## Description
<!-- SECTION:DESCRIPTION.BEGIN -->
`pixi` runs `depends-on` sequentially and stops at the first failure, so the order of the
`check` graph is the feedback loop. It was the wrong order: five checks that finish in under
1.5 seconds sat behind ~225 seconds of compilation, so a typo in a Backlog record took four
minutes to report.

Reorder them, remove a duplicated test run, and put the compile-based work in lanes a diff
can select. Measurements behind each step, taken cold on 2026-11-08, are in decision-135.
<!-- SECTION:DESCRIPTION.END -->

## Acceptance Criteria
<!-- AC.BEGIN -->
- [x] #1 `check-fast` holds every check that needs no compile, and is the first dependency of `check`; `repo-check` refuses a reordering or a missing member.
- [x] #2 `test` is no longer part of `check`, because `coverage` executes the same tests under instrumentation; the task stays for nextest's per-test reporting and for pre-push.
- [x] #3 CI declares three lanes (`fast`, `heavy`, `web`) plus a `gates` aggregator, and `repo-check` refuses a `ci.yml` with no `gates` job.
- [x] #4 `heavy` and `web` are selected by the diff, and an unknown path selects them rather than skipping them.
- [x] #5 Every generated job carries `timeout-minutes`, so a hang reports as a failure rather than six hours of `pending`.
- [x] #6 Generated jobs restore one shared Rust cache instead of each paying the cold build again.
- [x] #7 `universe / validate` no longer needs `view`, so the critical path is the slower job rather than the sum.
<!-- AC.END -->

## Definition of Done
<!-- DOD.BEGIN -->
- [x] #1 The classifier is tested against a real Git repository, including the case where the comparison revision is unavailable.
- [x] #2 The ordering and aggregator invariants are enforced by `repo-check`, with tests.
- [ ] #3 `pixi run gates` passes, and a pull request shows `fast` reporting before `heavy` finishes.
- [ ] #4 The first run on `main` after the split is green, including the `gates` aggregator.
<!-- DOD.END -->

## Notes
<!-- NOTES.BEGIN -->
2026-11-08 — the second deviation worth recording: sharing one built binary between the
`universe` jobs was considered and rejected, because the rendered commands go through
`pixi run --frozen universe` (`cargo run`), and cargo re-verifies fingerprints, so a
downloaded binary would be rebuilt anyway. A shared cache key gives the same effect without
changing a rendered command.
<!-- NOTES.END -->
