---
id: UNI-59
title: Cut the v0.1.0 pre-release from the rendered workflow
status: To Do
assignee: []
created_date: '2026-11-01'
updated_date: '2026-11-01'
labels:
  - release
  - airlock
  - packaging
milestone: m-12
dependencies:
  - UNI-38
  - UNI-53
  - UNI-57
  - UNI-58
documentation:
  - backlog/decisions/decision-132 - v0.1.0-is-a-pre-release-packaging-milestone,-not-a-v1-cut.md
  - backlog/decisions/decision-119 - Full-multi-ecosystem-v1-and-static-catalog-distribution.md
  - backlog/decisions/decision-126 - Release-control-lives-in-Universe-with-attempts-promotion-and-a-resumable-journal.md
  - backlog/docs/roadmap/doc-112 - Ordered-milestone-DAG-and-sprint-train.md
priority: high
type: feature
ordinal: 59000
---

## Description
<!-- SECTION:DESCRIPTION.BEGIN -->
Cut the tag, publish from the rendered workflow, and prove the loop from the outside.

This is a release task, not a build task: everything it needs is built by the tasks its
dependencies name. What it adds is the part a release actually consists of — a version
promoted once and asserted everywhere, a tag, notes that say what the tag is and is not,
two destinations verified after the fact, and a restore transcript that came from a clean
host rather than from the runner that produced the package.

The wording carries weight. `decision-132` makes `v0.1.0` a pre-release checkpoint, so
the notes must not read like a v1 announcement and must not borrow the v1 surface list.
An installed `universe-airlock` that can verify and restore a snapshot is a real thing a
consumer can use; it is not Conda, Cargo, Python wheels, npm, the portal, and Airlock
together, and the release notes say which of those is true.

Version authority stays where `decision-126` put it: the workspace version is a declared
literal, promoted by a human, and the parity between it, the recipe, and the changelog is
a repository check rather than a release-time computation.
<!-- SECTION:DESCRIPTION.END -->

## Acceptance Criteria
<!-- AC.BEGIN -->
- [ ] #1 The workspace version, the `universe-airlock` recipe version, and the changelog's top section agree on `0.1.0`, and the existing parity check is what proves it rather than a manual reading.
- [ ] #2 The `v0.1.0` tag is cut from a commit whose full gate run is green, and the tag is annotated with the same message the release notes open with.
- [ ] #3 Release notes state the pre-release status and the `linux-64`-only scope, name what `universe-airlock` actually does at this tag (verify, restore, version), and claim none of `decision-119`'s six v1 surfaces.
- [ ] #4 Both destinations are verified after the run: the branch channel solves for `universe-airlock=0.1.0` from a local checkout with network denied, and the prefix.dev channel resolves the same artifact by digest.
- [ ] #5 A clean container with only Git and Pixi runs the rendered restore entrypoint against the tag and prints `universe-airlock version`; the transcript is recorded in this task's notes, with the container image named.
- [ ] #6 A repository check fails if the changelog's top section and the tag disagree, so the next release cannot drift by memory.
<!-- AC.END -->

## Definition of Done
<!-- DOD.BEGIN -->
- [ ] #1 The rendered workflow's verdict job is green for the tag run, and its link is in the notes.
- [ ] #2 `pixi run gates` passes on the tagged commit.
- [ ] #3 The task records both destination URLs and the restore transcript.
<!-- DOD.END -->
