---
id: UNI-33
title: Implement Airlock bootstrap verify and restore skeleton
status: To Do
assignee: []
created_date: '2026-10-27'
updated_date: '2026-10-27'
labels:
  - airlock
  - bootstrap
  - offline
milestone: m-8
dependencies:
  - UNI-32
priority: high
type: feature
ordinal: 33000
---

## Description
<!-- SECTION:DESCRIPTION:BEGIN -->
Give `universe-airlock-bootstrap` its minimal real verbs—`version`, `verify`,
`restore`, and `doctor`—while keeping adapters, prompts, TUI, HTTP serving, and
publishing out of the artifact. Generated POSIX/PowerShell entrypoints merely
locate and launch the pinned native bootstrapper.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Bootstrap verifies manifest and payload before changing target state.
- [ ] #2 Restore installs the pinned Pixi-managed `universe-airlock` from local payload and activates transactionally.
- [ ] #3 Failure leaves the previous active state untouched.
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 Clean-environment fixture restores on Linux x86_64.
- [ ] #2 `pixi run gates` passes.
<!-- DOD:END -->
