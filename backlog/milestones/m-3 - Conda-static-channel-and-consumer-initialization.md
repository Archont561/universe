---
id: m-3
title: "M3 — Conda static channel and consumer initialization"
---

## Description

Publish verified artifacts and build the standard Conda static-channel tree, including
actual package bytes at canonical subdir paths, CEP-15/CEP-16 metadata, and consumer
initialization commands. GitHub Release links or portal pages alone are not a channel.

## Tasks

- `UNI-11` — multi-target artifact publisher (`UNI-10`, `UNI-31`, `UNI-41` blockers).
- `UNI-12` — static Conda indexes and package paths (`UNI-10`, `UNI-11` blockers).
- `UNI-13` — consumer `init`/`add` and generated workflows (`UNI-7`, `UNI-11`, `UNI-12` blockers).

## Exit condition

A real Conda/Pixi client resolves, downloads, and installs fixtures from the generated
static channel with public egress denied; metadata and package bytes match pinned digests;
publication credentials remain isolated.