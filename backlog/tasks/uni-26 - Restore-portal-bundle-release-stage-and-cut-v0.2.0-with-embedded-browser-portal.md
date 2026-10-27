---
id: UNI-26
title: Integrate the generated portal into the v1 packaging and release workflow
status: To Do
assignee: []
created_date: '2026-10-27 13:40'
updated_date: '2026-10-27'
labels:
  - portal
  - ci
  - release
milestone: m-7
dependencies:
  - UNI-25
  - UNI-41
documentation:
  - backlog/docs/architecture/doc-111 - V1-package-layout-and-offline-readiness.md
  - backlog/docs/roadmap/doc-4 - Milestone-roadmap-and-skills-sh-rollout.md
  - backlog/decisions/decision-8 - Owner-lane-vs-consumer-lane-GitHub-Actions-workflows-and-zero-checkout-OIDC-publishing.md
  - backlog/decisions/decision-119 - Full-multi-ecosystem-v1-and-static-catalog-distribution.md
priority: high
type: feature
ordinal: 26000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Integrate deterministic portal generation from a verified, pinned release/catalog snapshot
into the v1 workflow. The portal build consumes prevalidated local snapshot data and locally
available build dependencies; release discovery and artifact acquisition are explicit,
connected inputs. Do not let a moving GitHub `latest` reference or a browser link become a
catalog identity or native registry endpoint.

Build the SSG once, upload its generated tree as a workflow artifact, and pass the same bytes
to package/embed and static-site publication jobs. Keep generated SSG output git-ignored. A
placeholder may remain for local scaffold builds, but the release workflow must fail closed if
the generated portal tree for the pinned catalog is missing or does not match its recorded
digest. This task integrates the portal into v1; it does not cut a separate `v0.2.0` release.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Static portal output is byte-deterministic from a pinned catalog/release fixture and includes the catalog and package pages.
- [ ] #2 Release workflow builds the portal once, verifies the expected digest, and supplies identical output to all consumers/publication jobs.
- [ ] #3 A missing portal build, changed catalog pin, or checksum mismatch fails the v1 release workflow before publication.
- [ ] #4 Browser tests pass for root and non-root subpath deployments with network guard enabled; no hidden release lookup occurs during SSG generation.
- [ ] #5 Native Conda/Cargo/Python/npm metadata and artifact endpoints remain protocol-specific and are not replaced by portal links.
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 Docs and workflow tests describe a single integrated v1 release; no `v0.2.0` portal-only release stage remains.
<!-- DOD:END -->
