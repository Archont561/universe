---
id: UNI-78
title: Render and check only the lanes the manifest declares
status: Done
assignee: []
created_date: '2026-11-13'
updated_date: '2026-11-13'
labels:
  - architecture
  - ci
  - infrastructure
milestone: m-2
dependencies:
  - UNI-76
documentation:
  - backlog/tasks/uni-57 - Render-the-workflow-set-from-the-manifest.md
priority: medium
type: bug
ordinal: 78000
---

## Description
<!-- SECTION:DESCRIPTION.BEGIN -->
GitHub issue #122 reports that `derived_files` always includes `MAIN_FILE`, so a consumer that
declares only `[airlock.developer]` — no packages, no stores — still gets `universe.yml`.
Verified on `40bcf1ff`: `let mut files = vec![MAIN_FILE];`. The rendered producer lane cannot
pass in that repository: its `build` job uploads with `if-no-files-found: error` while
`universe build` over zero declared packages produces nothing, and the file also fires on
`pull_request` and on `push.tags: v*`, adding always-red required checks and colliding with any
tag-owned publish lane.

The rule does not need a new knob. `universe.yml` is the package lane, and a manifest that
declares no `[[packages]]` is saying it has no package lane — the file set is already derivable
from what the manifest declares; it is `MAIN_FILE` alone that is not.

The issue also records a second half: `ci check` cannot be scoped, so a consumer cannot drift-check
the Airlock file without committing the producer file. Once `derived_files` follows the manifest,
that half goes away with it — the check compares what the manifest calls for, and a lane nobody
declared is not something to compare.

Seen by a consumer (`ds`) as: `bun run tools check` red on `lint`/`actionlint` the moment the file
is committed, worked around by deleting the file after every render and re-implementing the drift
check out of band.
<!-- SECTION:DESCRIPTION.END -->

## Acceptance Criteria
<!-- AC.BEGIN -->
- [x] #1 `derived_files` includes `universe.yml` only when the manifest declares at least one `[[packages]]`; a manifest with none renders no package lane. — `a_manifest_with_no_packages_renders_no_package_lane`, red first at `["universe.yml", "universe-airlock.yml"]` against the expected `["universe-airlock.yml"]`.
- [x] #2 A manifest with `[[packages]]` and no writable stores still renders `universe.yml`, so build-and-test without publication keeps working; the existing variant fixtures keep their file sets or their expectations are corrected with the reason recorded. — `a_package_lane_that_publishes_nothing_still_renders`; every existing fixture kept its file set, because each of them declares packages.
- [x] #3 `ci render`, `ci check`, `--stdout` and `--json` all name the same set, so a consumer reading any of the three learns exactly which files the manifest calls for. — all four read `derived_files`; the CLI test `a_foreign_release_workflow_survives_a_render_and_is_reported` renders an Airlock-only project and `ci check` accepts its own output.
- [x] #4 The reproducer from the issue is a fixture: a manifest declaring only `[airlock.developer]` renders exactly `universe-airlock.yml`, and the file is asserted by name rather than by count. — the unit test, plus the BDD scenario "A consumer that declares no packages gets no producer lane", which asserts the file set *and* that `universe.yml` is absent from disk.
- [x] #5 A `universe.yml` left behind by an earlier render is removed when the packages that called for it go away, subject to the ownership rule `UNI-76` establishes. — `dropping_the_package_lane_removes_the_file_it_rendered`, which was red until the `derived_files` change landed.
<!-- AC.END -->

## Definition of Done
<!-- DOD.BEGIN -->
- [x] #1 The rendered set for every variant in `TESTS/test_rendered_workflows.py` is asserted, including the new no-packages one, and each rendered file still passes `actionlint`. — the file is `scripts/tests/test_rendered_workflows.py` in this repository; every existing variant declares packages, so all of them stayed as they were, and `pixi run lint-actions` is green over the re-rendered files. The new no-packages variant is asserted in Rust rather than duplicated in Python, because the Python suite renders manifests it chooses and the Rust fixture states the domain case.
- [x] #2 `pixi run gates` passes and this repository's own rendered files are unchanged, since it declares four packages. — `release.yml` and `universe.yml` both changed, but from `UNI-79`'s checkout flag: this manifest's file *set* is unchanged, which is what this task owns.
- [x] #3 `docs/WORKFLOWS.md` states that the file set is the manifest's, which file belongs to which lane, and that a lane nobody declares is not rendered. — the "file set is the set of lanes the manifest declares" bullet plus the corrected "generated only if you declared the capability" bullet, which previously read "no stores, no publish file" and implied every other file was unconditional.
<!-- DOD.END -->
