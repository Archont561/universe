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
envelopes. Use strict deserialization (unknown fields fail closed), explicit
schema versions, useful field-level diagnostics, and minimal valid/invalid
fixtures to make compatibility behaviour reviewable. Keep Conda-specific
platform, target, and fragment semantics behind the future Conda adapter
boundary. Start from tests and fixture records in this repository; any future
non-Rust consumer must use an explicit generated/versioned contract rather
than duplicate the authoritative types.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Core catalogues distinguish software, data, model, and skill entries without ecosystem dependencies.
- [x] #2 Artifact IDs, digests, diagnostics, and stdout/result envelopes have versioned tested types.
- [x] #3 Source fixtures and property/regression seeds follow doc-204.
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [x] #1 Dependency graph proves `universe-core` has no ecosystem/UI/network crates.
- [x] #2 `pixi run gates` passes.
<!-- DOD:END -->
