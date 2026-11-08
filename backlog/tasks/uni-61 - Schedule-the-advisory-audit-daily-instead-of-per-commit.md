---
id: UNI-61
title: Run the advisory audit on a daily schedule instead of on every commit
status: In Progress
assignee: []
created_date: '2026-11-08'
updated_date: '2026-11-08'
labels:
  - infrastructure
  - gates
  - security
milestone: m-0
documentation:
  - backlog/decisions/decision-134 - Run-the-advisory-audit-on-a-daily-schedule,-not-on-every-commit.md
  - backlog/decisions/decision-120 - Offline-first-audit-gate-with-a-bounded-project-local-advisory-cache.md
priority: medium
type: chore
ordinal: 61000
---

## Description
<!-- SECTION:DESCRIPTION.BEGIN -->
`pixi run gates` carried the advisory audit, and CI runs `pixi run gates` on every pull
request. The audit's input — the RustSec advisory database and the crates.io index —
changes without anybody committing, so the gate had two failure modes at once: a pull
request reds for an advisory its diff never touched (and cannot fix), and a disclosure is
only noticed whenever somebody next pushes.

Move it to a clock. `gates` becomes a per-diff contract with no footnote, and a daily
`.github/workflows/security.yml` runs `pixi run --frozen audit` against the same
offline-first runner decision-120 defined — unchanged, still bounded, still seeded by
`pixi run audit-sync`, and still bootstrapping itself on a cold cache where the network
allows.

Because the audit no longer runs per commit, nothing else notices if its schedule
disappears: a deleted workflow or a removed cron leaves the repository green forever while
never auditing. `repo-check` therefore requires a scheduled workflow that runs the audit,
and refuses a return of `audit` to `gates`.
<!-- SECTION:DESCRIPTION.END -->

## Acceptance Criteria
<!-- AC.BEGIN -->
- [x] #1 `gates` no longer depends on `audit`, and `repo-check` refuses a regression that re-adds it.
- [x] #2 A scheduled workflow runs the audit task daily, with every action pinned and the Pixi version named.
- [x] #3 `repo-check` requires that some workflow declares a `schedule:` and runs the audit task, and refuses an audit that only runs on push.
- [x] #4 The task itself is unchanged: `python -m scripts.audit check`, offline-first, bounded, with `audit-sync` still the one network path.
- [x] #5 The change is recorded as a decision, with decision-120 amended where it claimed `gates` includes the audit.
<!-- AC.END -->

## Definition of Done
<!-- DOD.BEGIN -->
- [x] #1 Tests cover both invariants: the audit may not return to `gates`, and a scheduled workflow must run it (`scripts/tests/test_repo_check.py`).
- [ ] #2 `pixi run gates` passes with the audit no longer in it.
- [ ] #3 The scheduled workflow has run at least once on `main` and reported `success`.
<!-- DOD.END -->

## Notes
<!-- NOTES.BEGIN -->
2026-11-08 — reference point: `Archont561/artifactspec` has a `security.yml` that runs
`pixi run --locked audit` on a weekly cron (`25 6 * * 1`) *and* keeps `audit` in its full CI
lane. The shape is adopted (hand-written scheduled workflow, pinned actions, the same task),
the cadence is not: daily, because the job costs one short run and a seven-day window on a
supply-chain advisory is a long time to sit on a known-bad dependency. Keeping both is
rejected here because `artifactspec`'s audit also covers its Python development environment,
which does change with a diff; ours covers only the Rust dependency graph.
<!-- NOTES.END -->
