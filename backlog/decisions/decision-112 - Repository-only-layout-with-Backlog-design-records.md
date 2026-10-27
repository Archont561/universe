---
id: decision-112
title: Repository-only layout with Backlog design records
date: '2026-10-25T23:49:00Z'
status: accepted
---

## Context

The user requested a single Rust-based Universe Airlock repository with its tooling, removal of standalone docs/design exports and ZIPs, and preservation of conversation conclusions inside Backlog.

## Decision

Keep the repository under `universe-airlock/`, with README as the entrypoint and design/architecture records under **backlog/docs** and **backlog/decisions**. Remove the top-level docs directory, standalone design workspace, ZIP exports, legacy ecosystem workarounds, and generated build/report artifacts.

Preserve actual source, manifests/locks, CI/hooks, packaging recipes, bootstrap tool locks, and Python/npm tooling. Do not create a replacement export/archive or seed implementation tasks/AI integrations as part of consolidation.

## Consequences

Distinguish accepted decisions, proposed recommendations, and implemented behavior. Normal developer commands can regenerate ignored outputs when needed; generated caches/builds are not source deliverables. Documentation does not authorize commits, pushes, or publication.

## Status boundary

This consolidation implements the requested repository layout. See the [design index](../docs/doc-101%20-%20Universe Airlock-design-and-status.md) and [tooling guide](../docs/doc-107%20-%20Rust-workspace-and-development-tooling.md).
