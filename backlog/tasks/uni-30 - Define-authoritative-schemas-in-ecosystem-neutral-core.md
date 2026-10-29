---
id: UNI-30
title: Define authoritative schemas in ecosystem-neutral core
status: Done
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

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
- 2026-10-29: Slice landed in `universe-core` as six modules over the existing
  scaffold vocabulary: `schema` (checked `SchemaVersion`, private inner value,
  `0` impossible to construct), `digest` (`Sha256Digest`, 64 lowercase hex,
  validated on parse and on deserialization), `artifact` (`ArtifactId` +
  `Artifact`), `catalog` (`Catalog` + `CatalogEntry` over the existing
  `EntryKind`), `diagnostic` (`Severity`, `Diagnostic`, `Envelope<T>`), and
  `snapshot` (`Snapshot` pinning one `Catalog`). Every record derives
  `deny_unknown_fields`, so an unexpected field fails closed instead of being
  ignored, and every `from_json` compares the declared version against the one
  this build implements.
- 2026-10-29: The stdout envelope is now the shared typed `Envelope<T>` instead of
  three ad-hoc local structs, so `version --json` and every future command emit
  one shape. `STDOUT_SCHEMA_VERSION` moved from `u32 = 1` to `SchemaVersion::V2`
  because `diagnostics` and `artifacts` widened from bare strings to typed
  records. The version command emits both arrays empty, so its bytes are
  unchanged, but the contract is not.
- 2026-10-29: Project and pinned-release *manifest* contracts are deliberately not
  defined here; `UNI-41` owns them. This slice stops at catalogues, artifacts,
  diagnostics, snapshots, and the envelope, which is what the ACs name.
- 2026-10-29: DoD#1 is enforced, not asserted. `scripts/crate_check.py` reads
  `cargo metadata --locked`, walks `universe-core`'s normal/build
  closure (dev edges excluded, since they never reach the published contract), and
  fails closed against an allowlist of serialization and error-derive crates. It
  also rejects any dependency on a workspace crate, and runs as `crate-check`
  inside `check`, so `gates` and `offline-gates` both enforce it. Proven to bite:
  adding `universe-assets` reports the workspace-crate violation and the
  transitive `include_dir`/`include_dir_macros` it drags in.
- 2026-10-29: Correction. `crate-check` originally passed `--offline`, which only
  worked while some earlier task had already warmed the cargo cache; once the gate
  tasks overlapped, or the cache was cold, it failed on a graph it could not read —
  not on a boundary it had judged. The flag was removed (`--locked` stays), and a
  test now pins that distinction so the check cannot quietly become an offline
  assertion again.
- 2026-10-29: AC#3 at merge time was met with committed, behaviour-named fixtures
  compiled into the test binary via `include_str!` plus a deterministic
  permutation-invariance test, because property tests were blocked: the offline
  vendor tree carries only the locked dependency set and had no `proptest`.
- 2026-10-29: That gap is now enrolled. `proptest` is pinned in
  `[workspace.dependencies]` and declared as a dev-dependency of `universe-core`,
  so it reaches `Cargo.lock` and the vendored transport through the relock
  workflow rather than a hand-written lock. The other documented test crates
  (`rstest*`, `wiremock`, `assert_cmd`, `predicates`, `tempfile`) are pinned
  centrally but stay unenrolled until the task whose acceptance criteria name
  them. Writing the property tests and their committed `proptest-regressions/`
  seeds is the follow-up this enrollment unblocks (doc-204).
<!-- SECTION:NOTES:END -->
