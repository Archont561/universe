---
id: UNI-43
title: Implement the Python wheel Simple API adapter
status: To Do
assignee: []
created_date: '2026-10-27'
updated_date: '2026-10-27'
labels:
  - python
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
ordinal: 43000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Implement a Python wheel registry adapter behind neutral core contracts. Generate a
PEP-503-compatible Simple Repository API (with the accepted PEP-691 representation where
configured) and publish verified wheel files plus project/version metadata. Preserve wheel
filename/tag semantics, Python metadata and dependency information, and hash-fragment links.
The v1 artifact scope is wheels; do not imply arbitrary source-distribution builds or silently
invoke a Python builder.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Fixtures cover normalized project names, supported API representations, wheel tags, metadata, yanked/duplicate versions as specified, and SHA-256 link fragments.
- [ ] #2 Mismatched filename, package identity, metadata version, size, or digest rejects the artifact before publication.
- [ ] #3 A real pip-compatible client resolves/downloads and installs fixture wheels from a local registry with public network egress denied.
- [ ] #4 Index and wheel paths are static/protocol-compatible and are not replaced by portal links; unknown wheels fail closed.
- [ ] #5 Deterministic adapter tests pass offline; connected pinned release acquisition is tested as a separate lane.
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 Local client and golden-fixture tests pass without network egress.
- [ ] #2 Document wheel-only v1 scope and required wheel/index cache for sealed tests.
<!-- DOD:END -->
