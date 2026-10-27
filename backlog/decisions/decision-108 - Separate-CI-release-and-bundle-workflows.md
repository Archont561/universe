---
id: decision-108
title: Separate CI release and bundle workflows
date: '2026-10-25T23:49:00Z'
status: accepted
---

## Context

Development checks, distributing the Universe Airlock executable, and publishing a project's dependency bundle have different permissions, inputs, and trust boundaries.

## Decision

Keep **ci.yml** and **release.yml** separate. The future init-generated bundle workflow is a third responsibility, not another application-release job. Repository workflow run steps use named Pixi tasks.

Use cargo-dist compressed application archives and **rattler-build** with `crates/universe-airlock/recipe.yml`. Trusted prefix.dev publication targets `archont561/archont561` from **release.yml**, using GitHub environment **prefix** and OIDC/id-token permission. Additional raw binary assets are not required.

## Consequences

Native archive/package tests and byte/commit-bound evidence precede publication. Windows/macOS configuration or recipe rendering is not native execution. Account-side Repository Access/environment protections and Codecov authorization remain owner work. Cross-service publication cannot be described as atomic.

## Status boundary

Repository automation configured; no external account setup/publication performed. Future generated bundle workflow is unimplemented. See [release guide](../docs/doc-108%20-%20Releases-and-trusted-publishing.md).

## 27 October 2026 refinement

The current interim pixi-sandbox developer publisher is distinct from the still-unimplemented Universe Airlock-generated bundle workflow. Native relock/upgrade templates are inactive; active CI retains named Pixi-task steps. See decision-116 and doc-110.
