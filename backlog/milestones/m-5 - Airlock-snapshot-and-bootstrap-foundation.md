---
id: m-5
title: "M5 — Airlock sealed-snapshot and bootstrap foundation"
---

## Description

Build the finite Airlock consumer foundation independently of producer/build/publish
capabilities: verified snapshots, content-addressed objects, transactional restore,
and a separate bootstrap package. Native protocol-specific views for all v1 registries
are completed later in M9 after the adapters exist.

## Tasks

- `UNI-32` — sealed snapshot verification (`UNI-30`, `UNI-31` blockers).
- `UNI-33` — bootstrap verify/restore (`UNI-32` blocker).
- `UNI-38` — Airlock Conda recipe/package (`UNI-31` blocker).

## Exit condition

A pinned fixture snapshot verifies and restores transactionally; corruption/missing
objects fail closed; the bootstrap remains separate from Kit and Airlock runtime
logic. This foundation is required before full M9 protocol consumption.
