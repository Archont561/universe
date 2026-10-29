---
id: m-9
title: "M9 — Airlock native views and integrated v1 evidence"
---

## Description

Join the four native package protocols to Airlock's finite verified snapshot model and
independently review the complete v1 evidence matrix. This milestone is the final technical
gate before the integrated release.

## Tasks

- `UNI-45` — Airlock scoped local views/execution across Conda, Cargo, Python wheels, and npm (`UNI-12`, `UNI-32`, `UNI-33`, `UNI-38`, `UNI-41`–`UNI-44` blockers).
- `UNI-47` — offline/connected test matrix and independent evidence review; depends on all v1 producer, release, package, Airlock, and portal acceptance tasks.

## Exit condition

Native clients resolve/fetch pinned artifacts from local Airlock views with public egress denied;
unsupported, corrupt, or missing objects fail closed; connected discovery/publishing is separated
from offline fixture evidence; independent review approves the evidence before UNI-16.
