---
id: draft-5
title: Catalog-scoped, ecosystem-gated lint rules
status: promoted
created_date: '2026-10-29'
updated_date: '2026-11-08'
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

## Reality check against this repository (verified 2026-10-29, corrected 2026-11-08)

- None of the five catalog subtrees exist. `packages/` holds `config`, `portal`, and
  `schema` — the Bun workspace for this repository's own tooling — and no `assets/`
  directory exists. Reusing `packages/` as the catalog root would collide with them, which
  the issue does not address.
- The referenced catalog layout comes from doc-109, which is explicit that `packages/conda`,
  `packages/cargo`, `packages/npm`, `packages/pypi`, and `assets` "are possible names, not
  frozen ref spellings". Rules must not freeze a name the design leaves open.
- Identity uniqueness across ecosystems depends on `UNI-41` (project and pinned-release
  manifest contracts), still `To Do`. Rules written now would encode an unapproved schema.
- ~~The `when: { path-match: ... }` gating model is downstream of draft-1's adoption
  decision.~~ **Corrected 2026-11-08:** `alint` is adopted (`decision-121`), so the gating
  model is available. It is still the wrong thing to reach for while the subtrees it gates
  do not exist: a path-gated rule over a missing directory is vacuously green.

## Prerequisites

- `UNI-41` manifest contracts, so the rule subjects have a declared identity.
- Catalog generation work (`UNI-12`, `UNI-46`) before path-gated rules can be trusted not to
  be vacuously green on an empty tree.
- draft-1 (alint or an equivalent declarative engine).

## Recommended disposition

Keep as draft; re-evaluate when the first catalog exists. Do not create the catalog
directories merely to host rule fixtures.

## Promoted (2026-11-08)

Opened as `UNI-69`, in m-6, after `UNI-41` and the three native adapters. The task states
the collision with `packages/` as an open question rather than assuming the issue's layout.
