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
- `UNI-31` — adapter boundary and conformance fixtures (`UNI-30` blocker). **Done**; six outcomes with one fixture each, and the 90% line gate now covers the rejection paths.
- `UNI-41` — multi-ecosystem project and pinned release manifest (`UNI-30` and `UNI-31` blockers).
- `UNI-57` — workflow renderer, the manifest's first consumer (`UNI-41` blocker).

## In flight (2026-10-31)

`UNI-41` and `UNI-57` are `In Progress`, not done. The manifest contract and the renderer
exist and are tested; what remains is the part that proves them against this repository
rather than against fixtures:

- `UNI-41` — release-manifest fixture matrix, `lint --mode restore`, the migration note for
  the old `[[targets]]` shape, and Universe's own `universe.toml`.
- `UNI-57` — `actionlint` over every workflow variant, the `pixi run ci-render` /
  `ci-check` tasks, and the committed first rendered file, which cannot land before the
  repository has a manifest of its own.
- Scenarios are not yet executable: `rstest-bdd` is declared in the workspace but absent
  from the vendored transport tree, recorded as `draft-8`.

## Exit condition

Manifest and release metadata fixtures are deterministic; identity, digest, safety,
and compatibility rules are explicit; ecosystem-specific protocol implementation
remains outside `universe-core`; and one real workflow set renders from the manifest,
so the contract has been exercised rather than merely described. These contracts unblock the producer, adapters,
Airlock, and portal data pipelines.
