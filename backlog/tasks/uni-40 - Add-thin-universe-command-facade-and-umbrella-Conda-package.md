---
id: UNI-40
title: Add the thin `universe` command facade and umbrella Conda package
status: To Do
assignee: []
created_date: '2026-10-27'
updated_date: '2026-10-27'
labels:
  - architecture
  - packaging
milestone: m-8
dependencies:
  - UNI-15
  - UNI-34
  - UNI-38
priority: high
type: feature
ordinal: 40000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Complete the thin `universe-cli` workspace crate and its Conda recipe, whose package name is
`universe`. Its binary routes ordinary producer commands to `universe-kit` and
`universe airlock …` to `universe-airlock`, forwards arguments/exit status, and owns no
product logic. The `universe` Conda package depends on the separately installable
`universe-kit` and `universe-airlock` packages; the bootstrap remains separate. Preserve
component recipes and direct component binaries for users who do not want the umbrella.

The facade scaffold is present in the workspace; this task completes compatibility, package
metadata, release versioning, clean-channel installation, and conformance behavior.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 `universe <kit-args>` forwards to `universe-kit`; `universe kit <args>` removes the explicit namespace; `universe airlock <args>` forwards to `universe-airlock`.
- [ ] #2 Arguments (including non-UTF8 values where supported), child exit codes, and missing-component diagnostics are tested; facade does not duplicate Kit/Airlock business logic.
- [ ] #3 Conda package `universe` installs the facade and pulls the separately versioned component packages as runtime dependencies; all components remain independently installable.
- [ ] #4 Fresh environment tests verify the umbrella command dispatches after install and that bootstrap is not accidentally bundled as producer/consumer implementation.
- [ ] #5 Recipe and workspace version metadata are synchronized at release, and the package installs from the project's static channel.
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 Offline dispatcher tests pass; clean-channel package install/smoke tests pass in the connected release acceptance lane.
- [ ] #2 No registry, catalog, or Airlock protocol implementation is added to `universe-cli`.
<!-- DOD:END -->
