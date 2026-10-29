---
id: doc-102
title: Product architecture and bundle model
type: specification
created_date: '2026-10-25T23:49:00Z'
updated_date: '2026-10-27T09:20:00Z'
---

# Product architecture and bundle model

**Status: agreed architectural direction.** The neutral vocabulary is implemented in `universe-core` (UNI-30): checked schema versions, artifact identity and digests, software/data/model/skill catalogues, diagnostics, the typed stdout envelope, and snapshots that pin one catalogue. The pipeline below — resolution, fetch, CAS, views, sealing — is still unimplemented, and project/release manifest contracts belong to UNI-41.

## Product contract

Universe Airlock exports a finite, immutable dependency universe rather than an installed environment or an entire public registry. It supports two different operations:

- **Locked replay:** retain selected artifacts, native identities, dependency graphs, environment membership, features/extras, and relevant native-client options from supported locked inputs.
- **Curated additions:** admit explicitly resolved package roots and their closures, allowing new offline additions only when a compatible graph exists within the sealed universe.

A broad version range produces a concrete resolution per declared scenario, not every upstream version. A bundle does not promise arbitrary future packages, all root combinations, or unverified feature/peer/target combinations. Missing packages and unsupported inputs fail locally, never through upstream fallback.

## Pipeline and capability boundaries

1. Discover supported manifests, locks, original source identities, selected environments, and target profiles.
2. Resolve scenarios using Rattler for conda and pinned native clients for their own ecosystems.
3. Freeze exact resolutions, source metadata, integrity evidence, and acquisition limits into a plan.
4. Fetch original bytes with bounded concurrency; verify native upstream integrity and local SHA-256.
5. Commit verified objects into an immutable content-addressed store.
6. Produce filtered metadata/views and explicitly versioned native-cache derivatives.
7. Verify graph equivalence and native-client scenarios under the required isolation policy.
8. Seal/publish verified data; advance a stable bootstrap reference only after its payload is complete.

Fetching must not silently re-resolve or upgrade a frozen plan. Acquisition does not execute arbitrary package lifecycle scripts or build scripts. A read-only HTTP view serves verified blobs and descriptors. In sealed mode it has no remote acquisition capability. In approved-Git-connected mode a separate bounded loader may fill misses only from digest-pinned published Git objects; it is not an online resolver or arbitrary public-registry fetcher. The native client can resolve additions against the published finite catalog without duplicating its solver in Universe Airlock.

## Canonical data and derived data

Canonical content consists of original archives/assets plus the original authoritative metadata used to resolve them. Do not repack `.conda`, `.tar.bz2`, `.tgz`, `.crate`, or `.whl` archives merely to transport them. Outer compression/shards may change transport representation, but reconstruction must recover the exact original bytes.

Native npm caches, generated local registry indexes, URL/path relocation, and installed target environments are derived state. Their compatibility can depend on exact native-client versions. They are not a substitute for canonical source artifacts.

A package identity includes ecosystem, original source/channel, native name/version, build/variant, and digest. Equal bytes can deduplicate in storage without erasing distinct source/trust identities. Keep different conda channels/labels and different registries logically separate.

## Target and scenario profiles

Record OS/architecture, libc and CPU baseline, relevant system/virtual packages, Python/wheel ABI, Node/npm fingerprint, and Cargo/rustc host/target information where required. Do not solve against the exporter's newer capabilities by accident.

Retain each scenario's input hashes, roots, graph nodes/edges, native options, and admitted artifact set. Unioning blobs across scenarios must not imply that every combination of roots is supported. A conda-only profile does not need irrelevant Cargo or Node tools.

## Proposed record families, not a frozen schema

| Record | Responsibility |
|---|---|
| Source | Credential-free endpoint, ecosystem, source ID, trust policy |
| Artifact/package | Native identity, original filename, size, SHA-256, upstream integrity, metadata references |
| Resolution profile | Target, selected project/environment/scenario, roots, graph, client fingerprint |
| Derived payload | Native-client/helper fingerprint, logical cache origin, recipe, supported scenarios, evidence |
| Inventory | Listed payload paths, sizes/hashes, object/view references |
| Bundle envelope | Schema/version, inventory commitment, profiles, provenance/trust data |

A SHA-256 CAS, small manifest, inventory, source/profile records, original inputs/metadata, and generated views are the working model. Exact filenames, JSON schema, signing envelope, and protocol version remain to be finalized and tested; removed standalone examples are not an implemented public format.

## Integrity and state invariants

- Every advertised installable artifact exists and verifies; incomplete views never become visible.
- Preserve original input bytes and metadata separately from transformations. Use typed relocation and graph comparison, never blanket string replacement.
- Reject unsupported lock schemas, secret-bearing original inputs, checksum drift, unsafe/duplicate paths, extraction bombs, and unlisted payload files.
- A new local SHA-256 does not authenticate an untrusted download; preserve upstream evidence and distinguish publisher authentication from checksum consistency.
- Signature trust must come from an operator-approved key/publisher, not from a key bundled beside the payload. Ed25519 is a candidate, not a frozen protocol choice.
- Imports and activation are transactional: verified data first, active state last; interrupted work must not expose partial objects or indexes.
- Installed environments and writable caches must not mutate canonical blobs through hardlinks. Use safe copy/reflink boundaries and cross-filesystem fallbacks.
- Source code is not included implicitly; any explicit source transfer must exclude secrets/unapproved paths and preserve declared workspace boundaries.

## Lazy catalog refinement

The accepted metadata-first execution/cross-catalog snapshot model is detailed in [Git-backed catalogs](doc-109%20-%20Git-backed-catalogs-and-on-demand-command-execution.md). Original package metadata/bytes remain canonical; bootstrap readiness and complete offline closure are different states.
