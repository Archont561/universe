---
id: UNI-28
title: Establish standalone Universe monorepo baseline
status: Done
assignee: []
created_date: '2026-10-27'
updated_date: '2026-10-27'
labels:
  - architecture
  - ci
  - core
milestone: m-0
dependencies: []
priority: high
type: chore
ordinal: 28000
---

## Description
<!-- SECTION:DESCRIPTION:BEGIN -->
Create the clean standalone `universe/` monorepo scaffold: neutral Rust core,
three version-reporting binaries, local agent skills, Python repository
checks, CI workflow, Backlog records, lockfiles, and an initial conventional
commit. The scaffold is intentionally small and gate-clean; package building,
publishing, snapshots, and adapters remain explicit follow-up tasks.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 The repository is self-describing: canonical docs, decisions, milestones, tasks, and skills live under `universe/` with checked IDs and links.
- [x] #2 `universe-core`, `universe-kit`, `universe-airlock`, and `universe-airlock-bootstrap` build from the locked Pixi environment and report version in plain and JSON modes.
- [x] #3 Formatting, lint, typecheck, tests, coverage, Backlog/repo/skills checks, workflow lint, dependency audit, and build all pass through `pixi run gates`.
- [x] #4 Repository automation is Python-only; there is no root TypeScript configuration or TypeScript repository scripting.
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [x] #1 `Cargo.lock`, `pixi.lock`, and `skills-lock.json` are current.
- [x] #2 The repository has a clean conventional initial commit.
- [x] #3 No placeholder `.gitkeep`, notice, or standalone-cleanup artifact is required to keep the tree valid.
<!-- DOD:END -->

## Implementation Notes
<!-- SECTION:NOTES:BEGIN -->
- 2026-10-27: Established the four-crate Rust workspace and Python checkers.
- 2026-10-27: `pixi run gates` passed for the scaffold, covering Rust and
  Python tests, coverage thresholds, format, lint, typecheck, audit,
  workflow, Backlog, repository, skills, and build checks.
<!-- SECTION:NOTES:END -->
