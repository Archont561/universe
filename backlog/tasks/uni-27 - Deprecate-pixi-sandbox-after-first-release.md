---
id: UNI-27
title: Retire pixi-sandbox after native developer transport is proven
status: To Do
assignee: []
created_date: '2026-10-27'
updated_date: '2026-10-27'
labels:
  - deprecation
  - infrastructure
  - offline
milestone: m-11
dependencies:
  - UNI-48
  - UNI-16
documentation:
  - backlog/decisions/decision-117 - Deprecate-pixi-sandbox-after-first-universe-airlock-release.md
  - backlog/decisions/decision-116 - Use-pixi-sandbox-for-interim-developer-transport.md
  - backlog/tasks/uni-48 - Implement-native-developer-transport-before-retiring-pixi-sandbox.md
priority: medium
type: chore
ordinal: 27000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Remove the interim `pixi-sandbox` dependency and publisher workflow only after the
integrated v1 is released and UNI-48 proves an equivalent native developer transport. A
stable Airlock release by itself is not sufficient evidence. Keep the current transport
available until the replacement can restore the declared developer profile, verify every
payload, and satisfy the documented offline/network boundary.

Then remove the sandbox dependency, publish workflow, tasks/scripts/tests/bootstrap config,
and update the development guide. This is post-v1 cleanup, not a v1 release blocker.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Integrated v1 and UNI-48 native developer-transport acceptance pass on a clean supported host.
- [ ] #2 A clean checkout can restore and verify the declared developer profile through the native path with the specified network boundary and no undeclared registry fallback.
- [ ] #3 Only after #1–#2 pass, remove `pixi-sandbox`, its publisher workflow, related tasks/scripts/tests/config, and obsolete docs; product gates no longer reference it.
- [ ] #4 The migration retains recoverability and records the native transport's exact payload/cache requirements and limitations.
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 The replacement path passes offline/connected evidence checks before the interim transport is removed.
<!-- DOD:END -->
