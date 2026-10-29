---
id: UNI-36
title: Establish offline developer transport
status: Done
assignee: []
created_date: '2026-10-27'
updated_date: '2026-10-29'
labels:
  - bootstrap
  - infrastructure
  - offline
milestone: m-0
dependencies:
  - UNI-28
priority: medium
type: chore
ordinal: 36000
---

## Description
<!-- SECTION:DESCRIPTION:BEGIN -->
Choose and implement a reproducible transport for materialising the locked
Pixi/Rust/Python/Cargo developer environment under a documented network
boundary. Evaluate a separately published, platform-qualified bundle containing
the default Pixi environment and a Cargo vendor tree for the workspace's
locked registry dependencies, with a native restore launcher that fetches only
the selected transport ref and restores locally. Keep transport publication manual and fail-closed by default; support
explicit no-fetch and no-user-tool-registration modes so restore does not
silently alter profiles or require public package registries after the bundle
is available. Do not treat this developer transport as an Airlock sealed
snapshot or as proof that npm, advisories, arbitrary tools, or native host
prerequisites are included. Generated launcher/configuration must be
reproducible and tested.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 A clean Linux x86_64 checkout can materialise the locked environment under the documented network boundary, or the limitation is explicitly accepted in a decision.
- [x] #2 The reviewed bundle config and publisher include the default Pixi environment, the complete locked Cargo dependency graph (vendoring registry crates when present), and republish triggers for root/crate Cargo manifests, Cargo.lock, pixi.toml, and pixi.lock.
- [x] #3 The checked-in restore launcher verifies the published branch before restore, supports a no-profile-edit mode, and has shell/config regression coverage.
- [x] #4 The transport choice states how it relates to UNI-27 and the Airlock bootstrap boundary.
- [x] #5 The pre-push hook runs `CARGO_NET_OFFLINE=true pixi run --offline --as-is offline-gates`; a freshly restored vendor-enabled environment passes without network access. Bun installation and advisory-database updates remain in the full CI gate, not the offline hook.
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [x] #1 The chosen transport proves `CARGO_NET_OFFLINE=true pixi run --offline --as-is offline-gates` from a freshly restored vendor-enabled environment with `target` cleared; the complete `pixi run gates` suite still passes in CI.
<!-- DOD:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
- 2026-10-27: Diagnosed the pre-push network failure. `.pixi-sandbox.toml`
  requests Cargo vendoring, but the active publisher passed `--cargo-vendor
  false`; the restored tree consequently had no vendor directory, and Cargo
  tried to update crates.io. The publisher now enables vendoring, repo-check
  guards the config/workflow/hook agreement. Pre-push now targets a dedicated
  offline-compatible gate set (Rust, Python, and repository checks); Bun install
  and advisory database updates remain in the complete CI gate because the
  transport explicitly does not bundle those inputs. The old restored bundle
  failed offline Cargo resolution with `no matching package named serde found`.
- 2026-10-27: The merged publisher run completed successfully and published
  sandbox snapshot `74b5d91` with 15 vendored crates (5.9 MiB). A verified fresh
  restore included `.pixi-sandbox/vendor` and configured the sandbox Cargo home.
  After clearing `target`, `CARGO_NET_OFFLINE=true pixi run --offline --as-is
  offline-gates` passed. `repo-check` now skips generated `.pixi-sandbox`
  contents so vendor license files are not mistaken for stale root-level
  license files. Other transport acceptance criteria remain open.
- 2026-10-29 (AC#1): Materialised the locked environment from a clean checkout in
  an egress-limited Linux x86_64 sandbox. `bash scripts/restore.sh` fetched only
  `sandbox/developer-linux-64` (snapshot `2026-10-27T22:00:53Z`, commit
  `bf724d5`, schema 5), verified 706 blobs / 699.2 MiB with 0 failures, materialised
  the `default` environment (13 816 entries, 2 371.7 MiB), and staged 15 vendored
  crates. `pixi install --locked` then reported a no-op, and `pixi run
  offline-gates` plus `pixi run web-check` passed. The documented boundary held:
  no conda channel, crates.io, npm, or RustSec host was needed for the restore
  itself. The one recorded limitation is that the advisory and npm payloads stay
  outside the transport (doc-110, decision-120), so the audit gate needs one
  `pixi run audit-sync` before it can run offline.
- 2026-10-29 (AC#2): `.pixi-sandbox.toml` declares `cargo_vendor = true` for the
  root and the `developer` bundle, and its `[workflow].push_paths` lists
  `.pixi-sandbox.toml`, `Cargo.toml`, `Cargo.lock`, `crates/**/Cargo.toml`,
  `pixi.toml`, and `pixi.lock`. The published snapshot carried the complete locked
  registry graph (15 crates, 5.9 MiB, `Cargo.lock` match recorded in the
  manifest). `repo-check` now enforces all three parts: vendoring enabled, the
  publisher passing `--cargo-vendor true`, and every republish trigger present, so
  a config edit that would let the branch go stale fails the gate instead of
  silently shipping an out-of-date transport.
- 2026-10-29 (AC#3): Added `tests/scripts/test_restore_script.py` — 30 cases that
  run the real launcher and cover branch derivation from the reviewed config,
  prefix/bundle renaming, multi-bundle and unknown-bundle refusal, unsupported
  schema, missing config, a host with no published bundle, 14 unsafe ref names,
  explicit-branch bypass, and the no-network/no-writes guarantee of
  `--print-branch`. Two guards pin the ordered contract that the branch is
  verified before anything is written, the restored tree is verified afterwards,
  and the `skip` branch of the user-tools policy performs no profile writes. The
  suite runs inside pytest, so it is part of `check` and reaches the pre-push hook.
  Verify-before-restore and the no-profile-edit mode were also observed directly
  in this session: the launcher printed `verify 706 blob(s) ... 0 failure(s)` before
  writing, and finished with `user tools: none registered (--user-tools skip; the
  project itself is complete)` under `PIXI_SANDBOX_USER_TOOLS=skip`.
- 2026-10-29 (AC#4): doc-110 now has a dedicated section separating this
  developer transport from `universe-airlock-bootstrap`/`UNI-33` sealed restore and
  from the `UNI-27` removal gate. decision-117's removal list was stale (it named
  a `utils` feature, `scripts/sandbox.py`, `tests/automation/test_sandbox.py`, and
  `config/sandbox-bootstrap.json`, none of which exist); its status note now points
  at `UNI-27` to re-derive that list from the tree.
- 2026-10-29: m-0's exit condition is met by the above: verified restore inside the
  documented boundary, offline gates green, and Backlog/repository checks valid.
<!-- SECTION:NOTES:END -->
