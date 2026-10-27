---
id: m-11
title: "M11 — Post-v1 adoption, optional extensions, and native developer transport"
---

## Description

Post-v1 work only. Publish the public skill after commands stabilize, validate a representative
consumer dogfood, record any optional marketplace adapter recommendation, and implement the
native developer transport needed before retiring the interim sandbox. These tasks do not block
or alter the v1 release gate.

## Tasks

- `UNI-17` — public skills.sh package (`UNI-16` blocker).
- `UNI-18` — CloudCompare/ODM Conda dogfooding (`UNI-16` blocker).
- `UNI-35` — separate development documentation site (independent; optional).
- `UNI-37` — evaluate out-of-v1 marketplace adapters (`UNI-16`, `UNI-30`, `UNI-31` blockers).
- `UNI-48` — native developer transport (`UNI-16`, `UNI-33`, `UNI-36`, `UNI-45` blockers).
- `UNI-27` — remove pixi-sandbox only after UNI-48 passes (`UNI-48` and `UNI-16` blockers).

## Exit condition

Post-v1 adoption and extension work is clearly labeled and does not expand the v1 adapter scope.
The interim developer transport is removed only after a clean native restore and independent
closure verification pass; otherwise it remains available.