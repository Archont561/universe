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
- `UNI-66` — declarative rule engine for adapter lint rules (`UNI-7` blocker; `UNI-31` is done).
- `UNI-67` — doc-102 integrity invariants as cross-artifact rules (`UNI-10`, `UNI-21` blockers).
- `UNI-68` — freshness for every generated artifact, not just the rendered workflows (`UNI-10`, `UNI-46` blockers).
- `UNI-76` — the renderer deletes and overwrites only the files it wrote.
- `UNI-77` — a publish job that cannot publish is refused before it is rendered.
- `UNI-78` — the rendered file set is the set of lanes the manifest declares (`UNI-76` blocker).
- `UNI-79` — rendered checkouts do not persist credentials.
- `UNI-80` — `ci render --stdout` separates its documents.

## Exit condition

Rust/BDD/property tests pass; command-recording fixtures prove invocation,
collection, isolation, and no network fallback; representative artifact fixtures
are audited deterministically. No builder becomes a Kit runtime dependency merely
to produce packages.
