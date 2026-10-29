---
id: draft-5
title: Catalog-scoped, ecosystem-gated lint rules
status: draft
created_date: '2026-10-29'
updated_date: '2026-10-29'
source: https://github.com/Archont561/universe/issues/17
labels:
  - architecture
  - lint
  - catalogs
related_tasks:
  - UNI-7
  - UNI-41
  - UNI-46
---

## Origin

GitHub issue #17, imported into Backlog on 2026-10-29. Unapproved design proposal.

## Proposal

Give each catalog subtree its own rule family gated by path (`packages/conda/**`,
`packages/cargo/**`, `packages/npm/**`, `packages/pypi/**`, `assets/**`), with cross-catalog
rules (version alignment, identity uniqueness) explicitly exempted so they run across all
catalogs.

## Reality check against this repository (verified 2026-10-29)

- None of the five catalog subtrees exist. `packages/` currently holds `config`, `schema`,
  and `portal` only, and no `assets/` directory exists.
- The referenced catalog layout comes from doc-109 as a *proposed* Git-backed catalog
  model; no catalog has been generated or committed.
- Identity uniqueness across ecosystems depends on `UNI-41` (project and pinned-release
  manifest contracts), still `To Do`. Rules written now would encode an unapproved schema.
- The `when: { path-match: ... }` gating model is an `alint` capability and is therefore
  downstream of draft-1's adoption decision.

## Prerequisites

- `UNI-41` manifest contracts, so the rule subjects have a declared identity.
- Catalog generation work (`UNI-12`, `UNI-46`) before path-gated rules can be trusted not to
  be vacuously green on an empty tree.
- draft-1 (alint or an equivalent declarative engine).

## Recommended disposition

Keep as draft; re-evaluate when the first catalog exists. Do not create the catalog
directories merely to host rule fixtures.
