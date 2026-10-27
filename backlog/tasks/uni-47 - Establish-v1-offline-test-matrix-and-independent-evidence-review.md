---
id: UNI-47
title: Establish the v1 offline test matrix and independently review release evidence
status: To Do
assignee: []
created_date: '2026-10-27'
updated_date: '2026-10-27'
labels:
  - offline
  - testing
  - audit
  - release
milestone: m-9
dependencies:
  - UNI-12
  - UNI-13
  - UNI-14
  - UNI-15
  - UNI-20
  - UNI-21
  - UNI-22
  - UNI-23
  - UNI-25
  - UNI-26
  - UNI-32
  - UNI-33
  - UNI-34
  - UNI-38
  - UNI-40
  - UNI-41
  - UNI-42
  - UNI-43
  - UNI-44
  - UNI-45
  - UNI-46
documentation:
  - backlog/docs/development/doc-202 - Development-guide.md
  - backlog/docs/development/doc-204 - Testing-and-regression-artifacts.md
  - backlog/docs/doc-106 - Offline-verification-and-acceptance-gates.md
  - backlog/docs/architecture/doc-111 - V1-package-layout-and-offline-readiness.md
priority: high
type: task
ordinal: 47000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Build a per-ecosystem evidence matrix that distinguishes deterministic offline verification
from connected release acquisition/publication. Run sealed Conda, Cargo, Python-wheel, and
npm client fixtures plus Airlock restore/verify/execute with network egress denied; test
portal generation from pinned local catalog/release fixtures; and verify static Conda package
paths. Record exact dependency, browser, package-cache, and advisory-database closure needed
for each gate. Have a reviewer independent of the implementation inspect test provenance,
expected artifacts, and failure coverage before UNI-16 can cut v1.

Do not mark the complete test suite offline based only on `offline-gates`. The current Bun
install/web check, Playwright browser payloads, Rust advisory database updates, GitHub Release
lookups, and arbitrary Conda/PyPI/npm/Cargo fixture caches require explicit offline cache or
fixture evidence. Connected checks (release discovery, upstream fetch, OIDC publishing) remain
separate named jobs and must not be represented as offline tests.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Matrix names each test, artifact/tool dependency, required cache/fixture, expected egress policy, and evidence output for all four ecosystems, Airlock, portal, and static Conda publishing.
- [ ] #2 Rust/Pixi/Bun/npm/Python/Conda/browser inputs required by the offline matrix are available locally and tests pass with network egress denied; unavailable items remain explicit gaps, not waived claims.
- [ ] #3 Connected GitHub release acquisition, upstream retrieval, advisory refresh, and credentialed publishing are separately classified and tested in the connected lane.
- [ ] #4 Independent reviewer verifies fixture identity/digests, negative/tamper tests, client behavior, static artifact paths, and no public registry fallback.
- [ ] #5 The broader full gate is only labeled fully offline if every test/dependency above passes in a clean offline environment; otherwise report the exact remaining gaps.
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 Signed/traceable evidence matrix is ready for UNI-16 and records the actual closure, not lockfile-only assumptions.
- [ ] #2 Any remaining online-only or missing-cache coverage is explicitly listed in the release notes and cannot be silently treated as a sealed pass.
<!-- DOD:END -->
