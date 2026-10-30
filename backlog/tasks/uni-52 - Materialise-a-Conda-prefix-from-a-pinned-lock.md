---
id: UNI-52
title: Materialise a Conda prefix from a pinned lock
status: To Do
assignee: []
created_date: '2026-10-31'
updated_date: '2026-10-31'
labels:
  - airlock
  - offline
  - adapters
milestone: m-5
dependencies:
  - UNI-32
  - UNI-51
documentation:
  - backlog/decisions/decision-102 - Original-conda-artifacts-and-effective-metadata.md
  - backlog/decisions/decision-103 - Finite-snapshots-locked-replay-and-curated-additions.md
  - backlog/docs/doc-106 - Offline-verification-and-acceptance-gates.md
priority: high
type: feature
ordinal: 52000
---

## Description
<!-- SECTION:DESCRIPTION.BEGIN -->
Turn a pinned lock plus mirrored artifacts into a working prefix on a host with no
network. A Conda package is not a tarball that can be dropped in place: paths carry a
build-time prefix placeholder that must be relocated, `info/paths.json` describes
symlinks and hardlinks, and `noarch: python` packages install differently from native
ones. Getting this wrong produces an environment that imports from the build host's
paths.

The restore is a replay, never a solve. The lock already names every artifact, so the
materialisation step builds an explicit, exact specification (URL or local path plus
the recorded sha256 for each entry) and asks an installer to realise it. Bundling a
small static installer is the accepted approach for the first version, because prefix
relocation is solved there and no dependency resolution is required; owning the
extractor in Rust is a later option, not a prerequisite.

Scope boundary to state plainly: restore reconstructs a prefix and its activation
environment. It does not reproduce the task runner's semantics, multi-environment
switching, or any solve-time behaviour. The lock is the whole contract.
<!-- SECTION:DESCRIPTION.END -->

## Acceptance Criteria
<!-- AC.BEGIN -->
- [ ] #1 A pinned lock restores into an empty prefix on Linux x86_64 with network egress denied, and the restored environment runs a native client.
- [ ] #2 Every installed artifact's digest matches the lock; a corrupted or missing artifact aborts before any state is activated, leaving the previous state untouched.
- [ ] #3 Prefix relocation is verified: no installed file references the original build prefix, and the check is part of the acceptance fixture rather than a manual step.
- [ ] #4 `noarch: python` and native packages are both covered by fixtures, including at least one package that carries an entry point.
- [ ] #5 The restore path contains no resolver and no channel metadata request; the run fails rather than falling back to a channel when an artifact is absent.
<!-- AC.END -->

## Definition of Done
<!-- DOD.BEGIN -->
- [ ] #1 A clean-container fixture restores Universe's own locked development environment from mirror branches only.
- [ ] #2 `pixi run gates` passes and no test requires network egress.
<!-- DOD.END -->
