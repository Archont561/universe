---
id: UNI-59
title: Cut the v0.1.0 pre-release from the rendered workflow
status: To Do
assignee: []
created_date: '2026-11-01'
updated_date: '2026-11-10'
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
- [x] #1 The workspace version, the `universe-airlock` recipe version, and the changelog's top section agree on `0.1.0`, and the existing parity check is what proves it rather than a manual reading. — 2026-11-09: `CHANGELOG.md` has a `## 0.1.0` section whose body says pre-release and `scripts/repo_check.py::validate_release_version` compares its first versioned heading with `Cargo.toml` and the recipe; `pixi run repo-check` is green.
- [ ] #2 The `v0.1.0` tag is cut from a commit whose full gate run is green, and the tag is annotated with the same message the release notes open with.
- [x] #3 Release notes state the pre-release status and the `linux-64`-only scope, name what `universe-airlock` actually does at this tag (verify, restore, version), and claim none of `decision-119`'s six v1 surfaces. — 2026-11-10: `docs/releases/0.1.0.md` opens with the one-line annotated-tag message, states the `decision-132` checkpoint, names all three verbs with the binary that owns each, and has a `## Not a v1 release` table disclaiming all six numbered surfaces of `decision-119` as absent or single-platform. `scripts/repo_check.py::validate_release_notes` enforces the file's existence from the workspace version, the annotation's shape, the five required facts, the six surface keys, and — once a `v0.1.0` tag exists — that its `%(contents:subject)` equals the opening line. Six Python tests were red before green, and a temporary annotated tag proved the drift check both ways; the tag was deleted. AC#2 is still open: no tag has been cut.
- [ ] #4 Both destinations are verified after the run: the branch channel solves for `universe-airlock=0.1.0` from a local checkout with network denied, and the prefix.dev channel resolves the same artifact by digest.
- [ ] #5 A clean container with only Git and Pixi runs the rendered restore entrypoint against the tag and prints `universe-airlock version`; the transcript is recorded in this task's notes, with the container image named.
- [x] #6 A repository check fails if the changelog's top section and the tag disagree, so the next release cannot drift by memory. — 2026-11-09: a temporary local `v0.2.0` tag made `pixi run repo-check` exit 1 naming both values; the tag was removed and the check returned to green.
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
- **2026-11-09 — release version parity is in the repository check; no tag was cut.**
  `CHANGELOG.md` now carries `## 0.1.0`, stating the `decision-132` checkpoint,
  the `linux-64`-only scope, and that Cargo, Python wheels, npm, the portal, the facade and
  other platforms remain out of scope. The scaffold entries moved under it; `## Unreleased`
  stays above, empty. `scripts/repo_check.py` gained `validate_release_version`, which reads
  the first **versioned** `##` heading (so `Unreleased` is not read as a release), compares it
  with `[workspace.package].version` and with the recipe's `context.version`, and compares the
  highest `v*` tag when one exists. Two readings the criteria left open are now decisions:
  "top section" means the first versioned section, and a checkout with no tags is not an error
  (`git tag -l` is empty here). A failing `git tag --list` is reported rather than treated as
  "no tags". `tags` is injectable so the fixture tests need no Git repository.
  Four tests in `scripts/tests/test_repo_check.py` were each red before green; `pixi run
  gates` passes. AC#3 stays open: the changelog supplies the wording, but the release notes
  and the annotated tag message are written with the tag in AC#2.
- **2026-11-09 — correction, recorded as a correction.** The first version of this slice
  wrote `## 0.1.0 (pre-release)`. The rendered `universe / validate` job failed in CI with
  `lint.version.changelog: no CHANGELOG.md section for version 0.1.0`: `universe lint`
  requires the released heading to equal the workspace version exactly, while the new Python
  check extracted a version out of a suffixed heading and was happy. Local `gates` does not
  run that verb, so the two checkers could disagree without anything local failing. The
  heading is now `## 0.1.0` with the pre-release status stated in its body, and
  `changelog_version` returns the heading itself rather than a parsed substring, so
  `repo-check` reaches the same verdict as the CI job. A fourth red-before-green test pins
  that contract. This changes no scope and no acceptance criterion.
- **2026-11-10 — AC#3 is ticked before the tag; the rest of the task is not.** The
  2026-11-09 note above says the release notes and the annotated tag message "are written
  with the tag in AC#2". That was a plan, not a rule: the notes are a pre-tag deliverable and
  are now written and checked. AC#2 must annotate the tag with their first line verbatim, so
  the two cannot disagree. The exact command at cut time is
  `git tag -a v0.1.0 -m "$(head -1 docs/releases/0.1.0.md)"`, run from a commit whose
  `pixi run gates` is green; `repo-check` reads the tag's `%(contents:subject)` and fails if
  it differs. AC#4 and AC#5 stay open for the reasons already recorded: both need a real tag
  run, prefix.dev's owner-side OIDC binding, and a named clean-container image. No tag was
  created, pushed, or approved here.
- **2026-11-10 — correction, recorded as a correction.** `CHANGELOG.md`'s `0.1.0` bullet
  said "The installed binary reports its version, verifies a sealed payload, and restores
  one." The installed `universe-airlock` Conda package implements `version` and `verify`
  only; `restore` is a verb of `universe-airlock-bootstrap`, which is a separate binary. The
  bullet now names which binary owns which verb, and the release notes say the same. Writing
  notes that "name what `universe-airlock` actually does" is what surfaced the overclaim.
  The `## 0.1.0` heading is unchanged, so `universe lint`'s parity check is unaffected.
- **A check that reads an independent source.** The six keys the notes must disclaim are
  transcribed from `decision-119`'s numbered `## Decision` list;
  `test_the_v1_surface_keys_are_transcribed_from_decision_119` parses that list and fails if
  the decision gains, loses, or reorders a surface. The notes file is selected from
  `[workspace.package].version`, not from a hardcoded `0.1.0`, so a version bump cannot
  leave the previous release's notes standing in for the new one. The check enforces that
  all six surfaces are *enumerated* in a dedicated section; whether the prose beside each
  one is truthful stays a review question, which is why the section is a table a reviewer
  can read rather than a hidden field.
- **2026-11-10 — a real release rehearsal, and the defect it found.** An annotated
  `v0.1.0-rc1` tag was pushed from `4d387a74` to exercise the rendered workflows without
  creating the release tag. `universe` (#42859692699) finished green, but `universe publish`
  (#42859692446) failed: `publish / build` succeeded, then both `publish / branch` and
  `publish / prefix-dev` stopped at the local check with
  `channel.fragment.none: no fragments under ./output/fragments; run universe audit
  --emit-fragment first`. No channel branch was pushed and nothing was uploaded, so the
  prefix.dev trusted-publisher binding remains unproven. The cause is a renderer gap of the
  same family as PR #74: `build_job` built and uploaded `output/`, while the only
  `audit --emit-fragment` step lives in `universe.yml`'s separate audit job, whose fragments
  never enter the publish run's artifact. A red-first renderer test now pins that the publish
  build writes `output/fragments` **before** `actions/upload-artifact`, and the renderer emits
  that step for the publish file only; `universe.yml` keeps its own audit job. Generated
  workflows were re-rendered with `ci-render`. AC#2, AC#4, AC#5 and all DoD boxes stay open:
  a second rehearsal from a commit containing the fix was needed before any `v0.1.0` tag;
  the note below records it. `v0.1.0-rc1` stays on the remote as the evidence of the failure.
- **2026-11-10 — second rehearsal: `v0.1.0-rc2` proves the fragment fix and isolates the
  remaining owner-side prerequisite.** An annotated `v0.1.0-rc2` tag was pushed from
  `5414f7c4`, the merge of PR #86, whose `main` push had `gates` and every `universe` job
  green. `universe` run 42875315717 finished green. `universe publish` run 42875315697 had
  `publish / build` green (now including the "Audit the built artifacts" step) and
  `publish / branch` green; `publish / prefix-dev` failed, so `publish / verdict` is red.
  The branch job created `refs/heads/channel` at `267c589f` —
  `channel universe: 1 package(s) linux-64 universe-airlock 0.1.0` — containing
  `linux-64/universe-airlock-0.1.0-h41f06ac_0.conda`, `linux-64/repodata.json`,
  `noarch/repodata.json`, `channeldata.json`, `.nojekyll`, `README.md`, and the fragment under
  `meta/linux-64/`. A local `git archive` of that branch confirms the archive's sha256
  (`f85ac96a833a2297543e9e7b60eb5bf8095a62f522ae8e62e15ce11e95f6fbe2`) and size (298506) match
  the repodata record exactly. This is the first real publish to the static channel, and the
  built-in `GITHUB_TOKEN` was enough to push it, as `decision-127` claims.
- **What still blocks AC#4, recorded rather than glossed.** `publish / prefix-dev` failed with
  `publish.store.failed: store 'prefix-dev': https://prefix.dev/api/oidc/mint_token: the token
  exchange answered 401: Failed to find OIDC publisher: GitHub publisher not found`. The
  `archont561/archont561` channel exists and already holds other packages, but its
  `linux-64/repodata.json` names no `universe-airlock` record, so nothing was uploaded. The
  owner must add a GitHub trusted publisher on that channel for owner `Archont561`, repository
  `universe`, workflow file `universe-publish.yml`, with no environment (the job declares
  none); `id-token: write` is already rendered. This is configuration, not a code fix.
  Separately, `pixi install --offline` from a local checkout of the channel, asking only for
  `universe-airlock=0.1.0`, failed with `libgcc >=16, for which no candidates were found`. The
  recipe declares no runtime requirements, but rattler-build detects `__glibc >=2.17` and
  `libgcc >=16`, and the branch channel carries only `universe-airlock`. A network-denied
  consumer therefore needs those runtime packages in its cache or in the channel itself.
  AC#4 stays open on both halves; AC#2, AC#5 and every DoD box stay open. No `v0.1.0` tag
  was created or pushed; both RC tags remain on the remote as evidence.
<!-- SECTION:NOTES.END -->
