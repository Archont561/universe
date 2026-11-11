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
- [x] #3 Recipe version and package identity are checked against the Cargo workspace and pass repository gates. — 2026-11-09: `scripts/repo_check.py` already compared `context.version` with `[workspace.package].version`; it now also compares the recipe's `package.name` with the crate's own `Cargo.toml` `package.name`. `test_a_recipe_name_must_agree_with_its_crate_manifest` is red without the second comparison, and `pixi run repo-check` is green.
- [ ] #4 Release CI builds, tests, and publishes the Airlock package as its own recipe alongside `universe-kit` and the umbrella `universe` facade; the bootstrap artifact remains a separate build.
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

2026-11-09 — AC#3 is ticked, and the reason it was open is recorded. The existing recipe
check compared `package.name` against a hardcoded crate-directory name, which is not the
Cargo workspace's identity; a recipe could agree with the directory and still name a package
the crate manifest does not declare. The check now reads `crates/<name>/Cargo.toml` itself.
`crates/universe-cli` is deliberately outside that loop: its recipe names the `universe`
facade rather than the crate, and the existing facade check already handles it. AC#1 (every
native target) and AC#4 (the release CI shape) stay open for the reasons above.
- **Status note — pre-release scope widened, recorded as a reversal.** The main-CLI
  amendment to `decision-132` adds `universe-kit` and the umbrella `universe` facade to the
  `v0.1.0` publish lane. AC#4 is reworded so it no longer claims Airlock is the only package
  in release CI; Airlock still has its own recipe, and the bootstrap artifact is still built
  separately. The criterion stays open: every native target and a clean consumer install with
  network denied are not proven by one `linux-64` dispatch. The `v0.1.0` tag was cut on
  2026-11-11 (see the correction below).
**Status note — correction (2026-11-11).** The annotated `v0.1.0` tag was cut from `1de18891bd69f8d16b4d261cfdaa53525555eabd` and pushed; its tag-triggered `release.yml` run `43073492712` is green, including `publish / verdict`. https://github.com/Archont561/universe/actions/runs/43073492712 This records the tag as evidence, not as a completion of this record's criteria: the tasks above remain To Do. The offline branch solve with network denied, a clean-container restore transcript, and the four-platform and integrated v1 lanes are still open. `UNI-59` records the tag run's digests and the remaining gaps. No further tag is cut.
<!-- NOTES.END -->
