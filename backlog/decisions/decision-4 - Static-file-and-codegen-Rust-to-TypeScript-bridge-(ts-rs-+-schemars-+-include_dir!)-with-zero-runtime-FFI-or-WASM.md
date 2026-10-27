---
id: decision-4
title: Static file-and-codegen Rust-to-TypeScript bridge (ts-rs + schemars + include_dir!) with zero runtime FFI or WASM
date: '2026-10-26 20:15'
status: accepted
---

## Status note

This is an accepted target architecture, not a description of the current implementation. The current Rust core does not define these portal/domain schemas, there is no schema export/check task, and the TypeScript contracts in `packages/schema` are hand-maintained portal contracts. No generated files are checked in. The M1 milestone explicitly keeps browser/documentation bindings out of the standalone scaffold until a source contract and package integration are implemented.

## Context

The intended product may author channel metadata, audit reports, and catalog payloads in Rust (`crates/universe-core`) and render them in a pre-built SvelteKit SSG frontend (`packages/portal`) embedded inside the CLI binary. This decision records the desired way for Rust and TypeScript to share types and assets without N-API native addons, WASM build toolchains, or circular Turbo build dependencies (`Rust -> TS -> SvelteKit -> Rust`).

## Decision — intended architecture

1. **Rust owns canonical domain schemas when those models are implemented**: define the required structs in the core schema module and derive serialization, TypeScript export, and JSON Schema support.
2. **Committed generated bindings with a drift gate**: generate TypeScript and JSON Schema from the real Rust source, commit outputs, and verify freshly exported output byte-for-byte in CI. Generation must not be represented by hand-authored files or tests that assert provenance without running a generator.
3. **Isolated asset embedding crate**: `crates/universe-assets` is intended to isolate portal asset embedding so UI edits do not trigger recompilation of `universe-core`.
4. **Zero runtime FFI or WASM**: Rust and SvelteKit should communicate through static JSON payloads and HTML hydration rather than runtime bindings.

## Consequences

When implemented, Rust compiler errors and TypeScript type-checking should catch field mismatches before merge, while generated JSON Schemas should support editor validation. No `wasm-bindgen`, `wasm-pack`, or `napi-rs` build targets are required. These guarantees do not hold until the source models, generator, and CI drift check exist.
