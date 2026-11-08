---
id: UNI-69
title: Scope lint rules per catalog subtree once catalogs exist
status: To Do
assignee: []
created_date: '2026-11-08'
updated_date: '2026-11-08'
labels:
  - architecture
  - channel
  - testing
milestone: m-6
documentation:
  - backlog/drafts/draft-5 - Catalog-scoped-ecosystem-gated-lint-rules.md
  - backlog/docs/doc-109 - Git-backed-catalogs-and-on-demand-command-execution.md
priority: low
type: feature
ordinal: 69000
---

## Description
<!-- SECTION:DESCRIPTION.BEGIN -->
GitHub issue #17 proposes one rule family per catalog subtree, gated by path, with
cross-catalog rules exempted so they run everywhere. Two facts in it need correcting before
the work starts:

- The catalog root is not decided. `doc-109` says `packages/conda`, `packages/cargo`,
  `packages/npm`, `packages/pypi`, and `assets` "are possible names, not frozen ref
  spellings" — and `packages/` here already holds the Bun workspace (`config`, `portal`,
  `schema`), so reusing that root would collide with it.
- No catalog exists, so a path-gated rule over a missing directory is vacuously green. The
  rules wait for the catalogs, not the other way round.

The gating mechanism itself is settled: `alint` is the declarative layer (`decision-121`),
and its ecosystem rulesets are the model.
<!-- SECTION:DESCRIPTION.END -->

## Acceptance Criteria
<!-- AC.BEGIN -->
- [ ] #1 The catalog root and the subtree names are decided in a record, with the collision against `packages/` resolved rather than assumed.
- [ ] #2 Each catalog subtree has its own rule family, gated so it is inert outside that subtree.
- [ ] #3 A Conda rule does not fire on a Cargo catalog, proven by a fixture where both exist.
- [ ] #4 Cross-catalog rules — version alignment and identity uniqueness — are declared separately and run across every catalog.
- [ ] #5 A subtree with no catalog present is reported as unchecked rather than clean, so an empty tree cannot read as a passing one.
<!-- AC.END -->

## Definition of Done
<!-- DOD.BEGIN -->
- [ ] #1 The rules and their gating are committed and run where catalog validation happens.
- [ ] #2 `doc-109` names the rule families as the enforcement of the layout it describes.
- [ ] #3 `pixi run gates` passes.
<!-- DOD.END -->
