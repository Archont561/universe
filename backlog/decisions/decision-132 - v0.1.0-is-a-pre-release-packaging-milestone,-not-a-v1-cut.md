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

1. Three `linux-64` Conda packages: `universe-airlock`, `universe-kit`, and the umbrella
   `universe` facade. Each version is asserted against the workspace rather than restated in
   its recipe. The two main-CLI packages joined the checkpoint in the amendment below.
2. Airlock's verify and restore skeleton (`UNI-32`, `UNI-33`) present, so the installed
   binary does something a consumer would want. A package whose only behaviour is `--version`
   is a pipeline proof, and calling it a product release would be the mislabelling this record
   exists to prevent.
3. Publication to all declared destinations: a hosted channel (`prefix-dev`), Universe's own
   static channel on a Git branch, and GitHub Releases, all generated and published by
   Universe rather than a stand-in tool. GitHub Releases was added by the amendment below.
4. The workflow that publishes it is rendered from `universe.toml` (`decision-124`), not
   hand-written, because the renderer exists before the release does and the bootstrap
   argument for hand-writing the first file does not apply.

Explicitly out of scope, and staying with `decision-119`: Cargo, Python wheels, npm, the
browsable portal, and any platform beyond `linux-64`. The main-CLI amendment below adds
`universe-kit` and the umbrella `universe` facade to this pre-release only; their four-platform
and integrated v1 work remains with `UNI-15`, `UNI-16`, and `UNI-40`.

Three things keep this honest:

- **The tag is a pre-release marker in the changelog and in the release notes.** It does not
  claim the v1 surfaces.
- **`decision-119` is not superseded.** This record narrows the *schedule*, and the v1
  acceptance gate stands exactly as written.
- **No v1 acceptance surface is relaxed.** Static-channel publication, the store/package
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
   scoped to this checkpoint, to the three declared `linux-64` packages, and to a retention
   rule that is an acceptance criterion rather than a later improvement. When the channel outgrows a
   branch, `UNI-11`'s hosted publication is already the general path.

The later amendments enlarge only the checkpoint's package and destination lists. The v1
gate in `decision-119` still requires all six surfaces, and `UNI-59`'s notes must state the
pre-release status rather than borrow them.

## Amendment: GitHub Releases is a third destination

The owner asked why no GitHub Release existed after the green publish run. The answer was
that this record scoped `v0.1.0` to two destinations, while `decision-127` still called
`releases` an implicit destination and `universe.toml` set `release = true`. Leaving that
contradiction standing would promise a release the checkpoint did not ship.

**This amendment widens the scope, and is recorded as a reversal.** `v0.1.0` now declares
three destinations: `prefix-dev`, the Git-branch static channel, and GitHub Releases.
`universe.toml` declares one `releases`-kind store carrying `Archont561/universe` and the
built-in `GITHUB_TOKEN`; the renderer gives it a `publish / releases` job for every package
with `release = true` without the package naming it. `decision-127` records the correction
to how its implicit rule is implemented. The v1 gate in `decision-119` is still not
superseded: this adds a package destination, not a product surface. `UNI-59` AC#4 and DoD #3
are reworded to cover all three destinations and stay open until each is verified after the
tag run. No `v0.1.0` tag is cut by this amendment.

The first re-watch after the amendment created the rolling GitHub Release
`packages-linux-64 artifacts` with the `universe-airlock` asset, but its verdict was red:
`publish / prefix-dev` refused the rebuilt package with a 409 because the same filename
carried a different digest. That is the non-reproducible-artifact defect already recorded in
`UNI-59`, not a failure of the new destination. It is recorded here so the amendment is not
read as a green end-to-end release: the third destination exists and published, while the
checkpoint's two-destination parity and offline-solve criteria remain open.

## Amendment: the rendered artifact uploads overwrite a changed digest

The owner chose `--force` for `prefix-dev` and `releases` after the first re-watch failed with
a 409: a rebuild of `universe-airlock-0.1.0-h41f06ac_0.conda` carried a different digest under
the same filename, and the hosted channel refused it. A later run would also have been refused
by the rolling GitHub Release asset.

**This is a reversal, and is recorded as a reversal.** Invariant 3 of `decision-126` says a
second publish of one `(release-id, target)` is an error rather than an overwrite, and the
`publish-artifacts` CLI still refuses a changed digest unless `--force` is given. For the
`v0.1.0` rendered publish lane only, `ci render` now adds that flag to the `publish-artifacts`
commands of `prefix-dev` and `releases` stores: prefix.dev receives `?force=true`, and the
rolling release deletes and re-uploads the occupied asset. `publish --store branch` is
unchanged; it replaces one snapshot tree and takes no overwrite flag. This does not amend the
`universe release` journal semantics, does not fix the non-reproducible-artifact defect, and
does not supersede the v1 gate in `decision-119`. A green verdict proves that one run's three
destinations agree on one digest; it does not prove that every rebuild produces those bytes.
No `v0.1.0` tag is cut by this amendment.

The first re-watch after this change moved the failure rather than closing it. Run
`42920755513` from `main` at `f86617cf` had green `publish / build`, `publish / releases` and
`publish / branch` jobs, but `publish / prefix-dev` answered `403 Not authorized` at
`https://prefix.dev/api/v1/upload/archont561/archont561?force=true`, so `publish / verdict`
stayed red. The URL proves the flag reached the endpoint. `https://prefix.dev/docs/prefix/api`
states that force-replacing an occupied filename requires a scoped key with read/write/delete,
and that a Repository Access entry must carry the same read-write-delete mode to grant it. The
renderer change is complete. The owner then removed the old package variant manually and
reran the failed jobs of the same run; attempt 2 finished green at `2026-11-10T23:23:11Z`
with `publish / prefix-dev` and `publish / verdict` successful. This proves that a clean
prefix.dev channel accepts the forced upload; it does not prove that the current Repository
Access entry can force-replace an occupied filename, and it does not make the build
reproducible. `UNI-59` records both attempts, the common artifact and digests, and the
still-open acceptance criteria. No `v0.1.0` tag is cut.

## Amendment: the main CLI packages are in scope

The owner asked why only Airlock was published. The answer was the out-of-scope line above:
`v0.1.0` declared one package while the repository already shipped a producer CLI and an
umbrella facade. Leaving that contradiction standing would promise a main CLI the checkpoint
did not publish.

**This amendment widens the scope, and is recorded as a reversal.** `v0.1.0` now declares
three `linux-64` Conda packages in `universe.toml`: `universe-airlock`, `universe-kit`, and
the umbrella `universe` facade. The rendered `release.yml` builds and publishes all three to
`prefix-dev`, the Git-branch static channel, and GitHub Releases. The facade declares
`depends_on = ["universe-kit", "universe-airlock"]` so `plan` orders its build after both
components; `package-cli` skips rattler-build's post-build test because those run dependencies
are not in the channel until the same release publishes them. The workspace gates prove the
facade binary itself.

This does not supersede `decision-119`, does not close `UNI-15`, `UNI-16`, or `UNI-40`, and
does not make the artifacts reproducible. Those tasks still own four-platform packaging and
the integrated v1 release; one `linux-64` pre-release is not their completion. `UNI-59` AC#4
is reworded to cover all three packages at all three destinations and stays open until the
tag run proves them. No `v0.1.0` tag is cut by this amendment.

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
