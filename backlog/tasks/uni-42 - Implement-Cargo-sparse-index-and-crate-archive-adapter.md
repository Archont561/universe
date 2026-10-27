---
id: UNI-42
title: Implement the Cargo sparse-index and crate-archive adapter
status: To Do
assignee: []
created_date: '2026-10-27'
updated_date: '2026-10-27'
labels:
  - cargo
  - registry
  - testing
milestone: m-6
dependencies:
  - UNI-31
  - UNI-41
documentation:
  - backlog/docs/architecture/doc-111 - V1-package-layout-and-offline-readiness.md
  - backlog/docs/doc-109 - Git-backed-catalogs-and-on-demand-command-execution.md
priority: high
type: feature
ordinal: 42000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Implement an independent Cargo native-registry adapter behind the neutral core boundary.
Generate a Cargo-compatible sparse index and immutable `.crate` archives from verified
release assets. Preserve crate version, dependencies/features, target conditions, checksum,
yank state, and index path conventions. Host the index and archives at protocol-compatible
static paths; do not substitute portal pages or bare GitHub download links for the registry.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Golden fixtures cover sparse index path naming, config/download URL construction, crate metadata, checksum, feature/dependency semantics, and yanked versions.
- [ ] #2 Tampered, duplicate, wrong-identity, or missing `.crate` artifacts fail closed before index publication.
- [ ] #3 A real Cargo client resolves and downloads from a local fixture registry with public network egress denied; a separate connected test verifies pinned release acquisition.
- [ ] #4 Deterministic output is byte-identical for the same verified input snapshot and includes no unpinned upstream lookup.
- [ ] #5 Adapter tests run offline without adding Cargo registry implementation to `universe-core`.
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 Protocol fixtures and local Cargo-client tests pass in the sealed offline matrix.
- [ ] #2 Registry endpoint and artifact paths are documented for static hosting and Airlock local views.
<!-- DOD:END -->
