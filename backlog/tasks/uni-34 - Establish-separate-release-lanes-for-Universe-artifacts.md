---
id: UNI-34
title: Establish separate release lanes for Universe artifacts
status: To Do
assignee: []
created_date: '2026-10-27'
updated_date: '2026-10-27'
labels:
  - ci
  - release
milestone: m-4
dependencies:
  - UNI-28
  - UNI-41
priority: medium
type: chore
ordinal: 34000
---

## Description
<!-- SECTION:DESCRIPTION:BEGIN -->
Create separate release lanes for the `universe` facade/umbrella package, `universe-kit`,
`universe-airlock`, and the distinct `universe-airlock-bootstrap` artifact. Preserve separate
artifacts, checksums, provenance, and clean-install smoke tests. Release records bind the
native registry assets and portal/catalog snapshot through the versioned manifest. Builds
remain untrusted and credential-free; publication uses a zero-checkout identity boundary.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Each artifact can be built, verified, and smoke-tested independently.
- [ ] #2 Consumer artifacts prove producer/publishing code is absent from their dependency graphs.
- [ ] #3 Static Conda channel, native Cargo/Python/npm endpoints, GitHub Release assets, and portal/catalog publication are dry-run tested against one pinned manifest before credentials are enabled.
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 Release tasks UNI-15, UNI-16, and UNI-20 are reconciled with the new artifact names.
- [ ] #2 `pixi run gates` and workflow lint pass.
<!-- DOD:END -->
