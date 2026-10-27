---
id: decision-101
title: Project identity and version-only scope
date: '2026-10-25T23:49:00Z'
status: superseded
---

## Context

The conversation contained historical naming/layout suggestions and a much broader product design. The user explicitly resolved identity and constrained the authorized Rust implementation.

## Decision

Use **Universe Airlock** throughout: executable/application crate `universe-airlock` in repository **`Archont561/universe`**. A prefix.dev channel is not configured. The current Conda recipe scaffold is `crates/universe-airlock/recipe/recipe.yaml`; it has not yet been built or validated.

The application scaffold exposes VERSION **`0.1.0`** and tests that value. Keep Rust `src/` and `tests/` separate. Do not implement init/restore/cache/TUI/update features merely because they appear in the design discussion.

## Consequences

CLI/core scaffolding, help/version argument handling, and development/release plumbing can exist without claiming the cache product is implemented. Future product work requires separate authorization and passing acceptance gates. Name/repository spelling must not be silently normalized.

## Status boundary

Identity/version scaffold is implemented. Broader product behavior remains design only. See the [design index](../docs/doc-101%20-%20Universe Airlock-design-and-status.md).
