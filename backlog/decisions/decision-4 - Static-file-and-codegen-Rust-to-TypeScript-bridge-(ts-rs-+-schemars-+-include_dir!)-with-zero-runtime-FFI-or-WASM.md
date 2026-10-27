---
id: decision-4
title: Static file-and-codegen Rust-to-TypeScript bridge (ts-rs + schemars + include_dir!) with zero runtime FFI or WASM
date: '2026-10-26 20:15'
status: accepted
---

## Context

`universe-kit` authors channel metadata, audit reports, and catalog payloads in Rust (`crates/universe-core`) and renders them in a pre-built SvelteKit SSG frontend (`packages/portal`) embedded inside the `universe-kit` CLI binary. We evaluated how Rust and TypeScript should share types and assets without introducing N-API native addons, WASM build toolchains, or circular Turbo build dependencies (`Rust -> TS -> SvelteKit -> Rust`).

## Decision

1. **Rust owns all domain schemas**: `crates/universe-core/src/schema/` defines the canonical structs (`UniverseConfig`, `PackageMeta`, `FragmentJson`, `CatalogJson`, `PackageDetailJson`, `AuditSummary`, `InstallInstructions`) deriving `serde::{Serialize, Deserialize}`, `ts_rs::TS` (`ts-rs = "12.0"`), and `schemars::JsonSchema` (`schemars = "1.2"`).
2. **Committed generated bindings with CI drift gate**:
   - `packages/schema/src/generated/*.ts` and `schemas/{universe,package}.schema.json` are generated from `universe-core` and committed to Git.
   - Committing generated files keeps the build graph strictly linear and acyclic: `@universe/schema` -> `@universe/portal` -> `@universe/rust`.
   - `cargo nextest` + `pixi run schema-check` verify in CI (`pixi run ci`) that freshly exported `ts-rs` and `schemars` outputs match committed files byte-for-byte (`git diff --exit-code packages/schema/src/generated schemas/`).
3. **Isolated asset embedding crate (`crates/universe-assets`)**:
   - `crates/universe-assets` contains only `include_dir!("$CARGO_MANIFEST_DIR/../../packages/portal/build")` (plus embedded public `skills/universe-kit/` templates) and a `build.rs` emitting `cargo:rerun-if-changed=../../packages/portal/build`.
   - Isolating `include_dir!` in `universe-assets` ensures editing Svelte/UnoCSS UI files never triggers recompilation of `universe-core`.
4. **Zero runtime FFI or WASM**: Rust and SvelteKit communicate strictly via generated static `.json` files (`catalog.json`, `packages/<name>/package.json`) and inline `<script id="cu-pkg-data" type="application/json">` hydration payloads injected via `lol_html` during `universe-kit pages build`.

## Consequences

- Rust compiler errors and `bun run typecheck` (`svelte-check`) catch any Rust <-> UI field mismatch before merge.
- Editor autocomplete and validation work out of the box on `universe.toml` and `package.toml` via published JSON Schemas (Taplo / Even Better TOML).
- No `wasm-bindgen`, `wasm-pack`, or `napi-rs` build targets are required on any platform.
