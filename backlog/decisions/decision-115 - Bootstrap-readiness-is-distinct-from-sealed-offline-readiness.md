---
id: decision-115
title: Bootstrap readiness is distinct from sealed offline readiness
date: '2026-10-27T09:20:00Z'
status: accepted
---

## Context

A thin bootstrap can make Universe Airlock available without downloading a project environment. That does not make missing dependency bytes available on a machine with no network.

## Decision

Separate bootstrap-ready, approved-Git-connected lazy consumption, and sealed offline readiness. Bootstrap carries the verified minimum needed for bundled Pixi to establish Pixi-managed Universe Airlock and its pinned catalog commitments; no circular dependency on Universe Airlock's own HTTP service is permitted.

Before a full air gap, prefetch/export/import the declared scenario's complete closure. Missing objects fail locally, including missing Git promisor objects. Public GitHub reachability is network access, not a full-airgap guarantee.

## Consequences

PATH registration is explicit/user-scoped. Inventory, install and build evidence stay separate. The fetcher and wrapped workload have distinct enforced network capabilities. Native flags/configuration alone do not certify isolation or runnable host prerequisites.

## Status boundary

Accepted requirement, future product implementation pending. See [lazy modes](../docs/doc-109%20-%20Git-backed-catalogs-and-on-demand-command-execution.md), [bootstrap](../docs/doc-103%20-%20Init-bootstrap-and-restore-lifecycle.md) and [acceptance](../docs/doc-106%20-%20Offline-verification-and-acceptance-gates.md).
