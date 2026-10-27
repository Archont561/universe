---
id: decision-110
title: Generic assets and Playwright are separate artifact types
date: '2026-10-25T23:49:00Z'
status: accepted
---

## Context

Not every dependency is a registry package. Playwright's browser downloads and operating-system prerequisites cannot be modeled as ordinary npm or conda package bytes alone.

## Decision

Treat generic downloadable files/archives and Playwright browser distributions/auxiliaries as distinct pinned artifact types. Preserve their own source/digests, platform constraints, safe extraction/layout policy, and consumption configuration. Do not force arbitrary assets into a language registry or conda identity.

Associate browser revisions with the locked Playwright version/target and handle expected cache paths, permissions, and auxiliary components. Model OS libraries/fonts/sandbox prerequisites separately and require native launch verification.

## Consequences

An Ubuntu fetch of another platform's asset is not proof that it installs/runs there. Approved asset/browser tests must not fall back to Internet downloads. Lifecycle-script skipping alone cannot certify browser automation.

## Status boundary

Accepted boundary; no asset/Playwright adapter has been implemented. See [adapter specification](../docs/doc-104%20-%20Ecosystem-adapters-and-external-assets.md).
