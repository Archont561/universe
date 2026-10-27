---
id: decision-104
title: Init-generated entrypoint and Pixi-managed restore
date: '2026-10-25T23:49:00Z'
status: accepted
---

## Context

The target workflow was explicitly corrected: the target user should run the generated restore entrypoint, not manually install Universe Airlock or run init on the target.

## Decision

`init` belongs on the project/export side and generates configuration, restore entrypoint(s), and a thin bundle workflow. The target entrypoint selects/clones bootstrap content, verifies/launches a native bootstrap, and uses **bundled Pixi** to establish **Pixi-managed Universe Airlock** from local original packages before restoring the selected payload.

Do not make restoration depend on undeclared Internet fetches or a preinstalled language ecosystem. Keep bootstrap/protocol needs minimal instead of linking all future application/UI code into bootstrap.

## Consequences

Bootstrap and payload references must be pinned/verifiable and compatible with the declared target. Installed prefixes remain target-created state, not exported canonical data. Target-state directory and precise generated script/config names are still to be selected.

## Status boundary

Workflow agreed; no initializer, restore entrypoint generator, or bootstrap program exists yet. See [lifecycle](../docs/doc-103%20-%20Init-bootstrap-and-restore-lifecycle.md).

## 27 October 2026 refinement

Bootstrap carries the tool-install closure and catalog pins, not the whole project environment. It can establish PATH availability before a profile is fully prefetched; see decision-115 and doc-109.
