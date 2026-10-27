---
id: decision-102
title: Original conda artifacts and effective metadata
date: '2026-10-25T23:49:00Z'
status: accepted
---

## Context

Exporting an installed Pixi environment is not the requested reusable dependency cache/channel. Rebuilding repodata from archive internals can also discard upstream dependency hotfixes.

## Decision

Use a Rattler-backed product backend for original conda package archives and authoritative **effective/hotfixed metadata**. Preserve source/channel/build identity, dependency closure, and target virtual-package semantics. Publish filtered installable channel views; do not export installed Pixi prefixes or activation state as the canonical payload.

Support projectless conda scenarios. Pixi is an optional lock-input adapter/downstream consumer. Choose a pinned helper or embedded index implementation when that feature is built; index generation alone is not closure acquisition.

## Consequences

Target environments are created later from original local packages. Archive bytes and original metadata remain canonical; generated indexes/caches are derivatives. `rattler-build` packages the Universe Airlock application and must not be mistaken for an already implemented product cache backend.

## Status boundary

Product implementation pending. See [architecture](../docs/doc-102%20-%20Product-architecture-and-bundle-model.md) and [adapter contracts](../docs/doc-104%20-%20Ecosystem-adapters-and-external-assets.md).
