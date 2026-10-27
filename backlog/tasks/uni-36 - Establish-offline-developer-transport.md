---
id: UNI-36
title: Establish offline developer transport
status: To Do
assignee: []
created_date: '2026-10-27'
updated_date: '2026-10-27'
labels:
  - bootstrap
  - infrastructure
  - offline
milestone: m-0
dependencies:
  - UNI-28
priority: medium
type: chore
ordinal: 36000
---

## Description
<!-- SECTION:DESCRIPTION:BEGIN -->
Decide how a clean checkout materialises the locked Pixi/Rust/Python/Cargo
environment under a documented network boundary: a generated Universe-native
developer bundle, an explicitly configured sandbox transport, or a documented
deferral until the Airlock consumer can carry developer environments.
Generated launchers and configuration must be reproducible and tested.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 A clean checkout can materialise the locked environment under the documented network boundary, or the limitation is explicitly accepted in a decision.
- [ ] #2 Generated launchers/config are reproducible and covered by tests.
- [ ] #3 The transport choice states how it relates to UNI-27 and the Airlock bootstrap boundary.
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 The chosen transport proves `pixi run gates` from a wiped environment.
<!-- DOD:END -->
