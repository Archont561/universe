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
Choose and implement a reproducible transport for materialising the locked
Pixi/Rust/Python/Cargo developer environment under a documented network
boundary. Evaluate a separately published, platform-qualified bundle containing
the default Pixi environment and a Cargo vendor tree whenever the locked
workspace has registry dependencies (the current crates use local path
workspace dependencies only), with a native restore launcher that fetches only
the selected transport ref and restores locally. Keep transport publication manual and fail-closed by default; support
explicit no-fetch and no-user-tool-registration modes so restore does not
silently alter profiles or require public package registries after the bundle
is available. Do not treat this developer transport as an Airlock sealed
snapshot or as proof that npm, advisories, arbitrary tools, or native host
prerequisites are included. Generated launcher/configuration must be
reproducible and tested.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 A clean Linux x86_64 checkout can materialise the locked environment under the documented network boundary, or the limitation is explicitly accepted in a decision.
- [ ] #2 The reviewed bundle config and publisher include the default Pixi environment, the complete locked Cargo dependency graph (vendoring registry crates when present), and republish triggers for root/crate Cargo manifests, Cargo.lock, pixi.toml, and pixi.lock.
- [ ] #3 The checked-in restore launcher verifies the published branch before restore, supports a no-profile-edit mode, and has shell/config regression coverage.
- [ ] #4 The transport choice states how it relates to UNI-27 and the Airlock bootstrap boundary.
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 The chosen transport proves `pixi run gates` from a wiped environment.
<!-- DOD:END -->
