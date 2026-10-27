---
id: m-2
title: "M2 — Kit commands, artifact orchestration, and audit"
---

## Description

Implement Kit's core validation, planning, configured external-command orchestration,
and artifact audit/security capabilities. Native builders remain external tools; Kit
validates, collects, and records their outputs without implementing package build
systems.

## Tasks

- `UNI-7` — lint recipes, manifests, licenses, and workflow drift.
- `UNI-8` — plan native dependency DAGs and published/missing artifacts.
- `UNI-9` — orchestrate configured build/test commands (`UNI-7`, `UNI-8`, `UNI-31`, `UNI-41` blockers).
- `UNI-10` — stream/audit Conda artifacts and emit fragments.
- `UNI-21`, `UNI-22` — harden artifact identity, archives, and source fetching (`UNI-10` blocker).

## Exit condition

Rust/BDD/property tests pass; command-recording fixtures prove invocation,
collection, isolation, and no network fallback; representative artifact fixtures
are audited deterministically. No builder becomes a Kit runtime dependency merely
to produce packages.