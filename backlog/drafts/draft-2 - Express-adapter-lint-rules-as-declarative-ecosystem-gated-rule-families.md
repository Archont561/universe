---
id: draft-2
title: Express adapter lint rules as declarative, ecosystem-gated rule families
status: promoted
created_date: '2026-10-29'
updated_date: '2026-11-08'
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

## Reality check against this repository (verified 2026-10-29, corrected 2026-11-08)

- ~~`EcosystemAdapter` does not exist yet.~~ **Corrected 2026-11-08:** the boundary exists
  and `UNI-31` is **Done**, but not under the name or the shape this issue assumes. The
  trait is `universe_core::adapter::Adapter` (`crates/universe-core/src/adapter.rs`) with
  `capabilities`, `plan`, `build`, and `audit` — **no `lint` method**. Lint never crosses
  the adapter boundary: `universe lint` reads the project manifest and calls
  `universe_conda::lint_recipe` directly, so the Conda rules are already a special case in
  the verb rather than a capability of the adapter.
- ~~`universe-kit lint` does not exist; `universe-kit` reports version only.~~ **Corrected
  2026-11-08:** `universe lint` exists (`crates/universe-kit/src/lint.rs`): the manifest's
  own validation, workspace/recipe version parity, and the recipe literals
  (`schema_version`, `context.version`, `package.name`) through `lint_recipe`, with
  `--json`. What it lacks is a rule engine — the rules are Rust, the diagnostics are built
  by hand, and there is no ruleset a new ecosystem can add without an edit.
- The issue's examples reference files this repository does not have: `python/pyproject.toml`,
  `crates/universe-core/lint-rules/`, `python/dist/*.whl`. **Still true.**
- The issue's DoD asks to update "Spec 0005-equivalent (decision-13 §2)"; decision-13
  has no spec numbering here, so that reference must be restated against the actual file.
- Design consequence, sharpened by the correction: the question is no longer "move match
  arms into rulesets". It is "does `Adapter` grow a `lint` seam, or does the ruleset live
  beside the manifest in `universe lint` and dispatch by ecosystem?" Adding `lint` to the
  trait would make every adapter carry a rule engine it does not need for plan/build/audit.
- **New on 2026-11-08:** `alint` is now the repository's declarative layer for repository
  shape (`decision-121`), with 80 rules and a policy-test suite. That answers the question
  for *this checkout* but not for the product: `alint` lints the repository it is run in,
  while an adapter ruleset has to ship inside `universe-kit` and lint a *consuming*
  repository. Reusing alint means the consumer needs its binary at lint time.

## Prerequisites

- `UNI-31` (adapter boundary and conformance fixtures) must land first, or the trait
  shape must be re-derived in this proposal.
- A decision recording the declarative rule model and its division of labour with the
  `EcosystemAdapter` trait, per decision-13.
- `UNI-7` (`universe-kit lint`) provides the command surface the rulesets plug into.

## Recommended disposition

Draft stays open until `UNI-31` is decided; then re-scope as part of `UNI-7` rather than
as a separate task.

## Promoted (2026-11-08)

Opened as `UNI-66`, in m-2, after `UNI-7`. The corrected reality check above is the task's
starting point: the boundary is done but has no lint seam, the lint verb exists but has no
rule engine, and where the rules belong is the task's first decision.
