---
id: decision-113
title: Native-client execution through local package views
date: '2026-10-27T09:20:00Z'
status: accepted
---

## Context

The user proposed running native commands such as `pixi add` through Universe Airlock while selectively supplying their package requests.

## Decision

Adopt a supported-command wrapper with invocation-scoped native mirror/source configuration and local protocol-compatible endpoints. Keep resolution with the native client. Universe Airlock maps exact artifact requests to its published, verified inventory; it does not infer a compatible version by filename or implement a universal HTTPS interceptor.

A local view serves verified objects. An approved-Git-connected coordinator may fill cache misses only from exact digest-pinned published locations. No arbitrary public-registry fallback, hidden solving, global configuration edits, or accidental persistent localhost lock identities.

## Consequences

Each ecosystem needs an explicit adapter, native semantic/relocation tests, and target-aware metadata. Arbitrary subprocess downloads are not automatically supported. Workload egress must be enforced outside package-manager settings. Filesystem-only mode remains a separate contract.

## Status boundary

Accepted design, no exec/proxy command implemented. See [on-demand execution](../docs/doc-109%20-%20Git-backed-catalogs-and-on-demand-command-execution.md).
