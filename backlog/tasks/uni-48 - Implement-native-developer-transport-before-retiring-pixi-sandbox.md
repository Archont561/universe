---
id: UNI-48
title: Implement and validate native developer transport before retiring pixi-sandbox
status: To Do
assignee: []
created_date: '2026-10-27'
updated_date: '2026-10-27'
labels:
  - infrastructure
  - offline
  - testing
milestone: m-11
dependencies:
  - UNI-16
  - UNI-33
  - UNI-36
  - UNI-45
documentation:
  - backlog/docs/architecture/doc-111 - V1-package-layout-and-offline-readiness.md
  - backlog/docs/doc-103 - Init-bootstrap-and-restore-lifecycle.md
  - backlog/docs/doc-106 - Offline-verification-and-acceptance-gates.md
  - backlog/decisions/decision-116 - Use-pixi-sandbox-for-interim-developer-transport.md
  - backlog/decisions/decision-117 - Deprecate-pixi-sandbox-after-first-universe-airlock-release.md
priority: medium
type: feature
ordinal: 48000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Implement and qualify the native developer-environment transport contemplated by decision-117,
using the released Airlock/catalog/payload mechanisms rather than relying on `pixi-sandbox`
for the final restore. Define the supported developer profile and enumerate its exact Pixi,
Rust, Python, Cargo-vendor, Bun/npm, browser, and host-system inputs. Preserve the separation
between a restored developer toolchain and a sealed product/project environment; do not claim
every tool or registry cache is included unless it is present and tested.

This task proves the replacement path but does not remove `pixi-sandbox`; retirement and
cleanup are a separate gated action in UNI-27. Keep the current transport operational until
this task's evidence is accepted.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 A fresh supported checkout restores a declared developer profile using only the documented sources and pins, without mutable `latest` references.
- [ ] #2 Every restored file/tool payload is integrity-verified; missing, corrupt, or unsupported platform inputs fail closed before activation.
- [ ] #3 Offline capability is tested with network egress denied after the declared bundle is present; online acquisition and publication are separate evidenced phases.
- [ ] #4 Bun/npm archives, Playwright/browser payloads, advisory data, Conda caches, and arbitrary user project artifacts are explicitly included, excluded, or deferred—not inferred from lockfiles.
- [ ] #5 Native restore and recovery are tested on a clean supported host while the existing pixi-sandbox path remains available as rollback until UNI-27.
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 Independent review accepts the exact closure and restore evidence; UNI-27 may then remove the interim transport.
- [ ] #2 No claim of complete offline development is made beyond the specific profile and evidence verified.
<!-- DOD:END -->
