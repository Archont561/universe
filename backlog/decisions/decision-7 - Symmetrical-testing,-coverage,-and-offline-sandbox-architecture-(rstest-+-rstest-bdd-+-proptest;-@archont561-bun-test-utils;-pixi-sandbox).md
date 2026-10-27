---
id: decision-7
title: Symmetrical testing, coverage, and offline sandbox architecture (rstest + rstest-bdd + proptest; @archont561/bun-test-utils; pixi-sandbox)
date: '2026-10-26 20:15'
status: accepted
---

## Context

`universe-kit` replaces ad-hoc per-repo Python scripts with a long-lived compiler/publisher/channel-indexer toolchain. Subtle bugs in DAG topological sorting, CEP-15 `repodata.json` determinism, `goblin` binary RPATH auditing, or relative subpath resolution in the SvelteKit portal would break downstream Conda channels silently. Both the Rust workspace and the Bun/SvelteKit workspace require symmetrical unit, BDD, property-based, snapshot, and isolated network/browser testing—and must execute reproducibly inside offline `pixi-sandbox` environments.

## Decision

1. **Rust testing & security stack (`crates/*`)**:
   - Runner: `cargo nextest run --workspace --all-targets` + `cargo test --workspace --doc` (because `cargo-nextest` does not run doctests).
   - Parametrized & fixture tests: `rstest = "0.27"` and `rstest-reuse = "0.7"`.
   - Gherkin BDD: `rstest-bdd = "=0.6.0"` and `rstest-bdd-macros = { version = "=0.6.0", features = ["strict-compile-time-validation"] }` (exact version lock required between runtime and proc-macro crates; Workspace MSRV >= `1.88`) running `crates/*/tests/features/*.feature`.
   - Property-based testing: `proptest = "1.12"` with `proptest-regressions/` committed to Git (covering topological build levels, byte-identical `repodata.json` independent of fragment input permutation, and `spdx` / version constraint roundtrips).
   - HTTP & CLI harnesses: `wiremock = "0.6"` (mocking GitHub Releases/GraphQL, PyPI, `crates.io`, and `prefix.dev`), `assert_cmd = "2.0"`, and synthetic minimal ELF64/PE32+/Mach-O byte builders for `goblin` auditor fixtures.
   - Coverage & supply-chain gates: `cargo llvm-cov nextest --workspace --lcov --output-path target/coverage/rust.lcov`, `cargo audit`, and `cargo deny check`.
2. **Bun / TypeScript / SvelteKit testing stack (`scripts/*`, `packages/*`)**:
   - Standardize on `@archont561/bun-test-utils = "^0.1.0"` (`https://github.com/Archont561/bun-test-utils`):
     - Core `test.extend()` fixtures (`tempDir`, `networkGuard`, `clock`, `spyLogger`).
     - `@archont561/bun-test-utils/bdd` (`Scenario` / `Given` / `When` / `Then` fluent BDD).
     - `@archont561/bun-test-utils/pbt` (`test.prop` + `fc` `fast-check` property tests on install snippet quoting, channel search filtering, and subpath URL joining).
     - `@archont561/bun-test-utils/snap` (`snapshot` inline/file snapshots for rendered install snippets and diagnostics).
     - `@archont561/bun-test-utils/vcr` (deterministic HTTP fixture replay where needed).
     - `happy-dom` preload (`packages/portal/tests/preload.ts`) for fast DOM unit tests + Playwright `webPage` (`packages/portal/e2e/*.e2e.ts`) mounting the built SSG output under a non-root prefix (`/my-org-universe/`) with `networkGuard` blocking all non-localhost requests (proving zero leaked root `/assets/...` requests and zero external CDN calls).
   - Coverage: portal/package tests emit LCOV when that package exists; repository Python scripts emit LCOV through pytest-cov, and Rust emits `target/coverage/lcov.info` through `cargo llvm-cov`.
3. **Browser feature layer & `pixi-sandbox` offline contract**:
   - Declare Playwright/Chromium dependencies in a dedicated `[feature.browser]` (`[environments] browser = ["browser"]`) in `pixi.toml` (`PLAYWRIGHT_SKIP_BROWSER_DOWNLOAD = "1"` pointing at Pixi's `playwright-chromium`), mirroring `Archont561/bun-test-utils`.
   - Configure `pixi-sandbox.toml` (`schema_version = 1`, `[rust] cargo_vendor = true`) with explicit `push_paths` and `relock_paths` covering `pixi.toml`, `pixi.lock`, `Cargo.toml`, `Cargo.lock`, `package.json`, `bun.lock`, `crates/**/Cargo.toml`, `crates/package.json`, and `packages/*/package.json`.

## Consequences

- Every subsystem has compile-time-checked BDD specs and property tests guarding invariants before v0.1.0 ships.
- Subpath asset routing bugs and accidental CDN leaks in `@universe/portal` fail `e2e` automatically under `networkGuard`.
- `relock.yml` and `publish-sandbox.yml` keep offline sandbox packs synchronized whenever any Rust, Bun, or Pixi manifest changes.
- **Update 2026-10-27**: `publish-sandbox.yml` now triggers on pushes that change only the root `Cargo.toml` or `pixi.toml` (mirrored by `[workflow].push_paths` in `pixi-sandbox.toml`). Lockfiles, `package.json` files, `bun.lock`, and `pixi-sandbox.toml` no longer republish the transport automatically — rebuild on demand via `workflow_dispatch`. `relock.yml` is unaffected (`relock_paths` stays broad).
