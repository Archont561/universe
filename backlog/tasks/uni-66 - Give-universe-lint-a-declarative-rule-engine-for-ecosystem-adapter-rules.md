---
id: UNI-66
title: Give `universe lint` a declarative rule engine for ecosystem adapter rules
status: To Do
assignee: []
created_date: '2026-11-08'
updated_date: '2026-11-08'
labels:
  - architecture
  - core
  - testing
milestone: m-2
documentation:
  - backlog/drafts/draft-2 - Express-adapter-lint-rules-as-declarative-ecosystem-gated-rule-families.md
  - backlog/decisions/decision-13 - Layer-universe-core-library-and-ecosystem-adapters-under-a-single-universe-kit-CLI.md
  - backlog/decisions/decision-121 - Adopt-alint-as-the-declarative-repository-lint-layer.md
priority: medium
type: feature
ordinal: 66000
---

## Description
<!-- SECTION:DESCRIPTION.BEGIN -->
GitHub issue #14 asks for adapter lint rules to be declarative and ecosystem-gated instead of
Rust match arms. Two of its premises have moved since it was written:

- `EcosystemAdapter` is `universe_core::adapter::Adapter`, `UNI-31` is **Done**, and the
  trait has `capabilities`, `plan`, `build`, and `audit` — **no `lint`**. Lint never crosses
  that boundary: `universe lint` reads the project manifest and calls
  `universe_conda::lint_recipe` directly, so the Conda rules are a special case in the verb
  rather than something an adapter offers.
- `universe lint` exists (`crates/universe-kit/src/lint.rs`) with a `--json` mode. It is
  not a rule engine: the rules are Rust, the diagnostics are built by hand, and a new
  ecosystem means another branch in the verb.

So the first question is a design one. Does `Adapter` grow a `lint` seam — which makes every
adapter carry a rule engine it needs for nothing else — or do rulesets live beside the
manifest and dispatch by ecosystem, leaving the trait alone? `alint` is now this
repository's declarative layer (`decision-121`), but alint lints the checkout it runs in,
while an adapter ruleset must ship inside `universe-kit` and lint a *consuming* repository;
that difference is what makes reuse a decision rather than an obvious win.
<!-- SECTION:DESCRIPTION.END -->

## Acceptance Criteria
<!-- AC.BEGIN -->
- [ ] #1 A decision says where the rules live — a new `lint` seam on `Adapter`, or rulesets beside the manifest — and whether they are interpreted by `alint` or by an engine inside `universe-kit`, including what a consumer must have installed.
- [ ] #2 A ruleset format exists for at least the Conda adapter: `schema_version`, `context.version`, and `package.name` as declarative rules, not match arms, gated to `*/recipe/recipe.yaml`.
- [ ] #3 A rule for one ecosystem is inert on a repository of another — proven by a test where the Conda ruleset runs against a tree with no recipe.
- [ ] #4 `universe lint --json` keeps the current `Diagnostic`-shaped output, so a consumer sees no format change.
- [ ] #5 A new ecosystem is added by a ruleset plus its registration, with no edit to the lint verb's rule list and no change to `Adapter`.
<!-- AC.END -->

## Definition of Done
<!-- DOD.BEGIN -->
- [ ] #1 The ruleset files are committed and loaded by `universe lint`.
- [ ] #2 `universe lint` passes on this repository — the scaffold has no violations today, and the ruleset must not invent any.
- [ ] #3 `pixi run gates` passes, including the BDD scenarios for the lint verb.
<!-- DOD.END -->
