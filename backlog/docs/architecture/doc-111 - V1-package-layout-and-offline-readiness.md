---
id: DOC-111
title: V1 package layout, release artifacts, and offline readiness
status: Accepted target architecture
created_date: '2026-10-27'
updated_date: '2026-10-27'
---

## Purpose and scope

This document records the accepted integrated v1 target from decision-119 and distinguishes it from the repository's current implementation. It updates the *planned package layout*; it does not claim that every adapter, registry endpoint, release workflow, or offline test described below already exists.

The v1 acceptance scope is Conda, Rust crates, Python wheels, npm packages, Airlock consumption, a browsable portal, and static Conda publishing. The portal is generated from a verified/pinned catalog and release data; it is not a registry. The former headless-`v0.1.0` / portal-`v0.2.0` staging is retired.

## Target repository/package layout

```text
crates/
  universe-core/                 neutral identity, catalog, integrity, policy, snapshot contracts
  universe-adapters/
    conda/                       Conda metadata, channel paths, repodata and package view
    cargo/                       sparse index and .crate protocol surface
    python/                      wheel metadata and simple-index surface
    npm/                         npm metadata and tarball protocol surface
  universe-kit/                  producer orchestration; existing Conda component recipe
  universe-airlock/              consumer/runtime; existing independent component recipe
  universe-airlock-bootstrap/    separate init/restore bootstrap artifact
  universe-cli/                  thin `universe` command dispatcher only
  universe-assets/               generated/embedded static portal and documentation assets
packages/
  portal/                        portal source, SSG build and tests
  ...                             existing schema/config packages where they remain appropriate
```

The adapter directory may be implemented as separate crates or modules while contracts stabilize, but ecosystem code must remain dependency-injected behind `universe-core` contracts; core must not acquire protocol-specific behavior. Do not combine Kit and Airlock implementation into one crate. Do not make `universe-cli` an alternate implementation of either component.

The user-facing Conda package layout is:

| Conda package | Contents / role |
|---|---|
| `universe` | Thin `universe` dispatcher executable; runtime-depends on `universe-kit` and `universe-airlock`. It is the convenient umbrella install. |
| `universe-kit` | Independently installable producer CLI and its required runtime assets. |
| `universe-airlock` | Independently installable consumer CLI/runtime. |
| `universe-airlock-bootstrap` | Separate bootstrap/download artifact; not folded into Kit or the umbrella package by default. |

The facade routes `universe airlock …` to the Airlock executable. Other commands are routed to Kit, with an optional explicit `universe kit …` spelling. It forwards arguments and exit status and owns no catalog, registry, build, or Airlock logic. Component binaries remain usable without the facade.

Every package keeps its own recipe near its implementation and its own version policy. Preserve per-package `VERSIONS` files where the repository already uses them. The workspace's current shared Rust version is an implementation baseline, not a reason to couple future release/version policies permanently.

## Producer and release artifact flow

1. The existing versioned `universe.toml` project manifest (with per-package `package.toml`) declares package identity, native source/build/test commands, artifact collection rules, target platforms, and registry metadata. Commands invoke external native builders; Kit does not compile Conda packages, wheels, crates, or npm tarballs itself.
2. CI or the owner release workflow runs the declared commands in a controlled job, records their exact commands/tool versions, and attaches artifacts plus a machine-readable release manifest and checksums to an immutable GitHub Release.
3. Kit verifies the pinned repository/tag/release identity, manifest schema, asset names, sizes, and digests before importing artifacts. It never substitutes a moving `latest` URL for a pinned release.
4. Native ecosystem adapters generate the protocol-specific indexes and static views. Conda output is published in standard channel `/<subdir>/repodata.json` and package paths, including `noarch` as appropriate. Cargo receives a sparse index and `.crate` archives; Python receives a Simple API index and wheel files; npm receives registry metadata and tarballs.
5. A catalog snapshot records the verified release identity and content digests. The static portal is built from that snapshot and verified release data. Binary artifacts are stored in static hosting/object storage or immutable release assets, never committed into Git history.
6. Publication has a connected acquisition/publish phase and a deterministic local build/validation phase. A portal page or GitHub Release download link alone is not a native package registry.

## Current offline-development assessment

**The repository's offline gates are genuinely offline for the checks they include, but the full gate suite is not fully offline.** `pixi.toml`'s `offline-gates` task runs repository/Rust/Python checks, coverage, actionlint, and builds without Bun installation or advisory database updates. `lefthook.yml` invokes it with `CARGO_NET_OFFLINE=true` and Pixi offline mode. The broader `gates` task also includes Bun `web-check` and Cargo advisory checks (`cargo deny`/`cargo audit`), which can need network access or an updated vulnerability database.

Current toolchain/dependency closure:

| Area | Present closure | Offline status / missing proof |
|---|---|---|
| Rust | `Cargo.lock` pins these 15 registry packages: `include_dir`, `include_dir_macros`, `itoa`, `memchr`, `proc-macro2`, `quote`, `serde`, `serde_core`, `serde_derive`, `serde_json`, `syn`, `thiserror`, `thiserror-impl`, `unicode-ident`, and `zmij`. A published Pixi sandbox bundle previously included Cargo vendor sources. | A lockfile pins versions but does not contain sources. Run `cargo check/test --locked --offline` and the actual pre-push task against a restored current sandbox before claiming local closure. Any new adapter dependencies require lock/vendor refresh and sandbox republication. |
| Pixi | `pixi.lock` pins the developer environment. | Lock resolution is not the same as retaining every conda package archive. A clean machine needs the required package cache or a restored/published sandbox. |
| Bun/npm workspace tooling | `package.json` and `bun.lock` pin JS workspace resolution. | The current offline gate does not run Bun installs. Bun/npm package tarballs are not included in Cargo vendor; populate/cache them and add a true offline install/build/test gate before asserting offline JS tests. |
| Portal browser tests | Playwright-related tests/tools exist in the project plan/tooling. | Browser executable payloads are separate downloads from npm package metadata. Verify the exact browser/runtime cache and run browser tests with network disabled; these are not proved by `bun.lock`. |
| Rust advisories | `cargo audit`/`cargo deny` are in the connected gate. | Advisory database updates are intentionally excluded from offline gates. CI needs a current database or an explicit pinned database snapshot for offline reproduction. |
| Conda native artifacts | No arbitrary project channel/package cache is implied by the developer sandbox. | A sealed Conda test needs fixture `.conda` packages, repodata, and any required platform dependencies cached locally. `pixi.lock` alone is insufficient. |
| Python wheels | No PyPI wheel/index cache is established for arbitrary package projects. | Vendor/cache wheel files and Simple API metadata for fixtures; prohibit fallback to PyPI in sealed tests. |
| Cargo registry fixtures | Cargo vendor currently covers the Rust workspace's locked dependencies, not future Universe user registries. | Test Cargo adapter behavior with a local sparse index and `.crate` fixtures; explicitly fail on network fallback. |
| npm registry fixtures | Not in Cargo vendor and not currently covered by offline gates. | Cache npm metadata/tarballs and test `npm`/Bun clients against a local fixture registry with network disabled. |
| GitHub release data | Network required to discover releases unless a snapshot is provided. | Include pinned, signed/hashed release-manifest and catalog fixtures in deterministic tests; connected release discovery/publishing remains separately classified. |

Thus: existing offline hooks protect a useful development subset, not the complete future product test matrix. No claim of “all tests fully offline” is warranted yet.

## Lockfiles and artifacts to refresh

- **`Cargo.lock`**: refresh when Rust workspace membership or any Cargo dependency changes. For new registry dependencies, regenerate with the chosen toolchain, run locked/offline resolution, rebuild `.pixi-sandbox/vendor`, publish the updated sandbox, then restore and verify from a clean offline state. Workspace-member-only additions can still alter the lockfile even with no new external crate.
- **`pixi.lock`**: refresh when `pixi.toml` environments, tasks that introduce tools, or dependency specifications change. Recreate/verify the developer sandbox after relevant changes.
- **`bun.lock`**: refresh whenever the root or workspace `package.json` files or Bun dependency declarations change. Then cache/fetch package archives and browser payloads separately if the offline gate depends on them.
- **Conda environment locks/fixtures**: pin and refresh environment lock data for product fixture environments and retain package archives/repodata if sealed tests require Conda resolution. This is distinct from `pixi.lock`.
- **Catalog/release snapshots and checksums**: refresh when pinned upstream releases, schemas, or artifact assets change; these are data snapshots/manifests, not Cargo/Pixi/Bun lockfiles.
- **Docs/backlog-only edits**: no package lock or vendor refresh is required.

For this change set, `universe-cli` is a new workspace member with no third-party dependencies; `Cargo.lock` now contains its workspace package entry and the locked registry closure remains 15 packages. `pixi.lock` and `bun.lock` do not need changes. No new Cargo vendor source is required; the existing `crates/**/Cargo.toml` publisher/relock path patterns already include the new member. Verify the lock with `cargo metadata --locked`/`cargo check --locked --offline` when the Rust toolchain and sandbox are available.

## Next implementation slices

The v1 implementation and release gate are tracked in UNI-40 through UNI-47 plus UNI-16; post-v1 native developer transport and sandbox retirement are UNI-48/UNI-27. The ordered task graph and undated sprint train are in roadmap doc-112. Milestones describe outcomes; tasks remain flat and dependencies name only real blockers. The release gate combines deterministic offline fixture tests with separately identified connected release/publish checks.
