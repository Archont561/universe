---
id: m-0
title: "M0 — Reproducible developer foundation"
---

## Description

Establish the standalone Universe repository, locked Pixi/Rust/Python toolchain,
repo checks, CI and Backlog conventions. Prove the interim offline developer
transport without confusing it with a sealed Airlock product snapshot.

## Tasks

- `UNI-28` — establish the standalone monorepo baseline (**Done**).
- `UNI-36` — establish/validate offline developer transport (**Done**).
- `UNI-39` — formalize Backlog planning/reference integrity (**Done**).
- `UNI-49` — make the audit gate offline-first with a bounded advisory cache (**Done**).
- `UNI-50` — adopt the alint declarative repository-lint layer (**In Progress**; config verified, gate enrollment pending relock).

## Exit condition

A clean supported checkout restores the locked developer environment within its
documented network boundary, offline gates pass, and backlog/repository checks
validate the records. The confirmed baseline gates M1 onward through `UNI-28`;
completion of `UNI-36` is a developer-transport readiness gate, not a prerequisite
for every product task.
