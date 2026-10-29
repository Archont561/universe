---
id: m-7
title: "M7 — Browsable portal from pinned catalog snapshots"
---

## Description

Deliver the human-facing portal as part of integrated v1. The portal is generated from
verified release manifests and a catalog snapshot, and remains distinct from native
registry endpoints. The former portal-only `v0.2.0` sequencing is superseded by decision-119.

## Tasks

- `UNI-5` — SvelteKit/Svelte/UnoCSS static portal (`UNI-30` blocker).
- `UNI-6` — portal test suites and embedded assets (`UNI-5` blocker).
- `UNI-25` — deterministic catalog/package hydration (`UNI-6`, `UNI-12`, `UNI-41` blockers).
- `UNI-26` — release workflow integration (`UNI-25`, `UNI-41` blockers).
- `UNI-46` — render portal from verified pinned releases/catalog (`UNI-5`, `UNI-6`, `UNI-12`, `UNI-25`, `UNI-41`–`UNI-44` blockers).

## Exit condition

Portal output is deterministic from local pinned fixtures; root and subpath browser tests
pass with network guard; the exact catalog/release digest is recorded. Portal links never
substitute for standard Conda/Cargo/Python/npm package endpoints.
