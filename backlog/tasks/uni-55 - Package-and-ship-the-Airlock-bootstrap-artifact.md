---
id: UNI-55
title: Package and ship the Airlock bootstrap artifact
status: To Do
assignee: []
created_date: '2026-10-31'
updated_date: '2026-11-12'
labels:
  - airlock
  - bootstrap
  - release
milestone: m-5
dependencies:
  - UNI-33
  - UNI-38
documentation:
  - backlog/decisions/decision-115 - Bootstrap-readiness-is-distinct-from-sealed-offline-readiness.md
  - backlog/decisions/decision-122 - One-product-two-editions.md
  - backlog/decisions/decision-127 - Manifest-facets-packages-stores-and-airlock-scopes.md
priority: high
type: chore
ordinal: 55000
---

## Description
<!-- SECTION:DESCRIPTION.BEGIN -->
Close a shipping gap: `crates/universe-airlock-bootstrap` builds a binary, but it has no
recipe, no run dependency anywhere, and therefore never reaches a channel or a release
asset. The workspace describes it as a distinct artifact that other records depend on,
so the absence is an inconsistency rather than a deliberate omission.

Package and ship it as its own artifact. It stays deliberately small: it verifies a
local payload and activates a restore, and it must not acquire producer, registry, or
network capability. It is a phase of the Airlock edition rather than a third product,
so it shares the edition's version and release train and carries no independent
version line.

Also decide and record whether it is a separately installable package or a bundled
asset of the consumer edition, and make the answer visible where consumers look — the
release assets and the umbrella package's runtime requirements.
<!-- SECTION:DESCRIPTION.END -->

## Acceptance Criteria
<!-- AC.BEGIN -->
- [x] #1 A recipe builds the bootstrap executable for every supported target and installs only that executable; its runtime requirements add no producer, registry, or network tooling. — 2026-10-31: `crates/universe-airlock-bootstrap/recipe/` plus `pixi run package-airlock-bootstrap`; `repo_check` now enforces its version parity.
- [ ] #2 The artifact is smoke-tested in a clean environment and its version contract matches the workspace version.
- [ ] #3 The release lane publishes it alongside the consumer artifact with checksums, and the version policy in `decision-126` applies to it as part of the same train.
- [x] #4 Whatever the packaging decision is, the consumer edition's documentation and the release asset list agree with it. — 2026-11-12: the decision is **separately installable**, recorded in `decision-132`'s fourth-package amendment. `README.md`, `docs/releases/0.1.0.md` and `universe.toml` all call it separately installable on the same version and train, with no runtime dependency; the facade does not depend on it. `scripts/tests/test_release_scope.py::test_the_bootstrap_is_a_separately_installable_package` and `::test_the_consumer_docs_and_the_asset_list_agree_on_the_bootstrapper` assert the agreement, and both were red before the manifest entry and the prose existed.
- [ ] #5 The public verification key is compiled into the bootstrap artifact and its rotation procedure — a bootstrap signed by both the old and the new key — is written down and exercised on a fixture (`decision-131`).
<!-- AC.END -->

## Definition of Done
<!-- DOD.BEGIN -->
- [ ] #1 No artifact referenced by another Backlog record is missing a shipping path.
- [ ] #2 `pixi run repo-check` and `pixi run gates` pass, with recipe/workspace version parity enforced.
<!-- DOD.END -->

## Notes
<!-- SECTION:NOTES:BEGIN -->
- **2026-11-12 — the shipping gap is declared closed, and how.** `UNI-59` could not gain a
  restore entrypoint without this: the entrypoint clones a pinned commit and executes
  `universe-airlock-bootstrap` from that checkout, and no ref carried the binary — `channel` is a
  Conda tree, and no branch holds a bootstrapper. The recipe and `pixi run
  package-airlock-bootstrap` already existed, so the missing piece was a manifest declaration and
  the workflow it renders.
  `universe.toml` now declares `universe-airlock-bootstrap` with `kind = "conda"`, `linux-64`
  only, `outputs = ["output/universe-airlock-bootstrap/linux-64/*.conda"]`, a smoke command, the
  same `publish` stores as its siblings, and `release = true`; `ci render` put one
  `publish-artifacts` step for it under `prefix-dev` and `releases` and added it to the single
  branch snapshot. It carries no `depends_on`, and the facade does not name it.
  **Why no `--test=skip`:** the facade's task has one because its run dependencies are not in the
  channel until the same release publishes them. The bootstrapper declares no run requirements at
  all, so rattler-build's post-build test has a solvable environment and is worth keeping — it is
  the clean-environment smoke that criterion 2 asks for.
- **What is still open here.** Criterion 2 needs that post-build test to run and pass and its
  output recorded; criterion 3 needs the publish job's read-back at all three destinations, with
  checksums; criterion 5 needs a release key, which is `decision-131`'s remaining work, not a
  packaging change. The umbrella edition's own documentation is the facade's run requirements:
  they name `universe-kit` and `universe-airlock` and deliberately not the bootstrapper, so
  installing the facade never implies a binary that runs before an environment exists.
<!-- SECTION:NOTES.END -->
