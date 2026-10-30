---
id: UNI-15
title: Finalize component Conda recipes and four-platform release packaging
status: To Do
assignee: []
created_date: '2026-10-26 20:15'
updated_date: '2026-10-31'
labels:
  - release
  - ci
milestone: m-4
dependencies:
  - UNI-9
  - UNI-10
  - UNI-34
  - UNI-41
documentation:
  - backlog/docs/architecture/doc-1 - Monorepo-and-toolchain-architecture.md
  - backlog/docs/architecture/doc-111 - V1-package-layout-and-offline-readiness.md
  - backlog/decisions/decision-1 - Two-tier-product-and-channel-boundary-(archont561-archont561-ships-universe-kit;-consumer-universes-target-arbitrary-channels).md
  - backlog/decisions/decision-8 - Owner-lane-vs-consumer-lane-GitHub-Actions-workflows-and-zero-checkout-OIDC-publishing.md
  - backlog/decisions/decision-119 - Full-multi-ecosystem-v1-and-static-catalog-distribution.md
priority: high
type: feature
ordinal: 15000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Finalize independently installable Conda component recipes and their four-platform
packaging/release workflow. Build jobs invoke `rattler-build` as a CI/build tool; it must not
be a runtime dependency of `universe-kit` merely because Kit orchestrates artifacts. Keep the
`universe-kit`, `universe-airlock`, and thin `universe` facade recipes separate. The facade
package depends on the component packages, with a pre-1.0 upper bound of the next minor
(`>=0.1,<0.2`, `decision-126`) rather than `<1`, so the solver cannot pair the umbrella with an
incompatible component. Airlock bootstrap remains a separate artifact of the consumer edition
(`decision-122`) and is packaged by UNI-55.

Release packaging validates binary/license/manifest evidence and records checksums. The
integrated v1 workflow requires the real generated portal bundle and protocol artifacts from
the pinned snapshot; a local placeholder is not acceptable release output. This task does not
cut either a Conda-only `v0.1.0` or a portal-only `v0.2.0` release.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Conda recipes build the component binaries on supported platforms and do not install `rattler-build` as a runtime dependency unless a separately justified user-facing command requires it.
- [ ] #2 Recipes pass package self-audit and clean install/functional smoke tests; the umbrella package installs the facade and resolves component runtime dependencies.
- [ ] #3 Release packaging includes the verified manifest, checksums, and portal/catalog snapshot identity; missing or placeholder release assets fail closed.
- [ ] #4 `pixi run repo-check` enforces recipe/workspace version synchronization and release workflow action pinning.
- [ ] #5 Every recipe's runtime bounds express the pre-1.0 compatibility boundary of `decision-126` (same minor), and a fixture proves the solver rejects a mismatched pair.
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 CI package jobs build and test each independent package; publication remains a separate credentialed lane.
<!-- DOD:END -->
