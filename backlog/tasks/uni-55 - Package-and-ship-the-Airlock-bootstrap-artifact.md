---
id: UNI-55
title: Package and ship the Airlock bootstrap artifact
status: To Do
assignee: []
created_date: '2026-10-31'
updated_date: '2026-10-31'
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
- [ ] #1 A recipe builds the bootstrap executable for every supported target and installs only that executable; its runtime requirements add no producer, registry, or network tooling.
- [ ] #2 The artifact is smoke-tested in a clean environment and its version contract matches the workspace version.
- [ ] #3 The release lane publishes it alongside the consumer artifact with checksums, and the version policy in `decision-126` applies to it as part of the same train.
- [ ] #4 Whatever the packaging decision is, the consumer edition's documentation and the release asset list agree with it.
<!-- AC.END -->

## Definition of Done
<!-- DOD.BEGIN -->
- [ ] #1 No artifact referenced by another Backlog record is missing a shipping path.
- [ ] #2 `pixi run repo-check` and `pixi run gates` pass, with recipe/workspace version parity enforced.
<!-- DOD.END -->
