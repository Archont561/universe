---
id: decision-116
title: Use pixi-sandbox for interim developer transport
date: '2026-10-27T09:20:00Z'
status: accepted
---

## Context

Universe Airlock is still a VERSION-only Rust scaffold. The user authorized pixi-sandbox initialization here so locked developer environments and Cargo sources can be transported before Universe Airlock implements reusable conda/Cargo catalogs.

## Decision

Use pinned pixi-sandbox 0.9.0 as workspace-managed developer tooling in utils. Start with the default environment, linux-64, and Cargo vendoring. Preserve the Python dispatcher, npm tooling, four Pixi features, existing CI/release separation, and version-only product scope.

Initialize with a root restore entrypoint and inactive native workflow templates; use a manual project-owned named-task publisher rather than enabling the upstream relock/upgrade bot. Native templates remain auditable/regenerable with the same explicit init paths.

## Consequences

Initialization does not publish a branch or certify offline development. Standalone transport bootstraps need pinned verified release binaries. npm payloads, RustSec data, extra tool downloads, isolated conda build dependencies and host requirements need explicit treatment. Do not adopt the upstream full automation/dependency stack or environment transport as Universe Airlock's permanent artifact model.

## Status boundary

Tooling integration initialized and locally checked, not a new application feature. Native render checks and plan validation passed; no real transport has been packed/restored/published. No commits/pushes/publication are authorized by initialization. See [interim transport guide](../docs/doc-110%20-%20Interim-development-transport-with-pixi-sandbox.md).
