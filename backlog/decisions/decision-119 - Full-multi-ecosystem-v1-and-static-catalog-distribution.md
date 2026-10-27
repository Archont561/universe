---
id: DECISION-119
title: Full multi-ecosystem v1 with a thin Universe facade and static catalog distribution
status: accepted
created_date: '2026-10-27'
updated_date: '2026-10-27'
---

## Context

The earlier release plan put a headless Conda producer in `v0.1.0`, deferred the browser portal to `v0.2.0`, and considered Cargo support later. That sequencing no longer matches the product scope. The first complete Universe v1 must be useful across the producer, consumer, and discovery surfaces together.

## Decision

The first stable product release is an integrated v1 comprising all of the following:

1. Conda packages and a standards-compliant static Conda channel.
2. Rust crates through a Cargo-compatible sparse index and crate archives.
3. Python wheels through a Python-compatible package index. The initial scope is wheels; source distributions/building arbitrary Python projects are not implied.
4. npm packages through an npm-compatible registry surface.
5. Universe Airlock consumption across the supported ecosystems.
6. A browsable static portal generated from a pinned catalog snapshot and verified GitHub Release metadata/assets.

The static portal is a catalog and discovery surface, not a registry. Package clients must receive the protocol-native metadata and artifacts they expect. In particular, Conda clients require package files at standard channel/subdirectory paths; links from a web page to GitHub Releases do not create a usable Conda channel.

Keep implementation ownership separate:

- `universe-core` contains ecosystem-neutral catalog, identity, integrity, policy, and snapshot contracts.
- Ecosystem adapters implement Conda, Cargo, Python-wheel, and npm protocols independently.
- `universe-kit` is the producer/orchestrator. It validates configuration, runs explicitly configured external build/test commands, collects their outputs, verifies and publishes artifacts/indexes, and generates catalog/portal snapshots. It does not implement package compilers/build systems.
- `universe-airlock` is the consumer. It stages native-client-compatible local views and offers explicitly scoped operations; it must not become a universal transparent proxy or silently capture arbitrary HTTP traffic.
- `universe-airlock-bootstrap` remains a separate bootstrap artifact.
- A thin `universe-cli` crate produces the `universe` front-door binary. It dispatches ordinary producer commands to `universe-kit` and `universe airlock …` to `universe-airlock`; it contains no ecosystem business logic. Its Conda package is named `universe` and depends on the separately installable `universe-kit` and `universe-airlock` packages. The component packages remain independently installable.

A release manifest and checksums pin every release's catalog and artifact metadata. Large binaries stay in immutable release assets or static object storage, not in Git. The producer publishes Conda package files and native indexes to a static channel backend in each ecosystem's standard layout. GitHub Release assets may be an origin/transport, but the channel and portal are generated/published from verified, pinned inputs.

The previous staged `v0.1.0` headless / `v0.2.0` portal schedule is superseded. No stable v1 release is complete until all six surfaces above pass end-to-end acceptance.

## Consequences

- Conda remains the initial producer workflow and recipe format, but the Conda-only release is not the final v1 acceptance target.
- Cargo, Python wheels, and npm are first-class v1 targets, not post-v1 exploratory work.
- Generic files, Agent Skills, and STAC are deferred unless separately accepted; they are not included in the v1 adapter budget.
- External native builders remain responsible for producing package artifacts. Kit's contract is command orchestration, collection, validation, indexing, and publication.
- Offline verification is evidence-specific: deterministic fixtures and offline gates must run without network; connected release metadata/artifact acquisition remains a separately identified operation. Pixi/Cargo/Bun lockfiles do not themselves guarantee that every package archive, browser payload, or Conda cache is present offline.
- See `backlog/docs/architecture/doc-111 - V1-package-layout-and-offline-readiness.md` for the package map and offline gap assessment.

## Supersedes

This decision supersedes the release sequencing in decision-11 and narrows/revises the adapter scope in decision-13. The neutral-core/separate-adapter architecture from decision-13 remains in force.
