---
id: UNI-36
title: Establish offline developer transport
status: In Progress
assignee: []
created_date: '2026-10-27'
updated_date: '2026-10-27'
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
- [ ] #1 A clean Linux x86_64 checkout can materialise the locked environment under the documented network boundary, or the limitation is explicitly accepted in a decision.
- [ ] #2 The reviewed bundle config and publisher include the default Pixi environment, the complete locked Cargo dependency graph (vendoring registry crates when present), and republish triggers for root/crate Cargo manifests, Cargo.lock, pixi.toml, and pixi.lock.
- [ ] #3 The checked-in restore launcher verifies the published branch before restore, supports a no-profile-edit mode, and has shell/config regression coverage.
- [ ] #4 The transport choice states how it relates to UNI-27 and the Airlock bootstrap boundary.
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
<!-- SECTION:NOTES:END -->
