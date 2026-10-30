---
id: UNI-16
title: Cut and verify the integrated Universe v1 release
status: To Do
assignee: []
created_date: '2026-10-26 20:15'
updated_date: '2026-10-31'
labels:
  - release
  - testing
milestone: m-10
dependencies:
  - UNI-47
documentation:
  - backlog/docs/architecture/doc-111 - V1-package-layout-and-offline-readiness.md
  - backlog/docs/roadmap/doc-4 - Milestone-roadmap-and-skills-sh-rollout.md
  - backlog/decisions/decision-119 - Full-multi-ecosystem-v1-and-static-catalog-distribution.md
priority: high
type: task
ordinal: 16000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Cut the first stable integrated Universe v1 only after the independent evidence review in
UNI-47 passes. Publish the component/umbrella artifacts and pinned release manifest, native
registry indexes/assets, standard static Conda channel, and generated portal from the same
verified catalog/release snapshot. Keep credentialed publishing isolated from untrusted build
jobs and publish checksums/provenance for every release asset. The release is cut through the
`universe release` verbs (`decision-126`): a prerelease attempt produces a draft with no tag,
promotion publishes it, and every destination is mandatory, journaled, and resumable.

The release is incomplete if any required surface is absent: Conda, Cargo crates, Python
wheels, npm packages, Airlock consumption, the browsable portal, or static Conda package
files at standard channel paths. This replaces the former Conda-only `v0.1.0` and deferred
portal `v0.2.0` release tasks; it does not merely rename either staged release.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Versioned `universe`, `universe-kit`, `universe-airlock`, and separate bootstrap artifacts are available with verified manifests/checksums and intended package dependencies.
- [ ] #2 Conda packages resolve/install through a static channel with package bytes at standard subdir paths; Cargo and Python-wheel clients resolve/fetch native fixtures from their protocol-compatible endpoints; an npm client installs from the declared hosted store (`npmjs` or `github-packages`) and the portal links to that store rather than serving metadata (`decision-127`).
- [ ] #3 Airlock consumes pinned catalog snapshots for all four protocols and fails closed for missing/corrupt artifacts and unsupported requests, with sealed tests denying external egress.
- [ ] #4 Portal is generated from the exact pinned catalog/release snapshot and displays supported package metadata; it is not treated as the registry endpoint.
- [ ] #5 Independent evidence review, clean-install checks, release checksums, provenance/attestations, and connected publication evidence are attached to the release record.
- [ ] #6 The release workflow used for this release is the committed rendered file, `pixi run ci-check` passes at the released commit, and the workflow's setup step references `./setup-dev` by local path rather than by SHA.
- [ ] #7 The release record names the controller identity (`mode: dev` with commit, or `mode: release` with version and asset digest), and re-running the failed publish jobs completes the release without republishing any destination.
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 The integrated v1 release artifacts and catalog snapshot are published and independently verifiable.
- [ ] #2 No test report claims the full suite is offline unless every dependency/cache closure was verified with network egress denied.
<!-- DOD:END -->
