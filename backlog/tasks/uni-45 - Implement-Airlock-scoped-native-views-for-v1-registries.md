---
id: UNI-45
title: Implement Airlock scoped native views for all v1 registries
status: To Do
assignee: []
created_date: '2026-10-27'
updated_date: '2026-10-27'
labels:
  - airlock
  - registry
  - offline
milestone: m-9
dependencies:
  - UNI-12
  - UNI-32
  - UNI-33
  - UNI-38
  - UNI-41
  - UNI-42
  - UNI-43
  - UNI-44
documentation:
  - backlog/docs/doc-109 - Git-backed-catalogs-and-on-demand-command-execution.md
  - backlog/docs/doc-111 - V1-package-layout-and-offline-readiness.md
  - backlog/decisions/decision-113 - Native-client-execution-through-local-package-views.md
  - backlog/decisions/decision-114 - Pinned-Git-catalogs-and-selective-artifact-retrieval.md
priority: high
type: feature
ordinal: 45000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Complete Airlock native consumption for Conda, Cargo, Python wheels, and npm using the
protocol-compatible metadata/artifact views supplied by the adapters. Support explicitly
scoped native-client execution and session routing; preserve original source/lock identities
where the client contract permits. Do not transparently intercept arbitrary HTTP, install a
custom TLS root, rewrite persistent user configuration, or silently fall back to public
registries. Bootstrap remains a separate artifact and is not part of this runtime feature.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 End-to-end fixtures execute native Conda/Pixi, Cargo, pip-compatible, and npm clients against pinned local views with public network egress denied.
- [ ] #2 Exact catalog commit/index digest and artifact digest/size are enforced; corruption, missing objects, unknown requests, and unsupported protocols fail closed.
- [ ] #3 Scope/session boundaries are enforced and tested: no arbitrary client capture, no credential propagation to build commands, and no unintended global config/lockfile mutation.
- [ ] #4 Resolver semantics remain owned by native clients; Airlock only exposes the pinned finite universe and verified artifacts.
- [ ] #5 Both prepared sealed-air-gap inputs and approved-Git-connected selective cache misses are tested as distinct modes.
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 Sealed egress-denied native-client tests pass for all four protocols.
- [ ] #2 Bootstrap-ready and fully sealed offline-ready claims are separately documented with evidence.
<!-- DOD:END -->
