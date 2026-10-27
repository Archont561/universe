---
id: UNI-32
title: Implement Airlock sealed snapshot verify vertical slice
status: To Do
assignee: []
created_date: '2026-10-27'
updated_date: '2026-10-27'
labels:
  - airlock
  - offline
  - testing
milestone: m-8
dependencies:
  - UNI-30
  - UNI-31
priority: high
type: feature
ordinal: 32000
---

## Description
<!-- SECTION:DESCRIPTION:BEGIN -->
Implement the smallest `universe-airlock` consumer slice: read a sealed
manifest, verify every object in a SHA-256 content-addressed store, expose a
local native view, and activate state transactionally. Treat archive input as
untrusted: validate digest syntax before deriving paths, stream large objects
with bounded resource use, stage writes before activation, and fail closed on
mismatch or corruption. The binary contains no producer, build, watcher, or
publishing capability. Do not reuse placeholder or non-cryptographic CAS code.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 A fixture snapshot verifies completely and is consumable through a native client view offline.
- [ ] #2 Missing, extra, corrupted, and wrong-platform objects fail closed with structured diagnostics.
- [ ] #3 Tests prove no network is required after import.
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 Evidence levels distinguish inspected, verified, and unknown artifacts.
- [ ] #2 `pixi run gates` passes.
<!-- DOD:END -->
