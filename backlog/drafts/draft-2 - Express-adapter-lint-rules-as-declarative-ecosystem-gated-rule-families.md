---
id: draft-2
title: Express adapter lint rules as declarative, ecosystem-gated rule families
status: draft
created_date: '2026-10-29'
updated_date: '2026-10-29'
source: https://github.com/Archont561/universe/issues/14
labels:
  - architecture
  - lint
related_tasks:
  - UNI-7
  - UNI-31
---

## Origin

GitHub issue #14, imported into Backlog on 2026-10-29. Unapproved design proposal.

## Proposal

Replace Rust match arms inside `EcosystemAdapter::lint` with declarative, composable,
ecosystem-gated rulesets (YAML/TOML). `lint()` keeps only the `command` kind for
irreducible checks. A new ecosystem adds a ruleset plus an `extends:` entry instead of a
new match arm.

## Reality check against this repository (verified 2026-10-29)

- `EcosystemAdapter` does not exist yet: no trait, no adapter crate, no `lint` method in
  any `.rs` file. The adapter boundary is `UNI-31`, still `To Do` in m-1.
- `universe-kit lint` does not exist; `universe-kit` reports version only.
- The issue's examples reference files this repository does not have: `python/pyproject.toml`,
  `crates/universe-core/lint-rules/`, `python/dist/*.whl`.
- The issue's DoD asks to update "Spec 0005-equivalent (decision-13 §2)"; decision-13
  has no spec numbering here, so that reference must be restated against the actual file.
- Design consequence: adopting declarative rules before `UNI-31` fixes the trait shape
  would decide the adapter contract from a lint convenience, inverting the dependency.

## Prerequisites

- `UNI-31` (adapter boundary and conformance fixtures) must land first, or the trait
  shape must be re-derived in this proposal.
- A decision recording the declarative rule model and its division of labour with the
  `EcosystemAdapter` trait, per decision-13.
- `UNI-7` (`universe-kit lint`) provides the command surface the rulesets plug into.

## Recommended disposition

Draft stays open until `UNI-31` is decided; then re-scope as part of `UNI-7` rather than
as a separate task.
