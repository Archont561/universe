---
id: m-1
title: "M1 — Neutral schemas and adapter contracts"
---

## Description

Define authoritative ecosystem-neutral Rust domain models for `universe.toml`,
package/entry manifests, catalogues, fragments, artifacts, diagnostics, snapshots,
and result envelopes. `UNI-30` owns the neutral schema slice; `UNI-31` owns the
adapter boundary and conformance fixtures; `UNI-41` extends the existing project
manifest and defines the pinned multi-ecosystem release-manifest contract.

Generated bindings for browser/documentation packages consume versioned schemas
from Rust/core contracts and are wired through Pixi as explicit package work.
The portal is required in the integrated v1 (M6/M10); the old `v0.2.0`
deferment is superseded by decision-119.

Exit condition: neutral model and manifest validation are deterministic and
fixture-tested, with no ecosystem protocol implementation, UI, or network
behavior leaking into `universe-core`.