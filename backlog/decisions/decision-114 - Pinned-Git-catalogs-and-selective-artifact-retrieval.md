---
id: decision-114
title: Pinned Git catalogs and selective artifact retrieval
date: '2026-10-27T09:20:00Z'
status: accepted
---

## Context

The user proposed authoritative conda/language package branches with indexes, a bootstrap branch, and selective package retrieval rather than full environment downloads.

## Decision

Publish logical catalogs by registry/protocol, with native metadata and original package bytes or independently addressable shards. Branches are mutable publication pointers; the consumer's truth is an exact catalog manifest commit plus digest. Bootstrap pins one consistent cross-catalog snapshot and advances last after data/index availability checks.

Use capability-checked selective Git retrieval or an explicitly supported immutable host API. Do not confuse shallow/sparse checkout with blob filtering, silently fall back to a bulk clone, or store the entire catalog as one compressed object.

## Consequences

Retain referenced revisions through reachable history/immutable refs. Keep source namespaces, effective hotfixed metadata, checksums and closure completeness. Advertise only available artifacts. Git object reuse is not a promise of zero transfer overhead or remote storage reclamation; 2 GiB remains a per-push constraint.

## Status boundary

Accepted architecture; exact ref names/wire formats/backends remain undecided. See [catalogs](../docs/doc-109%20-%20Git-backed-catalogs-and-on-demand-command-execution.md) and [Git transport](../docs/doc-105%20-%20Git-payload-transport-and-curated-requests.md).
