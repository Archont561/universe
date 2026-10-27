---
id: UNI-46
title: Generate the browsable portal from verified pinned release and catalog data
status: To Do
assignee: []
created_date: '2026-10-27'
updated_date: '2026-10-27'
labels:
  - portal
  - catalog
  - testing
milestone: m-6
dependencies:
  - UNI-5
  - UNI-6
  - UNI-12
  - UNI-25
  - UNI-41
  - UNI-42
  - UNI-43
  - UNI-44
documentation:
  - backlog/docs/architecture/doc-3 - Static-channel-and-SvelteKit-UnoCSS-portal-specification.md
  - backlog/docs/architecture/doc-111 - V1-package-layout-and-offline-readiness.md
  - backlog/decisions/decision-119 - Full-multi-ecosystem-v1-and-static-catalog-distribution.md
priority: high
type: feature
ordinal: 46000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Generate the browsable catalog/documentation portal from an immutable catalog snapshot and
verified, pinned GitHub Release manifests/assets. The SSG build must not query a moving
release endpoint. It displays package metadata, versions, provenance/digests, and native
install guidance for Conda, Cargo, Python wheels, and npm. It is a discovery/documentation
surface only; clients resolve against separate protocol-compatible registries.

The same release/catalog snapshot identifier is recorded in portal output and release
metadata. The portal build becomes a required v1 component and is not a standalone later
minor release. Static Conda channel package paths remain owned by UNI-12.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The build validates pinned release identity, manifest schema, asset names, size, and digests before rendering; changed/tampered/mutable inputs fail closed.
- [ ] #2 Portal records the exact snapshot/release identities and displays all four ecosystems without inventing registry availability from links alone.
- [ ] #3 Build output is deterministic and generated offline from a local verified fixture once tool dependencies/assets are present.
- [ ] #4 Unit/property/snapshot and root/non-root browser tests pass with network guard; no release lookup occurs during SSG rendering.
- [ ] #5 E2E tests separately confirm Conda package paths and other native registry endpoints are served by their adapters, not by portal HTML.
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 Portal and native registry artifacts reference the same immutable catalog snapshot in an end-to-end fixture.
- [ ] #2 The distinction between discovery portal and package registry is explicit in UI/docs/tests.
<!-- DOD:END -->
