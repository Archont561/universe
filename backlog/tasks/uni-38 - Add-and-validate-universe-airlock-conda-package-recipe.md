---
id: UNI-38
title: Add and validate the Universe Airlock Conda package recipe
status: In Progress
assignee: []
created_date: '2026-10-27'
updated_date: '2026-11-09'
labels:
  - release
  - airlock
  - packaging
milestone: m-5
dependencies:
  - UNI-31
priority: high
type: feature
ordinal: 38000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
A co-located `crates/universe-airlock/recipe/recipe.yaml` and platform variant file now provide an initial MIT-licensed recipe scaffold for the finite offline consumer. Complete packaging and release integration without adding producer, network, Cargo-registry, or Conda runtime capabilities to the Airlock artifact. The bootstrap executable remains a separate, smaller artifact.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The recipe builds `universe-airlock` from the locked workspace for every supported native target and installs only the Airlock executable.
- [x] #2 Conda package tests verify the version contract and representative offline-consumer behavior; runtime requirements do not add producer/build/network tooling.
- [ ] #3 Recipe version and package identity are checked against the Cargo workspace and pass repository gates.
- [ ] #4 Release CI builds, tests, and publishes the Airlock package independently from `universe-kit` and the bootstrap artifact.
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 Native package builds and CI release checks pass for every supported target.
- [ ] #2 A minimal consumer install passes with network access denied.
<!-- DOD:END -->

## Notes
<!-- NOTES.BEGIN -->
**2026-11-09 — the installed package now does something a consumer wants.** The recipe keeps
the `--version` check and adds a second script test that runs
`universe-airlock verify --payload payload` against a closed two-file fixture copied from
`recipe/payload/`. The fixture's digest is written down from an independent source, and
`crates/universe-airlock/tests/verify.rs` runs the same verification over that fixture from
the source tree, so a drifted fixture fails the Rust suite before it reaches a package build.
`rattler-build` is not part of the restored developer sandbox, so the package test itself is
not executed here; it is exercised at the `v0.1.0` tag run.

AC#1 (every supported native target) remains open and is out of scope for the checkpoint:
`decision-132` makes `v0.1.0` `linux-64` only. AC#4 (independent release CI) is likewise a
release-lane task. The flags in `universe.toml` that gate `universe-kit`'s verify verbs stay
off; they are not this consumer's verb.
<!-- NOTES.END -->
