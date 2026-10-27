---
id: decision-103
title: Finite snapshots locked replay and curated additions
date: '2026-10-25T23:49:00Z'
status: accepted
---

## Context

Offline replay and adding a newly requested package are different promises. A folder of downloaded artifacts does not guarantee correct native resolution or a complete reusable universe.

## Decision

Support both **locked-project replay** and **curated additions**, constrained to a finite sealed artifact universe and explicitly resolved scenarios. Preserve original input bytes, native graph/source identity, features/peers/options, and target-qualified environment membership.

Use native ecosystem semantics rather than one generic version/range solver. A missing/unsupported artifact or incompatible scenario fails locally; never fall through to public registries. A broad root range admits concrete recorded resolutions, not every matching version.

## Consequences

Keep scenario-specific graphs even when their blobs deduplicate. Typed relocation requires graph-equivalence checks and does not silently modify original workspaces. HTTP success does not imply files-mode or arbitrary-package compatibility.

## Status boundary

Accepted product contract, not an implemented feature. See [architecture](../docs/doc-102%20-%20Product-architecture-and-bundle-model.md) and [acceptance gates](../docs/doc-106%20-%20Offline-verification-and-acceptance-gates.md).
