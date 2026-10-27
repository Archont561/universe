---
id: UNI-39
title: Formalize Backlog planning, ordered sprint train, and reference integrity
status: Done
assignee: []
created_date: '2026-10-27'
updated_date: '2026-10-27'
labels:
  - architecture
  - infrastructure
  - testing
milestone: m-0
dependencies:
  - UNI-28
documentation:
  - backlog/docs/development/doc-203 - Backlog-structure-and-planning.md
  - backlog/docs/roadmap/doc-112 - Ordered-milestone-DAG-and-sprint-train.md
  - backlog/README.md
  - .agents/skills/backlog/SKILL.md
priority: medium
type: chore
ordinal: 39000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Formalize Universe's Backlog layout and planning practices without importing
other projects' product scope. Keep milestone outcomes, the undated sprint-train
overlay, blocking dependencies, and flat task records distinct. Reconcile
milestones into a topological presentation, map tasks to milestones/sprints, and
improve the local checker so milestone/dependency references cannot silently
point at missing records.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 The Backlog README, planning guide, roadmap DAG/sprint train, and backlog skill agree on record layout, milestone backlinks, dependency semantics, flat tasks, and undated sprint-overlay conventions.
- [x] #2 `backlog-check` rejects a task whose `milestone` does not identify a milestone record or whose `dependencies` contains an ID that does not identify a task; valid references pass.
- [x] #3 Tests cover empty, inline, and block dependency lists plus valid and dangling milestone/dependency references.
- [x] #4 Reconciled records pass the strengthened checker, preserve stable IDs, and keep scope changes tied to decision-119 and the ordered v1 DAG.
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [x] #1 `pixi run backlog-check` and the focused checker tests pass.
- [x] #2 Formatting, Python lint/typecheck, Backlog validation, and offline gates pass in the restored environment; connected gates are tracked separately.
<!-- DOD:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
- 2026-10-27: Adapted useful external Backlog conventions. Kept
  milestone-to-task backlinks and blocking dependencies distinct from optional
  sprint-train planning; did not introduce a sprint task type or claim support
  for parent/subtask records.
- 2026-10-27: Added `doc-203`, updated the README and backlog skill, and
  strengthened `backlog-check` to verify milestone and dependency targets.
  Focused checker tests: 6 passed; `backlog-check`, `fmt-check`, Ruff, and
  mypy passed.
- 2026-10-27: `pixi install --locked` and `pixi run setup` passed after a
  verified sandbox restore. `pixi run test`, `pixi run typecheck`, `pixi run
  lint`, and `pixi run gates` stop while Cargo tries to update crates.io:
  fetching `https://index.crates.io/config.json` returns TLS
  `unexpected eof while reading` while resolving `serde`. The full Rust gates
  are therefore not claimed as passing; this is an external network blocker.
- 2026-10-27: Reconciled v1 task assignments into M0–M11, removed accidental
  cross-scope edges, added the native developer-transport prerequisite
  (UNI-48), and published an undated S0–S9 sprint train in doc-112. Sprints are
  sequencing only, not task parents or synthetic dependency edges.
- 2026-10-27: Restored the verified sandbox, ran `pixi install --frozen
  --offline` and `pixi run --offline --as-is offline-gates`; both passed.
<!-- SECTION:NOTES:END -->
