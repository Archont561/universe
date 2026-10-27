---
id: m-1
title: "M1 — Neutral schemas and adapter contracts"
---

## Description

Define authoritative ecosystem-neutral Rust models, adapter conformance contracts,
and the versioned multi-ecosystem project/release manifest used by producer,
registry, Airlock, and portal work.

## Tasks

- `UNI-30` — authoritative neutral schemas.
- `UNI-31` — adapter boundary and conformance fixtures (`UNI-30` blocker).
- `UNI-41` — multi-ecosystem project and pinned release manifest (`UNI-30` and `UNI-31` blockers).

## Exit condition

Manifest and release metadata fixtures are deterministic; identity, digest, safety,
and compatibility rules are explicit; ecosystem-specific protocol implementation
remains outside `universe-core`. These contracts unblock the producer, adapters,
Airlock, and portal data pipelines.