---
id: decision-105
title: Data-driven requests and immutable payload publication
date: '2026-10-25T23:49:00Z'
status: accepted
---

## Context

Curated additions must persist without creating a new executable workflow per package. Git payload publication must not expose an incomplete snapshot through a stable bootstrap reference.

## Decision

Store request/configuration data under `.universe-airlock/` and process explicit synced changes through a fixed trusted, thin workflow. Publish reusable immutable payload data first; advance the stable bootstrap ref **last**, with exact payload commit/digest references.

Use sharding/outer compression where needed while preserving original artifact bytes. Respect ordinary Git file limits and the **per-push** 2 GiB constraint; this is not a branch quota.

## Consequences

Request acceptance, acquisition, verification, publication, and stable-ref advancement are separate states with recovery behavior. Ref spelling, approximately 90 MiB shard/1 GiB volume budgets, and exact Brotli policy are proposals, not enforced guarantees. Native platform certification is separate from cross-target acquisition.

## Status boundary

Accepted direction, implementation pending. See [Git transport and requests](../docs/doc-105%20-%20Git-payload-transport-and-curated-requests.md).

## 27 October 2026 refinement

Logical catalogs are registry/protocol-qualified. Pin exact catalog/index commitments across branches, retain reachable revisions, and selectively retrieve package objects; see decision-114 and doc-109.
