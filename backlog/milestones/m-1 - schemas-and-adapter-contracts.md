---
id: m-1
title: "M1 — Schemas and adapter contracts"
---

## Description

Define the authoritative ecosystem-neutral Rust domain models for
`universe.toml`, package/entry manifests, catalogues, fragments, artifacts,
diagnostics, snapshots, and result envelopes. `UNI-30` owns the neutral
schema slice; `UNI-31` owns the adapter boundary and conformance fixtures.

Generated bindings for a future browser or documentation package are not part
of the standalone scaffold. If such a package is introduced, it must consume
versioned schemas from the Rust/core contract and be wired through Pixi as an
explicit package.

The SvelteKit/UnoCSS portal remains deferred to milestone M6 and `v0.2.0` by
decision-11.

Exit condition: `pixi run gates` proves the schema/adapter contracts with
fixtures while `universe-core` stays free of ecosystem, UI, and network
dependencies.
