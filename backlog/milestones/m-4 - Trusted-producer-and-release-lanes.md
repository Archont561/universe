---
id: m-4
title: "M4 — Trusted producer and release lanes"
---

## Description

Complete upstream release intake, component packaging, source/audit evidence, and
fail-closed owner publishing. This is release infrastructure—not a Conda-only
stable release. Builders run in credential-free jobs; publishing validates and
transfers the exact pinned artifact set.

## Tasks

- `UNI-14` — pinned upstream release watcher (`UNI-7`, `UNI-41` blockers).
- `UNI-34` — separate release lanes (`UNI-28`, `UNI-41` blockers).
- `UNI-15` — component/facade Conda recipes and four-platform packaging (`UNI-9`, `UNI-10`, `UNI-34`, `UNI-41` blockers).
- `UNI-20` — fail-closed guard and isolated publisher (`UNI-11`, `UNI-15`, `UNI-34`, `UNI-41` blockers).
- `UNI-23` — installed-prefix functional tests and source bundles (`UNI-9`, `UNI-15` blockers).

## Exit condition

Owner release jobs verify repository/tag identity, manifests, provenance, and checksums;
Conda component packages install independently; static and registry publication does not
advance a catalog on partial or failed uploads. This milestone does not cut v1.
