---
id: decision-14
title: Canonical Universe product and artifact names
date: '2026-10-27'
status: accepted
---

## Context

Universe has a connected producer, a sealed offline consumer, and a thin
bootstrapper. Those artifacts have different trust, size, and lifecycle
profiles. Collapsing them into one executable would make the offline proof
weaker and force the bootstrapper to carry producer machinery.

## Decision

- Repository and project family: **Universe** (`Archont561/universe`).
- Connected owner/producer executable: **`universe-kit`**.
- Sealed/offline consumer executable: **`universe-airlock`**.
- Thin standalone native bootstrapper: **`universe-airlock-bootstrap`**.
- Shared library: **`universe-core`**.

One codebase and shared core do not imply one artifact. The producer may
build, watch, index, and publish. The Airlock consumer must not gain those
capabilities by flag; they are absent at link time. The bootstrapper is
smaller still and may not absorb adapters, a server, prompts, a TUI, or
publishing logic.

## Consequences

The scaffold creates all three binaries immediately, even though each only
reports version. Release lanes, package names, generated entrypoints, and
documentation use these canonical artifact names.
