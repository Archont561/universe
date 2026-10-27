---
id: UNI-38
title: Add and validate the Universe Airlock Conda package recipe
status: To Do
assignee: []
created_date: '2026-10-27'
updated_date: '2026-10-27'
labels:
  - release
  - airlock
  - packaging
milestone: m-8
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
- [ ] #2 Conda package tests verify the version contract and representative offline-consumer behavior; runtime requirements do not add producer/build/network tooling.
- [ ] #3 Recipe version and package identity are checked against the Cargo workspace and pass repository gates.
- [ ] #4 Release CI builds, tests, and publishes the Airlock package independently from `universe-kit` and the bootstrap artifact.
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 Native package builds and CI release checks pass for every supported target.
- [ ] #2 A minimal consumer install passes with network access denied.
<!-- DOD:END -->
