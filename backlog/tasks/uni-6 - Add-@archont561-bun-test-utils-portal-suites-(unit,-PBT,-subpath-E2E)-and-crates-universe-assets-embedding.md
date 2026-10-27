---
id: UNI-6
title: Add @archont561/bun-test-utils portal suites (unit, PBT, subpath E2E) and crates/universe-assets embedding
status: To Do
assignee: []
created_date: '2026-10-26 20:15'
updated_date: '2026-10-27 19:00'
labels:
  - portal
  - testing
milestone: m-6
dependencies:
  - UNI-5
documentation:
  - backlog/docs/architecture/doc-1 - Monorepo-and-toolchain-architecture.md
  - backlog/docs/architecture/doc-3 - Static-channel-and-SvelteKit-UnoCSS-portal-specification.md
  - backlog/decisions/decision-4 - Static-file-and-codegen-Rust-to-TypeScript-bridge-(ts-rs-+-schemars-+-include_dir!)-with-zero-runtime-FFI-or-WASM.md
  - backlog/decisions/decision-6 - SvelteKit-3-+-Svelte-5-SSG-+-UnoCSS-for-@universe-portal-using-template-route-cloning-and-no-Storybook.md
  - backlog/decisions/decision-7 - Symmetrical-testing,-coverage,-and-offline-sandbox-architecture-(rstest-+-rstest-bdd-+-proptest;-@archont561-bun-test-utils;-pixi-sandbox).md
  - backlog/decisions/decision-119 - Full-multi-ecosystem-v1-and-static-catalog-distribution.md
priority: high
type: feature
ordinal: 6000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
> Portal implementation is part of integrated v1 (decision-119). `crates/universe-assets` and its scaffolded SSG preference/fallback exist; generated-portal tests, deterministic extraction, coverage, and release integration remain incomplete.

Complete the portal testing and asset-embedding loop (`decision-4`, `decision-6`, `decision-7`, `decision-119`). The package layout is scaffolded; this task still owns the functional test and integration work:
- Write `bun test` suites in `packages/portal/tests/` using `@archont561/bun-test-utils` (`./bdd`, `./pbt`, `./snap`, and `happy-dom` preload):
  - Property-based tests (`test.prop` + `fc`) on `install-snippet.svelte.ts`, `catalog-filter.svelte.ts`, and relative subpath joining.
  - Snapshot tests (`snapshot`) on rendered `pixi`/`conda`/`mamba` snippets across multi-channel configurations.
- Write Playwright `webPage` E2E tests in `packages/portal/e2e/subpath.e2e.ts` serving a fixture-hydrated portal under a non-root prefix (`http://127.0.0.1:<port>/org-universe/`) with `networkGuard` blocking all external network traffic and failing on any root `/assets/...` 404 request.
- Implement `crates/universe-assets` using `include_dir!` (supporting `UNIVERSE_KIT_PORTAL_DIST` override in `build.rs` for cross-platform `release.yml` builds) and verify asset extraction in Rust tests.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 `pixi run test` executes all `@universe/portal` unit, BDD, PBT, and snapshot suites with LCOV coverage emitted.
- [ ] #2 `pixi run --environment browser test-e2e` verifies cold load on `/org-universe/`, client-side navigation to `/org-universe/packages/cloudcompare/`, and direct cold reload on `/org-universe/packages/cloudcompare/` with zero console errors, zero 404s, and zero external requests under `networkGuard`.
- [ ] #3 `crates/universe-assets` embeds `packages/portal/build/` and exposes deterministic file enumeration to `universe-kit`.
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 Merged Rust + Bun LCOV report (`target/coverage/lcov.info`) includes `packages/portal/src/lib/**` and `crates/universe-assets/**`.
<!-- DOD:END -->
