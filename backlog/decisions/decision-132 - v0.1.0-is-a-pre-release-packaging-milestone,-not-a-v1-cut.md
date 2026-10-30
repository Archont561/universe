---
id: DECISION-132
title: v0.1.0 is a pre-release packaging milestone, not a v1 cut
status: accepted
created_date: '2026-10-31'
updated_date: '2026-11-01'
---

## Context

`decision-119` retired the staged "headless `v0.1.0`, portal `v0.2.0`" schedule and said a
stable v1 is not complete until Conda, Cargo, Python wheels, npm, Airlock consumption, and the
browsable portal all pass acceptance. That record is still right about v1, and nothing here
reopens it.

The owner then asked for something narrower: a `v0.1.0` that ships Airlock as a real Conda
package to prefix.dev *and* to a static channel, on one platform, with Airlock actually able to
verify and restore. Every one of those capabilities is pre-existing work in the backlog
(`UNI-10`, `UNI-11`, `UNI-12`, `UNI-32`, `UNI-33`, `UNI-38`), and none of them is a v1
acceptance surface on its own. Without a record, a `v0.1.0` tag would read as a partial v1
release and would contradict a decision that is still accepted.

## Decision

`v0.1.0` is a **pre-release packaging milestone**. It cuts a versioned tag, publishes real
artifacts to real destinations, and is explicitly *not* the integrated v1 release of
`decision-119`.

The scope of `v0.1.0` is:

1. `universe-airlock` packaged as a Conda package, `linux-64` only, with its version asserted
   against the workspace rather than restated in the recipe.
2. Airlock's verify and restore skeleton (`UNI-32`, `UNI-33`) present, so the installed
   binary does something a consumer would want. A package whose only behaviour is `--version`
   is a pipeline proof, and calling it a product release would be the mislabelling this record
   exists to prevent.
3. Publication to both store kinds: a hosted channel (`prefix-dev`) and Universe's own static
   channel, both generated and published by Universe rather than a stand-in tool.
4. The workflow that publishes it is rendered from `universe.toml` (`decision-124`), not
   hand-written, because the renderer exists before the release does and the bootstrap
   argument for hand-writing the first file does not apply.

Explicitly out of scope, and staying with `decision-119`: Cargo, Python wheels, npm, the
browsable portal, `universe-kit` publication, the umbrella `universe` facade, and any platform
beyond `linux-64`.

Three things keep this honest:

- **The tag is a pre-release marker in the changelog and in the release notes.** It does not
  claim the v1 surfaces.
- **`decision-119` is not superseded.** This record narrows the *schedule*, and the v1
  acceptance gate stands exactly as written.
- **Nothing already accepted is relaxed.** Static-channel publication, the store/package
  mapping, and the per-store publish jobs all follow `decision-127` and `decision-128`.

## Amendment (2026-11-01): the checkpoint is a closed loop, and its channel rides a branch

The owner restated what they want `v0.1.0` to *do*: render the Airlock release workflow,
publish its Conda packages to branches, hand a consumer a rendered script that restores
them, and leave a developer able to build Conda packages locally against that channel.
That is narrower than "publishes real artifacts to real destinations" and more useful,
because it names the consumer at the end of the pipeline.

The scope above stands. Three things are now stated rather than implied:

1. **The checkpoint is one loop, not a checklist.** Render → publish → restore → develop,
   in that order, each beat observable from outside the repository. `m-12` records the
   four beats and their exit condition; `UNI-59` cuts the tag only after all of them are
   demonstrated together.
2. **The static channel for `v0.1.0` is a Git branch.** Item 3 above requires Universe's
   own static channel; `decision-5` leaves the host open ("static hosting or object
   storage") and this record picks the one that needs no HTTP endpoint in front of it, so
   a disconnected host with only Git can restore. `UNI-58` owns the transport. This is a
   choice of host, not a reversal of `decision-5`, which continues to require standard
   subdir paths and the package bytes at them.
3. **The cost of that choice is recorded, not discovered.** A branch is a poor CDN: every
   build grows the repository and cloners pay for history. The transport is therefore
   scoped to this checkpoint, to one package, and to a retention rule that is an
   acceptance criterion rather than a later improvement. When the channel outgrows a
   branch, `UNI-11`'s hosted publication is already the general path.

Nothing here enlarges the release: the v1 gate in `decision-119` still requires all six
surfaces, and `UNI-59`'s notes must state the pre-release status rather than borrow them.

## Consequences

- The v1 gate continues to require all six surfaces; `v0.1.0` is a checkpoint on the Conda
  track, not a shortcut.
- `UNI-10`, `UNI-11`, `UNI-12`, `UNI-38` are delivered in slices that a `v0.1.0` needs, and
  each slice records which of its acceptance criteria are deferred and why. A narrowed
  acceptance criterion that is ticked without that note is the dishonesty `backlog/README.md`
  warns about.
- A `v0.1.1` or later may add platforms without re-planning this record.
- The first release workflow is rendered by `pixi run ci-render` once this repository's own
  `universe.toml` exists (`UNI-41`).
