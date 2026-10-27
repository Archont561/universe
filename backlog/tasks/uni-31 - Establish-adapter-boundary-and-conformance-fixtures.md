---
id: UNI-31
title: Establish adapter boundary and conformance fixtures
status: To Do
assignee: []
created_date: '2026-10-27'
updated_date: '2026-10-27'
labels:
  - architecture
  - audit
  - testing
milestone: m-1
dependencies:
  - UNI-30
priority: high
type: feature
ordinal: 31000
---

## Description
<!-- SECTION:DESCRIPTION:BEGIN -->
Define the in-process, serializable adapter request/result boundary and a
conformance fixture suite before extracting the Conda implementation. Official
adapters compile into the relevant binary; no external adapter process protocol
ships in this task.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Adapter capabilities, plan/build/audit requests, diagnostics, and artifact references are serializable.
- [ ] #2 Conformance fixtures cover success, user failure, usage failure, infrastructure failure, digest mismatch, and unknown evidence.
- [ ] #3 Core retains indexing, publication authority, content addressing, and generation assembly.
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 Existing UNI-7 through UNI-12 tasks reference the boundary rather than inventing per-command shapes.
- [ ] #2 `pixi run gates` passes.
<!-- DOD:END -->
