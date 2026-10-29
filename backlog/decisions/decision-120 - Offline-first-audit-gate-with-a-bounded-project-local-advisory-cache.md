---
id: decision-120
title: Offline-first audit gate with a bounded, project-local advisory cache
date: '2026-10-29'
status: accepted
---

## Context

`pixi run gates` includes the `audit` task, so the gate contract in decision-15
claims an audit that a restored, egress-denied checkout could not actually prove.
The previous task ran `cargo deny check && cargo audit --deny warnings` directly.
Both tools need external data that the repository does not contain:

- a RustSec advisory database (cargo-audit uses `$CARGO_HOME/advisory-db`, cargo-deny
  uses `$CARGO_HOME/advisory-dbs`);
- a crates.io index cache for the yanked-crate check.

doc-110 records both as inputs the developer transport does not supply. Observed
in an airlocked sandbox: `cargo audit` fetched its advisory database, then blocked
on `Updating crates.io index` for more than seven minutes, because the index host
was unreachable and the call had no bound. The gate therefore hung instead of
failing, and a cached run could not be distinguished from a complete one.

## Decision

- The audit gate is one Pixi task running `python -m scripts.audit check`. The raw
  `cargo deny check && cargo audit` chain is retired, and `repo-check` rejects a
  regression to it.
- Audit inputs are caches under `CARGO_HOME`: project-local after a restore
  (`.pixi-sandbox/cargo-home`), the tool default otherwise. They are never
  committed, and the developer transport does not ship them: a 51 MiB advisory
  database with Git history does not belong in a Git transport.
- One advisory clone serves both tools: `cargo deny fetch db` seeds
  `$CARGO_HOME/advisory-dbs`, and cargo-audit is pinned to that clone with
  `--db <clone>` plus `--no-fetch`. No command ever runs cargo-audit against its
  own database path, so there is exactly one cache to seed and no code path that
  can re-enter the index update that caused the hang.
- `pixi run audit-sync` is the only network path. It seeds or refreshes the
  advisory clone (required) and the index cache (best effort).
- With caches present, the gate denies the network explicitly
  (`cargo deny --offline check`, `cargo audit --no-fetch`) and completes in under
  a second. Every subprocess runs under a hard timeout, so the gate can never
  hang again.
- Without advisory caches, the gate fails closed (exit 2) with an actionable
  message when network access is denied, and bootstraps once with bounded
  timeouts when it is allowed. CI keeps running `pixi run gates` unchanged.
- The yanked check needs the index cache. When it is absent the gate reports one
  explicit degradation notice naming the skipped count and `audit-sync`, and
  still passes. A silent pass is not permitted.

## Consequences

Advisory, bans, licences, and sources checks are enforced offline. Yanked-crate
enforcement is only guaranteed where `audit-sync` could reach the index, and the
gate says so every run. The transport remains unchanged, so a freshly restored
checkout still needs one `audit-sync` (or one online `pixi run audit`) before it
can audit offline. This decision does not make the advisory database part of the
sealed Airlock model, and it does not claim a vulnerability-free dependency
graph: it claims the audit gate runs, bounded, under the documented boundary.
