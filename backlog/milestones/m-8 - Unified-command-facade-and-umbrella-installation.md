---
id: m-8
title: "M8 — Unified command facade and umbrella installation"
---

## Description

Complete the thin `universe` command front door and convenient umbrella Conda package.
The facade routes to separately packaged Kit/Airlock components; it owns no producer,
registry, catalog, or consumer implementation. The bootstrap remains a separate artifact.

## Tasks

- `UNI-40` — `universe` facade and umbrella package (`UNI-15`, `UNI-34`, `UNI-38` blockers).

## Exit condition

A clean static-channel install provides the facade and its compatible Kit/Airlock runtime
packages; dispatch, argument preservation, diagnostics, exit status, and independent component
installation are verified.