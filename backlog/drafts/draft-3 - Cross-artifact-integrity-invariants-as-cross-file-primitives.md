---
id: draft-3
title: Cross-artifact integrity invariants as cross-file relational primitives
status: promoted
created_date: '2026-10-29'
updated_date: '2026-11-08'
source: https://github.com/Archont561/universe/issues/15
labels:
  - architecture
  - lint
  - integrity
related_tasks:
  - UNI-7
  - UNI-10
  - UNI-21
---

## Origin

GitHub issue #15, imported into Backlog on 2026-10-29. Unapproved design proposal.

## Proposal

Express the doc-102 integrity invariants as declarative cross-file relational rules
(`pair`, `unique_by`, `every_matching_has`, `file_graph`, `generated_file_fresh`) instead
of prose plus hand-written checks. Each invariant maps to at least one rule with a named
primitive.

## Reality check against this repository (verified 2026-10-29, corrected 2026-11-08)

- doc-102 does carry the quoted invariants as prose, so the premise is accurate.
  `backlog/docs/doc-102 …md` line 64 has "Every advertised installable artifact exists and
  verifies; incomplete views never become visible."
- `petgraph` is still not a dependency anywhere: the workspace lists `goblin`,
  `minijinja`, `rattler_*`, `minisign-verify`, `spdx`, `toml_edit`, `serde*`,
  `thiserror`, and no graph crate. **The 2026-10-29 wording that the workspace lists "only
  `include_dir`, `serde`, `serde_json`, `thiserror`" was already stale when written** — it
  described the scaffold before the Conda and workflow crates landed. The conclusion is
  unchanged: "using the existing petgraph DAG" describes work that does not exist.
- There are no fragment envelopes, artifacts, inventories, or `recipes/*` in the tree yet;
  the rules would have no subjects until `UNI-10`/`UNI-21` produce fragments. `FragmentEnvelope`
  appears in no `.rs` file.
- The issue cites "Spec 0005", which does not exist in this repository's numbering.
- **Corrected 2026-11-08:** the rules can be expressed against `alint` today (102 kinds,
  `pair`, `unique_by`, `every_matching_has`, `file_graph`, `generated_file_fresh`, all
  verified against 0.20.1), so the engine is not the blocker. The subjects are.

## Prerequisites

- `UNI-10` (fragment emitter and streaming audit) and `UNI-21` (artifact identity and
  payload-leak hardening) to define the subjects the rules check.
- An artifact/CAS layout decision, since `pair` and `generated_file_fresh` both assume a
  stable fragment-to-artifact naming relation.
- The same declarative-rule decision that draft-2 depends on.

## Recommended disposition

Split later into two tracks: lint-time relational rules (fold into `UNI-7`) and the
runtime audit invariants that already belong to `UNI-10`/`UNI-21`. Keep as draft now.

## Promoted (2026-11-08)

Opened as `UNI-67`, in m-2, after `UNI-10` and `UNI-21`. The subject-free problem is why
it is not earlier: a cross-artifact rule with no artifact passes, and a passing rule that
can never fail is worse than none.
