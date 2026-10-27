---
id: UNI-39
title: Formalize Backlog planning and reference integrity
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
  - backlog/README.md
  - .agents/skills/backlog/SKILL.md
priority: medium
type: chore
ordinal: 39000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Formalize Universe's Backlog layout and planning practices without importing
other projects' product scope or inventing sprint records. Document the
distinction between milestone outcomes, optional sprint-train plans, blocking
dependencies, and task composition. Improve the local checker so task milestone
and dependency references cannot silently point at missing records.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 The Backlog README, planning guide, and backlog skill agree on record layout, milestone backlinks, dependency semantics, sprint-train use, archive conventions, and the fact that Universe has no sprint or parent/subtask task type.
- [x] #2 `backlog-check` rejects a task whose `milestone` does not identify a milestone record or whose `dependencies` contains an ID that does not identify a task; valid references pass.
- [x] #3 Tests cover empty, inline, and block dependency lists plus valid and dangling milestone/dependency references.
- [x] #4 Existing Backlog records pass the strengthened checker without adding empty-directory placeholders or changing unrelated task scope.
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [x] #1 `pixi run backlog-check` and the focused checker tests pass.
- [x] #2 Formatting, Python lint/typecheck, and Backlog validation pass. The full gates' Rust-dependent stages are blocked by the external crates.io TLS failure recorded below.
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
<!-- SECTION:NOTES:END -->
