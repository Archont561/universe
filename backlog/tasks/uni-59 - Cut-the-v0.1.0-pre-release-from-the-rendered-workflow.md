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

## Notes
<!-- SECTION:NOTES.BEGIN -->
- **2026-11-08 — the publish path can be rehearsed without a tag, and how far that goes.**
  `crates/universe-kit/tests/publish_mock.rs` runs the real `publish-artifacts` verb against a
  destination the test owns: the manifest is parsed, the fragment is read, the artifact's
  digest is verified, the endpoint comes from the store's `url`, and the upload goes over a
  loopback socket. It proves the three properties a tag run depends on — the bytes are posted
  to the address the manifest declares, a destination reporting the same digest receives no
  upload, and a destination holding a different digest is refused rather than overwritten.
  Because `Target` takes its endpoint from `store.url` when one is present, no second code
  path exists for the mock: what is exercised is what the workflow runs.
- **What the rehearsal cannot prove, and therefore does not tick AC#4:** that prefix.dev
  accepts the artifact and that GitHub's OIDC exchange mints a token at all. Both need the
  real destination.
- **2026-11-08 — fixed: the publish file builds what it publishes.** The finding that made a
  tag meaningless is closed. `universe-publish.yml` had no build job, and
  `actions/download-artifact` reads only its own run's artifacts, so every publish job would
  have downloaded an empty directory: a green pipeline that published nothing. The renderer
  now emits the same build job in both files (`build_job` in
  `crates/universe-workflow/src/lib.rs`), every store job carries `needs: [build]`, and the
  publish verdict counts the build, so a failed build cannot read as a successful release.
  `the_publish_file_builds_what_it_publishes` fails without it — verified by removing the
  build and watching it fail. A tag run can now be exercised end to end against the mock
  destination from the rehearsal above.
- **A layout fact the rehearsal surfaced:** for `kind = "prefix-dev"`, `Target::location`
  addresses everything at `api/v1/upload/<channel>`, so `apply`'s per-artifact existence check
  degenerates to one check per channel. With one package in the channel that is exact; with
  two, the second artifact's digest is compared against the first's answer. Worth deciding
  before the channel holds more than one package.
- **2026-11-09 — one owner-side prerequisite removed.** The `publish / branch` job now
  exports `BRANCH_GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}` from `secrets = ["GITHUB_TOKEN"]`
  in `universe.toml`. It is the short-lived token GitHub injects into every run, and the job
  already declares `contents: write`, so no `UNIVERSE_CHANNEL_TOKEN` repository secret needs to
  be created. The prefix.dev trusted publisher binding (OIDC) is still owner-side and is still
  unproven by the loopback rehearsal above. `decision-127` records why the built-in token is
  sufficient and when a future ruleset would change that answer.
<!-- SECTION:NOTES.END -->
