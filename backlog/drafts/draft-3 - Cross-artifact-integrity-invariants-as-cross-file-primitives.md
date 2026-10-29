---
id: draft-3
title: Cross-artifact integrity invariants as cross-file relational primitives
status: draft
created_date: '2026-10-29'
updated_date: '2026-10-29'
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

## Reality check against this repository (verified 2026-10-29)

- doc-102 does carry the quoted invariants as prose, so the premise is accurate.
- The claimed existing machinery is absent: `petgraph` is not a dependency anywhere
  (workspace `Cargo.toml` lists only `include_dir`, `serde`, `serde_json`, `thiserror`),
  so "using the existing petgraph DAG" describes future work.
- There are no fragment envelopes, artifacts, inventories, or `recipes/*` in the tree yet;
  the rules would have no subjects until `UNI-10`/`UNI-21` produce fragments.
- The issue cites "Spec 0005", which does not exist in this repository's numbering.
- Only a `command`-kind subset is expressible today; the rest is fixture-blocked.

## Prerequisites

- `UNI-10` (fragment emitter and streaming audit) and `UNI-21` (artifact identity and
  payload-leak hardening) to define the subjects the rules check.
- An artifact/CAS layout decision, since `pair` and `generated_file_fresh` both assume a
  stable fragment-to-artifact naming relation.
- The same declarative-rule decision that draft-2 depends on.

## Recommended disposition

Split later into two tracks: lint-time relational rules (fold into `UNI-7`) and the
runtime audit invariants that already belong to `UNI-10`/`UNI-21`. Keep as draft now.
