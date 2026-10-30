---
id: UNI-31
title: Establish adapter boundary and conformance fixtures
status: Done
assignee: []
created_date: '2026-10-27'
updated_date: '2026-10-31'
labels:
  - architecture
  - audit
  - testing
milestone: m-1
dependencies:
  - UNI-30
priority: high
type: feature
ordinal: 31000
---

## Description
<!-- SECTION:DESCRIPTION:BEGIN -->
Define the in-process, serializable adapter request/result boundary and a
conformance fixture suite before extracting the Conda implementation. Official
adapters compile into the relevant binary; no external adapter process protocol
ships in this task.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Adapter capabilities, plan/build/audit requests, diagnostics, and artifact references are serializable. — 2026-10-31: `adapter.rs` in `universe-core`, round-tripped through JSON by `crates/universe-core/tests/adapter.rs`.
- [x] #2 Conformance fixtures cover success, user failure, usage failure, infrastructure failure, digest mismatch, and unknown evidence. — 2026-10-31: six fixtures in `crates/universe-core/tests/fixtures/adapter-*.json`, each asserted to satisfy the conformance rules and to round-trip unchanged.
- [x] #3 Core retains indexing, publication authority, content addressing, and generation assembly. — 2026-10-31: `scripts/crate_check.py` still proves the neutral dependency closure, and `snapshot.rs`/`catalog.rs`/`artifact.rs` own identity and generation assembly; no adapter gained a core responsibility.
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [x] #1 Existing UNI-7 through UNI-12 tasks reference the boundary rather than inventing per-command shapes. — 2026-10-31: each of those task files now carries the `adapter.rs` paragraph.
- [x] #2 `pixi run gates` passes. — 2026-10-31: green in CI run 41107055677 (`gates` 1 m 9 s).
<!-- DOD:END -->
