---
id: UNI-15
title: Author 4-platform crates/universe-kit/recipe/recipe.yaml and finalize headless release.yml staging
status: To Do
assignee: []
created_date: '2026-10-26 20:15'
updated_date: '2026-10-27 13:40'
labels:
  - release
  - ci
milestone: m-4
dependencies:
  - UNI-28
  - UNI-13
documentation:
  - backlog/docs/architecture/doc-1 - Monorepo-and-toolchain-architecture.md
  - backlog/decisions/decision-1 - Two-tier-product-and-channel-boundary-(archont561-archont561-ships-universe-kit;-consumer-universes-target-arbitrary-channels).md
  - backlog/decisions/decision-3 - Hybrid-Rattler-library-+-subprocess-architecture-and-pure-Rust-rustls-dependency-policy.md
  - backlog/decisions/decision-8 - Owner-lane-vs-consumer-lane-GitHub-Actions-workflows-and-zero-checkout-OIDC-publishing.md
  - backlog/decisions/decision-11 - Headless-v0.1.0-first,-browser-portal-deferred-to-v0.2.0.md
priority: high
type: feature
ordinal: 15000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Finalize the self-packaging Conda recipe and 4-platform release pipeline for `universe-kit` (`doc-1`, `decision-1`, `decision-3`, `decision-8`):
- Author `crates/universe-kit/recipe/recipe.yaml` building `universe-kit` with `cargo auditable` / `cargo install --locked --path crates/universe-kit`, declaring `rattler-build >=0.58,<0.65` in `requirements.run`, and running `universe-kit --version` + `universe-kit init --help` + `universe-kit pages --help` in `tests:`.
- Stage `v0.1.0` headless per decision-11: no release job runs the SvelteKit toolchain, so `packages/portal/build/` is absent and `crates/universe-assets/build.rs` falls back to the committed offline placeholder in `crates/universe-assets/placeholder/`. Point the recipe's portal source entry at that placeholder directory (or remove the entry for v0.1.0 and restore it in `UNI-26`) so all four `rattler-build` jobs embed identical placeholder bytes without Bun on the macOS/Windows runners. The real SSG build and `UNIVERSE_KIT_PORTAL_DIST` staging are restored in `UNI-26` for `v0.2.0`.
- Audit `universe-kit`'s own `.conda` package using `universe-kit audit` inside `ci.yml` (`package` job) and `release.yml` (`build` job) — self-hosting binary/SPDX/license verification!
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 `rattler-build build --recipe crates/universe-kit/recipe/recipe.yaml` succeeds in CI, declares `rattler-build` in `requirements.run`, and passes `universe-kit audit` on its own `.conda` artifact.
- [ ] #2 `release.yml` supports `existing_run_id` artifact reuse so transient registry errors can be retried without rebuilding 4-platform binaries.
- [ ] #3 `pixi run repo-check` enforces exact version parity between `Cargo.toml`, `pixi.toml`, and `crates/universe-kit/recipe/recipe.yaml` once the recipe exists.
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 `ci.yml` `package` job builds, tests, and self-audits `universe-kit-*.conda` cleanly.
<!-- DOD:END -->
