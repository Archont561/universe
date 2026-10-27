---
id: UNI-30
title: Define authoritative schemas in ecosystem-neutral core
status: To Do
assignee: []
created_date: '2026-10-27'
updated_date: '2026-10-27'
labels:
  - architecture
  - core
  - testing
milestone: m-1
dependencies:
  - UNI-28
priority: high
type: feature
ordinal: 30000
---

## Description
<!-- SECTION:DESCRIPTION:BEGIN -->
Define the first small ecosystem-neutral schema slice for manifests,
catalogues, entries, artifacts, diagnostics, snapshots, and result
envelopes. Keep Conda-specific platform, target, and fragment semantics
behind the future Conda adapter boundary. Start from tests and fixture
records in this repository.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Core catalogues distinguish software, data, model, and skill entries without ecosystem dependencies.
- [ ] #2 Artifact IDs, digests, diagnostics, and stdout/result envelopes have versioned tested types.
- [ ] #3 Source fixtures and property/regression seeds follow doc-204.
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 Dependency graph proves `universe-core` has no ecosystem/UI/network crates.
- [ ] #2 `pixi run gates` passes.
<!-- DOD:END -->
