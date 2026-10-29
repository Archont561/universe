---
id: m-6
title: "M6 — Native Cargo, Python-wheel, and npm adapters"
---

## Description

Implement the three additional native package protocols in v1: Cargo sparse index and
crate archives, Python wheel Simple API/index, and npm metadata/tarballs. The adapter
contracts and pinned release manifest must be in place first. Conda's standard static
channel remains M3; Airlock consumption across all four protocols is M9.

## Tasks

- `UNI-42` — Cargo sparse-index/crate archive adapter (`UNI-31`, `UNI-41` blockers).
- `UNI-43` — Python wheel Simple API adapter (`UNI-31`, `UNI-41` blockers).
- `UNI-44` — npm registry adapter (`UNI-31`, `UNI-41` blockers).

## Exit condition

Each native client resolves/fetches pinned fixtures from protocol-compatible local endpoints;
metadata, artifact identity, hashes, and paths are checked; tests pass with public egress
blocked. Generic assets, Agent Skills, and STAC remain outside v1.
