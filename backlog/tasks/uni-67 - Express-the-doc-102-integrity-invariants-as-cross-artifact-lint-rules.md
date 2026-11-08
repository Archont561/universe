---
id: UNI-67
title: Express the doc-102 integrity invariants as cross-artifact lint rules
status: To Do
assignee: []
created_date: '2026-11-08'
updated_date: '2026-11-08'
labels:
  - architecture
  - audit
  - testing
milestone: m-2
documentation:
  - backlog/drafts/draft-3 - Cross-artifact-integrity-invariants-as-cross-file-primitives.md
  - backlog/docs/doc-102 - Product-architecture-and-bundle-model.md
priority: medium
type: feature
ordinal: 67000
---

## Description
<!-- SECTION:DESCRIPTION.BEGIN -->
GitHub issue #15 maps the integrity invariants in `doc-102` — "Every advertised installable
artifact exists and verifies; incomplete views never become visible" — onto cross-file
relational primitives. The mapping is sound and the engine exists (`alint` 0.20.1 ships
`pair`, `unique_by`, `every_matching_has`, `file_graph`, and `generated_file_fresh`), but
the subjects do not: no fragment envelope type, no inventory, no `recipes/`, and no
`petgraph` anywhere in the workspace.

That is why this task follows `UNI-10` and `UNI-21` rather than the alint adoption. A
cross-artifact rule with no artifacts is vacuously green, and a green rule that can never
fail is worse than a missing one — it is a claim nobody can falsify.
<!-- SECTION:DESCRIPTION.END -->

## Acceptance Criteria
<!-- AC.BEGIN -->
- [ ] #1 Each invariant in `doc-102` §integrity maps to a named rule with the primitive it uses, and the mapping lives next to the rules rather than in a task checkbox.
- [ ] #2 Fragment-to-artifact pairing is enforced: every advertised fragment names an artifact that exists, with its declared digest and size.
- [ ] #3 Package identity is unique across ecosystems, keyed by the identity `UNI-41` defines.
- [ ] #4 No rule can pass vacuously: each has a fixture repository where it fails, run by `pixi run alint-test` or its `universe lint` equivalent.
- [ ] #5 `doc-102` names the rules as the machine-readable enforcement of the invariants it states in prose.
<!-- AC.END -->

## Definition of Done
<!-- DOD.BEGIN -->
- [ ] #1 The rules are committed and run in the gate that owns them.
- [ ] #2 A decision records the division of labour between these lint-time rules and the runtime audit in `UNI-10`/`UNI-21`.
- [ ] #3 `pixi run gates` passes.
<!-- DOD.END -->
