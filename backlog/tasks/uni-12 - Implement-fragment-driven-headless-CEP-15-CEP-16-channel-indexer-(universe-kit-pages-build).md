---
id: UNI-12
title: Implement standard static Conda channel indexing and artifact publication paths
status: To Do
assignee: []
created_date: '2026-10-26 20:15'
updated_date: '2026-10-27'
labels:
  - channel
  - testing
milestone: m-3
dependencies:
  - UNI-10
  - UNI-11
documentation:
  - backlog/docs/architecture/doc-3 - Static-channel-and-SvelteKit-UnoCSS-portal-specification.md
  - backlog/docs/architecture/doc-111 - V1-package-layout-and-offline-readiness.md
  - backlog/decisions/decision-5 - Static-Conda-channel-on-GitHub-Pages-+-GitHub-Releases-via-CEP-15-base_url,-CEP-16-shards,-and-immutable-JSON-fragments.md
  - backlog/decisions/decision-119 - Full-multi-ecosystem-v1-and-static-catalog-distribution.md
priority: high
type: feature
ordinal: 12000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Implement deterministic Conda channel indexing and a static publish layout from verified
package artifacts and audit fragments. Emit standards-compatible metadata for all configured
and supported subdirs, including empty subdirs, and publish the actual package files at the
channel's canonical subdir paths (or an explicitly tested equivalent static object-store
layout). GitHub Release links alone are not a usable Conda channel and must not be treated as
substitutes for static package availability.

Emit CEP-15 repodata and CEP-16 shard files, `run_exports` metadata, channel metadata, and
`.nojekyll` as specified in doc-3. Resolve `base_url`/artifact references so each advertised
package can be downloaded by a native Conda client from the published static channel. Portal
HTML/catalog hydration is implemented separately in UNI-25/UNI-46; this task owns the
Conda solver and package artifact surface only.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 `proptest` proves byte-identical repodata, compressed metadata, and shards for arbitrary input fragment order.
- [ ] #2 Every configured subdir, including empty subdirs, has valid metadata and standard channel paths; every advertised artifact's bytes are present at the canonical static path and match its digest/size.
- [ ] #3 Local HTTP integration proves a real Conda/Pixi client resolves, downloads, and installs a package from the generated static channel with network egress denied outside the local fixture.
- [ ] #4 Missing artifact bytes, digest mismatch, unsupported metadata, or unavailable static path fails publication closed; a GitHub Release URL alone fails the fixture acceptance test.
- [ ] #5 Output contains Conda metadata and package files only; portal extraction/HTML hydration is not invoked.
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 BDD/property tests and local HTTP client resolution tests pass with the offline fixture set.
- [ ] #2 The deployment contract documents the static hosting/object-storage paths and makes no claim that portal links are a channel.
<!-- DOD:END -->
