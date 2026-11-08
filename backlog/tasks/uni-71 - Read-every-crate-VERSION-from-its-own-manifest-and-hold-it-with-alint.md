---
id: UNI-71
title: Read every crate's VERSION from its own manifest and hold it with alint
status: Done
assignee: []
created_date: '2026-11-08'
updated_date: '2026-11-08'
labels:
  - infrastructure
  - gates
  - release
milestone: m-0
documentation:
  - backlog/decisions/decision-122 - One-product-two-editions.md
  - backlog/decisions/decision-126 - Release-control-lives-in-Universe-with-attempts-promotion-and-a-resumable-journal.md
priority: medium
type: chore
ordinal: 71000
---

## Description
<!-- SECTION:DESCRIPTION.BEGIN -->
`decision-126` makes the workspace version a declared literal that a human promotes, and
`decision-122` makes one version cover the whole artifact set. Neither says where a crate
learns its version, so three answers coexist: `env!("CARGO_PKG_VERSION")` in six crates,
`pub const COMPONENT_VERSION` under a second name in `universe-workflow`, and a hand-typed
`pub const VERSION: &str = "0.1.0"` in `universe-assets` and `universe-cli`. Two crates,
`universe-channel` and `universe-conda`, do not say anything at all. `universe-assets` also
pins `version = "0.1.0"` in its own `Cargo.toml` instead of inheriting the workspace's.

A release bump is then three edits in three files, and the only thing that notices a missed
one is a test inside the crate that was already forgotten.

The fix is the smallest possible chain: the root `Cargo.toml` holds the version, every crate
manifest inherits it with `version.workspace = true`, and every crate's entrypoint exports
`pub const VERSION: &str = env!("CARGO_PKG_VERSION")`. `env!` is read by the compiler from
that crate's own manifest, so a crate cannot compile against a version other than the one it
declares. `alint` holds both links, because both are repository shape rather than behaviour.
<!-- SECTION:DESCRIPTION.END -->

## Acceptance Criteria
<!-- AC.BEGIN -->
- [x] #1 Every crate under `crates/` exports `pub const VERSION: &str = env!("CARGO_PKG_VERSION")` from its `src/lib.rs`, and no crate spells its version as a string literal.
- [x] #2 Every crate manifest inherits `version.workspace = true`, so a release bump is one edit in the root `Cargo.toml`.
- [x] #3 `alint` refuses a crate whose entrypoint does not export the manifest-derived constant, and refuses a crate manifest that declares its own version.
- [x] #4 Versioned-package `VERSIONS` files remain the packaging mirror they were, and the tests that hold them equal to the manifest still pass.
- [x] #5 Policy tests under `alint-tests/` fail a tree that breaks each link and pass one that satisfies both.
<!-- AC.END -->

## Definition of Done
<!-- DOD.BEGIN -->
- [ ] #1 The new rules were seen failing against this repository before the crates were fixed, and the failure output is recorded in the notes.
- [ ] #2 `pixi run alint-check` and `pixi run alint-test` pass, and `pixi run gates` is green.
- [ ] #3 `AGENTS.md` names the chain in one place, so the next crate is written that way rather than discovered by a red gate.
<!-- DOD.END -->

## Notes
<!-- NOTES.BEGIN -->
2026-11-08 — the rules were written first and run against the repository before any crate was
touched. `pixi run alint check` reported six violations and two failing rules:

```
─── crates/universe-assets/Cargo.toml ──────────────────────────────────────────
  ✗  error    crate-manifests-inherit-the-workspace-version
              A crate manifest must inherit `version.workspace = true` rather
              than declare its own version
─── crates/universe-assets/src/lib.rs ──────────────────────────────────────────
  ✗  error    crate-entrypoint-reads-its-manifest-version
─── crates/universe-channel/src/lib.rs ─────────────────────────────────────────
  ✗  error    crate-entrypoint-reads-its-manifest-version
─── crates/universe-cli/src/lib.rs ─────────────────────────────────────────────
  ✗  error    crate-entrypoint-reads-its-manifest-version
─── crates/universe-conda/src/lib.rs ───────────────────────────────────────────
  ✗  error    crate-entrypoint-reads-its-manifest-version
─── crates/universe-workflow/src/lib.rs ────────────────────────────────────────
  ✗  error    crate-entrypoint-reads-its-manifest-version
Summary (6 violations):
  ✗ 6 errors
  80 passing · 2 failing
```

What each violation was, and what it became:

- `universe-assets` declared `version = "0.1.0"` in its own manifest and repeated the literal in
  its entrypoint. Both are gone: the manifest inherits the workspace version, and the constant
  is `env!("CARGO_PKG_VERSION")`. Its `VERSIONS` file stays, and the test that holds it equal to
  the constant is what keeps the mirror honest.
- `universe-cli` had the same hand-typed constant with `version.workspace = true` already in
  place; only the constant changed.
- `universe-channel` and `universe-conda` had no constant at all. They now export one under the
  same documentation as the rest of the workspace.
- `universe-workflow` named the same value `COMPONENT_VERSION`, and imported
  `universe_core::VERSION` for the generator stamp. It now exports `VERSION`, stamps its own,
  and a unit test asserts it equals the core's — the one assertion that can still fail if a
  crate ever inherits a different literal.

The two rules are `crate-manifests-inherit-the-workspace-version` (`toml_path_equals` on
`$.package.version.workspace`) and `crate-entrypoint-reads-its-manifest-version`
(`every_matching_has` over `crates/*`, requiring the exact `env!` line in each entrypoint).
Their policy tests are `alint-tests/version/*.yml`: one tree per link broken, plus the tree
that satisfies both.

`env!` is the part that makes the chain work: the compiler resolves it from the crate's own
manifest, so a crate cannot ship a version it did not declare, and with every manifest
inheriting, the root `Cargo.toml` holds the one literal a release promotes. `AGENTS.md` house
rule 5 and `docs/DEVELOPMENT.md` §Workspace boundaries name the chain, so the next crate is
written that way instead of being caught afterwards.
<!-- NOTES.END -->
