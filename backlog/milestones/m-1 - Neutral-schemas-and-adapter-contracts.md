---
id: m-1
title: "M1 — Neutral schemas and adapter contracts"
---

## Description

Define authoritative ecosystem-neutral Rust models, adapter conformance contracts,
and the versioned multi-ecosystem project/release manifest used by producer,
registry, Airlock, and portal work.

## Tasks

- `UNI-30` — authoritative neutral schemas (**Done**; six modules plus the `crate-check` boundary gate).
- `UNI-31` — adapter boundary and conformance fixtures (`UNI-30` blocker).
- `UNI-41` — multi-ecosystem project and pinned release manifest (`UNI-30` and `UNI-31` blockers).
- `UNI-57` — workflow renderer, the manifest's first consumer (`UNI-41` blocker).

## Exit condition

Manifest and release metadata fixtures are deterministic; identity, digest, safety,
and compatibility rules are explicit; ecosystem-specific protocol implementation
remains outside `universe-core`; and one real workflow set renders from the manifest,
so the contract has been exercised rather than merely described. These contracts unblock the producer, adapters,
Airlock, and portal data pipelines.
