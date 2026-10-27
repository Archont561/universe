---
id: doc-105
title: Git payload transport and curated requests
type: specification
created_date: '2026-10-25T23:49:00Z'
updated_date: '2026-10-27T09:20:00Z'
---

# Git payload transport and curated requests

**Status: agreed publication direction; product workflow/refs are not implemented.** This is distinct from the repository's application-release workflow.

## Persistent requests, fixed trusted workflow

Store project configuration and curated requests under `.universe-airlock/`. A request is structured data naming intended sources, targets, roots/versions, assets, and policy constraints. Adding a dependency should not create a new executable workflow or bespoke restore script.

An explicit sync submits the selected data/config revision. A fixed trusted workflow validates the request schema, source/URL allowlists, target profiles, download/storage limits, and script policy, then delegates acquisition/verification/publication responsibilities to Universe Airlock. User-controlled data must not be interpolated as arbitrary shell/code or granted publication permission by itself.

The generated workflow is thin: an initial pinned archive download/checksum/extract/version block, then application subcommands for the work. It must not grow a second implementation of resolution, caching, packing, integrity checks, or restore.

## Immutable data and stable entrypoint

Reuse verified immutable payloads when their content/identity/profile matches. Publish payload data to dedicated Git branches/refs, potentially orphan branches for data/history separation. The exact ref naming scheme remains to be selected.

A bootstrap manifest must identify exact immutable payload commits/shards. Publish and verify all required payload references first. Advance the stable profile bootstrap ref **last**, so a target cloning that entrypoint never selects partially uploaded data.

Restore pins the chosen snapshot; reading a moving latest/bootstrap ref once does not authorize mixing payload generations during a resumed restore. Publication and local activation need explicit interruption/recovery behavior.

## Size constraints and proposed budgets

- Ordinary Git files above **100 MiB** are not suitable for this transport.
- **2 GiB is a per-push limit**, not a per-branch capacity/quota.
- Approximately **90 MiB shards** and **1 GiB per payload branch/volume** were suggested safety budgets, not approved hard requirements or implemented guarantees.

Plan limits with margin for pack overhead, existing objects, and retries. Multiple refs/branches do not remove the per-push constraint. Git/LFS and GitHub release assets are distinct transports; do not quietly assume either one solves ordinary Git-blob limits.

## Compression and reconstruction

Brotli is a transport option where useful; original archives already compressed may not benefit. Never replace canonical package bytes with a semantically different repack. Record the transport encoding, ordered shard sizes/digests, expected original object digest, and reconstruction bounds.

The selected immutable snapshot must contain everything needed to verify/reassemble its payload. Detect missing/duplicate/reordered/corrupt shards before activation. Retain canonical digests and provenance regardless of outer compression, shard size, or branch layout.

## Native targets and failure semantics

Initially use one Ubuntu bundling runner for acquisition/packing where supported. Cross-target downloads are allowed only when the upstream asset/metadata contract supports them; they are not native install/launch verification. Add native Windows/macOS product verification only as those targets become supported.

Request acceptance, completed fetching, passed offline tests, published payload, and advanced bootstrap ref are different states. A failure leaves the prior stable entrypoint usable and must not falsely mark an incomplete request as restored/published. Cross-service application release publication has separate failure semantics documented in the release guide.

## Registry catalogs and selective Git retrieval

The [accepted lazy catalog refinement](doc-109%20-%20Git-backed-catalogs-and-on-demand-command-execution.md) organizes catalogs by registry/protocol and pins exact commits/index digests together in bootstrap. Ref names remain illustrative. Keep referenced objects reachable, avoid full package-tree checkouts, require selective-fetch support, and treat a full air gap as pretransferred closure rather than an implicit Git connection. The temporary developer transport is [pixi-sandbox](doc-110%20-%20Interim-development-transport-with-pixi-sandbox.md), not this future product protocol.
