---
id: UNI-44
title: Implement npm registry metadata and tarball adapter
status: To Do
assignee: []
created_date: '2026-10-27'
updated_date: '2026-10-27'
labels:
  - npm
  - registry
  - testing
milestone: m-9
dependencies:
  - UNI-31
  - UNI-41
documentation:
  - backlog/docs/architecture/doc-111 - V1-package-layout-and-offline-readiness.md
  - backlog/docs/doc-109 - Git-backed-catalogs-and-on-demand-command-execution.md
priority: high
type: feature
ordinal: 44000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Implement an npm-compatible registry adapter behind the neutral core boundary. Produce
package metadata/packuments and immutable package tarballs from verified, pinned release
assets, preserving scoped package names, versions, dependency/peer/optional dependency
semantics, platform/engine constraints, dist-tags policy, tarball URL, and integrity values.
Keep npm tarball bytes and metadata at protocol-compatible static paths; GitHub release
pages are not an npm registry.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Golden fixtures cover scoped/unscoped package names, semver versions, dependency/peer/optional metadata, engines/platforms, tarball references, integrity, and dist-tag behavior.
- [ ] #2 Invalid/tampered tarballs, mismatched identity, duplicate versions, and mutable asset references fail closed.
- [ ] #3 A real npm-compatible client resolves and installs a package from a local registry fixture with external network egress denied.
- [ ] #4 Generated metadata and tarball paths are deterministic and match the pinned release/catalog snapshot.
- [ ] #5 Offline tests prove the registry client cannot fall through to the public npm registry.
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 Local client/golden tests pass offline and connected release acquisition remains separately classified.
- [ ] #2 Required npm metadata/tarball cache inputs for sealed tests are documented; `bun.lock` is not claimed to include registry artifacts.
<!-- DOD:END -->
