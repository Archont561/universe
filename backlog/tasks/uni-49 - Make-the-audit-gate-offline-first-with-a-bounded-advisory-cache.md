---
id: UNI-49
title: Make the audit gate offline-first with a bounded advisory cache
status: In Progress
assignee: []
created_date: '2026-10-29'
updated_date: '2026-10-29'
labels:
  - infrastructure
  - offline
  - gates
milestone: m-0
dependencies: []
priority: high
type: chore
ordinal: 49000
---

## Description
<!-- SECTION:DESCRIPTION:BEGIN -->
The `audit` gate previously ran `cargo deny check && cargo audit --deny warnings`
directly. Neither tool carries its own data: both need a RustSec advisory
database, and the yanked check needs a crates.io index cache. In an
egress-denied checkout `cargo audit` fetched its database and then blocked on
`Updating crates.io index` for over seven minutes, so `pixi run gates` hung
instead of failing, and a cached run was indistinguishable from a complete one.

Replace the raw chain with one offline-first gate runner that treats the audit
inputs as bounded caches: offline by default when the caches exist, fail closed
when they are missing and the network is denied, and bootstrap once with bounded
timeouts when it is allowed. See decision-120 for the policy and doc-110 for the
transport boundary this preserves.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 `pixi run audit` completes with network access denied when the advisory caches exist, and no subprocess can exceed its configured timeout.
- [x] #2 A missing advisory cache with denied network fails closed with a non-zero exit and a message naming `pixi run audit-sync`.
- [x] #3 `pixi run audit-sync` seeds both advisory databases and the index cache where the network allows, tolerating an unreachable index.
- [x] #4 Both tools are invoked with explicit offline flags (`cargo deny --offline check`, `cargo audit --no-fetch`), and yanked-check degradation is reported explicitly instead of failing per crate.
- [x] #5 pytest covers cache resolution, argv policy, bootstrap ordering, timeout mapping, and both failure paths.
- [x] #6 `repo-check` rejects an `audit` task that bypasses the gate runner or omits `audit-sync`.
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [x] #1 `pixi run audit` and `pixi run audit-sync` exist, and `pixi run gates` keeps the audit gate without changing the offline gate set.
- [x] #2 decision-120 records the policy, and doc-110 no longer lists the audit gate as unprovable offline.
- [x] #3 `pixi run gates` passes.
<!-- DOD:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
- 2026-10-29: `scripts/audit.py` owns the gate. `check` resolves the single
  cargo-deny advisory clone under `CARGO_HOME` and runs `cargo deny --offline
  check` plus `cargo audit --no-fetch --db <clone> --deny warnings`; it refuses
  to start either tool when the clone is absent and
  `CARGO_NET_OFFLINE`/`--offline` denies the network. `sync` runs `cargo deny
  fetch db` (required) then `cargo deny fetch index` (best effort, presence
  checked rather than trusting the exit status, because that command can succeed
  without writing anything).
- 2026-10-29: Sharing one clone matters. A bare `cargo audit` re-enables its own
  database fetch and then blocks on `Updating crates.io index`; pinning `--db`
  and `--no-fetch` removes that path entirely, and deleting cargo-audit's own
  `$CARGO_HOME/advisory-db` no longer affects the gate.
- 2026-10-29: Measured in the restored sandbox, cargo-audit's own database
  removed. Cached and network-denied: exit 0 in <1s (the whole gate; the same
  command previously ran past 420s without finishing). Cold cache and
  network-denied: exit 2 in <1s with the `audit-sync` message. Cold cache with
  network allowed (the CI path): exit 0 in ~1s after seeding. The yanked check
  reports "yanked enforcement degraded (15 check(s) skipped)" because the
  transport carries no crates.io index cache; that is decision-120's stated
  limitation, not a silent pass.
- 2026-10-29: `repo_check.validate_audit_gate` pins the task wiring, mirroring the
  existing pre-push/vendor guard, so a future edit cannot quietly restore the
  hanging command or drop the cache-seeding task.
<!-- SECTION:NOTES:END -->
