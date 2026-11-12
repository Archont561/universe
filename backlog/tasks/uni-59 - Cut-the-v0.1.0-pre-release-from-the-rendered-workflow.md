---
id: UNI-59
title: Cut the v0.1.0 pre-release from the rendered workflow
status: In Progress
assignee: []
created_date: '2026-11-01'
updated_date: '2026-11-11'
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
every declared destination verified after the fact, and a restore transcript that came from
a clean host rather than from the runner that produced the package.

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
- [x] #2 The `v0.1.0` tag is cut from a commit whose full gate run is green, and the tag is annotated with the same message the release notes open with. — 2026-11-11: annotated tag `v0.1.0` (object `d49d8fe587f1805095b70a3935dd4e73bf0d3019`) points at `1de18891bd69f8d16b4d261cfdaa53525555eabd`, whose `ci` and `universe` runs were green. `pixi run gates` passed at that commit before the tag was pushed, and the tag-triggered `universe` run `43073492717` is green. `git for-each-ref --format='%(contents:subject)' refs/tags/v0.1.0` equals `head -1 docs/releases/0.1.0.md` verbatim; `pixi run repo-check` and `pixi run --frozen universe -- lint --json` both pass with the tag present. The tag-triggered `release.yml` run `43073492712` has a green `publish / verdict`. https://github.com/Archont561/universe/actions/runs/43073492712
- [x] #3 Release notes state the pre-release status and the `linux-64`-only scope, name what `universe-airlock` actually does at this tag (verify, restore, version), and claim none of `decision-119`'s six v1 surfaces. — 2026-11-10: `docs/releases/0.1.0.md` opens with the one-line annotated-tag message, states the `decision-132` checkpoint, names all three verbs with the binary that owns each, and has a `## Not a v1 release` table disclaiming all six numbered surfaces of `decision-119` as absent or single-platform. `scripts/repo_check.py::validate_release_notes` enforces the file's existence from the workspace version, the annotation's shape, the five required facts, the six surface keys, and — once a `v0.1.0` tag exists — that its `%(contents:subject)` equals the opening line. Six Python tests were red before green, and a temporary annotated tag proved the drift check both ways; that tag was deleted. The real tag was cut afterwards, which is criterion 2's evidence above.
- [ ] #4 All three packages are verified at all three declared destinations after the run: the branch channel solves for each of `universe-airlock=0.1.0`, `universe-kit=0.1.0` and `universe=0.1.0` from a local checkout with network denied, a fresh prefix.dev read resolves each package by digest, and each GitHub Release asset resolves by digest. — 2026-11-11: the digest halves are proven by the tag run `43073492712` (see the notes below); the offline solve from a local checkout is **not** proven. A fresh `pixi install --offline` against a clone of `refs/heads/channel` fails with `libgcc >=16, for which no candidates were found`: the recipes pulled the C standard library's run export and the branch channel does not carry libgcc. The recipes now suppress it (`ignore_run_exports.by_name: [libgcc]`) and `universe lint` refuses a recipe that takes `stdlib('c')` without that suppression, but the **published** `v0.1.0` artifacts still carry `libgcc >=16`; the criterion closes when a rebuilt channel is published and solved offline. This is a measured failure, not a claim of success, and the criterion stays open.
- [ ] #5 A clean container with only Git and Pixi runs the rendered restore entrypoint against the tag and prints `universe-airlock version`; the transcript is recorded in this task's notes, with the container image named. — 2026-11-11: no container runtime is available in this workspace (`docker` and `podman` are absent, `unshare -n` is refused by the sandbox). Also, no restore entrypoint is declared by this repository's `universe.toml`: `pixi run airlock-check` reports `no restore entrypoint is declared`, and the tag tree contains no `.universe-airlock/restore.sh`. `UNI-53` DoD #1 stays open on the same transcript. Do not claim a clean-container, sealed, or Pixi-managed environment.
- [x] #6 A repository check fails if the changelog's top section and the tag disagree, so the next release cannot drift by memory. — 2026-11-09: a temporary local `v0.2.0` tag made `pixi run repo-check` exit 1 naming both values; the tag was removed and the check returned to green.
<!-- AC.END -->

## Definition of Done
<!-- DOD.BEGIN -->
- [x] #1 The rendered workflow's verdict job is green for the tag run, and its link is in the notes. — 2026-11-11: `publish / verdict` in run `43073492712` is green; https://github.com/Archont561/universe/actions/runs/43073492712
- [x] #2 `pixi run gates` passes on the tagged commit. — 2026-11-11: `pixi run gates` exits 0 at `1de18891bd69f8d16b4d261cfdaa53525555eabd` with 617 Rust tests and the full Python, alint, Vale and actionlint checks green. The `ci` run `43066271239` and `universe` run `43066271086` on that commit are also green, and the tag-triggered `universe` run `43073492717` is green.
- [ ] #3 The task records all three destination URLs and the restore transcript. — 2026-11-11: the three destination URLs are recorded below, but the clean-container restore transcript is not (see the criterion 5 note). DoD #3 stays open.
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
  none); `id-token: write` is already rendered. **This sentence is superseded below:** the
  owner later declared a `prefix` environment and the job now declares it too. The rest of
  the paragraph still stands. This is configuration, not a code fix.
  Separately, `pixi install --offline` from a local checkout of the channel, asking only for
  `universe-airlock=0.1.0`, failed with `libgcc >=16, for which no candidates were found`. The
  recipe declares no runtime requirements, but rattler-build detects `__glibc >=2.17` and
  `libgcc >=16`, and the branch channel carries only `universe-airlock`. A network-denied
  consumer therefore needs those runtime packages in its cache or in the channel itself.
  AC#4 stays open on both halves; AC#2, AC#5 and every DoD box stay open. No `v0.1.0` tag
  was created or pushed; both RC tags remain on the remote as evidence.
- **2026-11-10 — re-watch after the owner updated prefix.dev publishing: the same 401 three
  times.** The owner reported updating publishing on prefix.dev. Three further publish runs
  still failed `publish / prefix-dev` with the identical `publish.store.failed: ... 401: Failed
  to find OIDC publisher: GitHub publisher not found`: a rerun of failed jobs on the
  `v0.1.0-rc2` run (attempt 2, 18:52 UTC), a full rerun of that run (attempt 3, 18:55 UTC),
  and a `workflow_dispatch` of `universe-publish.yml` from `main` at `b4b9b326` (run
  42880702045, 18:59 UTC). Every run had `publish / build` and `publish / branch` green and a
  red verdict. A `main` dispatch failing the same way rules out a tag-ref or event-name
  restriction: the mismatch is in the publisher entry's owner, repository, workflow filename,
  or environment fields, or in which channel the entry was added to. The job presents
  `Archont561/universe`, workflow file `universe-publish.yml` (full path
  `.github/workflows/universe-publish.yml`), audience `prefix.dev`, and **no environment** —
  the rendered job declares none, so an entry that names one cannot match. The manifest's
  channel is `archont561/archont561`; its `linux-64/repodata.json` still names no
  `universe-airlock` package. AC#2, AC#4, AC#5 and all DoD boxes stay open.
- **A reproducibility defect the reruns exposed, recorded as a correction.** Each rebuild of
  `universe-airlock-0.1.0-h41f06ac_0.conda` produced different bytes with the same name,
  version and build string: the first rc2 publish recorded sha256
  `f85ac96a833a2297543e9e7b60eb5bf8095a62f522ae8e62e15ce11e95f6fbe2` / 298506 bytes at channel
  commit `267c589f`, the full rerun `e4b527541467930fe013091cbbf33298132732bcf008602dbe4b098e6d2c9caf`
  / 298502 at `8f2ca660`, and the `main` dispatch
  `ca0aa2f788bc2d8456e51e4b7ab878b475a033cf172f09f76d162ea8094a52c0` / 298513 at `00056f30`.
  `publish / branch` rewrites the channel branch as an orphan commit each time. The digest
  verification in the note above was true of that commit at that time, but a release that pins
  a digest must pin the digest of the exact run that publishes it, not one measured from an
  earlier rehearsal.
- **The `prefix` environment became a store facet, and the re-watch still failed.** PR #89
  merged `2d814a3b`: `Store.environment` is validated, `universe.toml` names `prefix` on
  `prefix-dev`, and `ci-render` emits `environment: prefix` on `publish / prefix-dev` and on
  no other job. A `workflow_dispatch` of `universe-publish.yml` from `main` at that commit
  (run 42888243912) had `publish / build` and `publish / branch` green; `publish / prefix-dev`
  failed again with `publish.store.failed: store 'prefix-dev':
  https://prefix.dev/api/oidc/mint_token: the token exchange answered 401: Failed to find
  OIDC publisher: GitHub publisher not found`, so `publish / verdict` is red. The environment
  claim was necessary but not sufficient. No `v0.1.0` tag was created or pushed.
- **Correction, recorded as a correction.** The note above says the owner must add a publisher
  "with no environment (the job declares none)". That is no longer true: the owner declared a
  `prefix` environment, the rendered job declares it, and the exchange still finds no
  publisher. The stale requirement is superseded by the measurement below; it is not a
  description of the current configuration.
- **The remaining mismatch is the workflow filename, measured rather than guessed.** A
  temporary diagnostic workflow on the working branch (runs 42889858141, 42890244038 and
  42890480692) decoded this repository's OIDC identity claims — never the token — and read
  the channel's registered publishers from prefix.dev's public GraphQL endpoint. The claims
  carry `aud=prefix.dev`, `environment=prefix`, `repository_owner=Archont561` and
  `repository=Archont561/universe`; `workflow_ref` and `job_workflow_ref` both name the file
  that requested the token, so this is not a reusable-workflow mismatch. The channel
  `archont561/archont561` has 12 GitHub publishers. The only one whose repository is
  `universe` is `Archont561 / universe / release.yml / environment prefix`. The rendered
  publish file is `universe-publish.yml`, so no publisher entry matches the token. This is
  owner-side configuration, not a renderer defect: the trusted publisher for this repository
  must name `universe-publish.yml` (with environment `prefix`), or the rendered publish file
  must be renamed to `release.yml`. Until one of those changes, another rerun cannot
  succeed. The diagnostic workflow was removed and the branch force-pushed back to
  `2d814a3b`; no diagnostic workflow remains in the repository. AC#2, AC#4, AC#5 and every
  DoD box stay open, and `linux-64/repodata.json` on the prefix.dev channel still names no
  `universe-airlock` package.
- **The owner chose the rename, and the OIDC publisher now matches.** PR #91 renamed the
  rendered tag-triggered file to `release.yml`; `ci render` removes the former
  `universe-publish.yml`, so one tag starts one publish workflow. The token's `workflow_ref`
  basename now equals the filename the channel's publisher entry names. This supersedes the
  note above as a reversal, not an edit to history: at that earlier commit the rendered file
  was `universe-publish.yml`. `decision-124` and `doc-2` record the reversal.
- **Re-watch after the rename: the 401 is gone, and a later defect is exposed.** A
  `workflow_dispatch` of `release.yml` from `main` at `59641d8b` (run 42898855277) had
  `publish / build` and `publish / branch` green. `publish / prefix-dev` no longer answered
  `Failed to find OIDC publisher`: the token was minted and the job reached the upload, then
  failed with `publish.store.failed: store 'prefix-dev':
  https://prefix.dev/api/v1/upload/archont561/archont561: HEAD answered 405`. The verdict is
  red. This proves the `prefix` environment and the `release.yml` filename are the binding
  the registry expects; it does not yet prove an upload. The cause is a client bug:
  `HttpRemote::head` treated 405 as a transport failure, while this endpoint accepts POST
  only. A red-first wiremock test now makes 405 mean "the unchanged probe is unavailable,
  send the artifact", and the registry decides whether it already holds the bytes. Another
  re-watch after that fix is required before AC#4 can be considered. AC#2, AC#4, AC#5 and
  every DoD box stay open; no `v0.1.0` tag was cut, and `linux-64/repodata.json` on the
  prefix.dev channel still names no `universe-airlock` package.
- **The next re-watch reached the upload body and found the missing headers.** A
  `workflow_dispatch` of `release.yml` from `main` at `d2c099c4` (run 42900930515), after the
  405 fix, again had `publish / build` and `publish / branch` green. `publish / prefix-dev`
  minted the token, tolerated the 405 probe, and sent the POST; it failed with
  `publish.store.failed: store 'prefix-dev':
  https://prefix.dev/api/v1/upload/archont561/archont561: POST answered 400: Header of type
  'content-type' was missing`. The verdict is red. prefix.dev's REST upload endpoint requires
  `Content-Type: application/octet-stream`, `X-File-Name` and `X-File-SHA256` besides
  `Authorization`. A red-first wiremock test now pins those headers, and `--force` is
  translated into `?force=true` on this endpoint so an occupied name can be overwritten when
  the caller asks. Another re-watch after that change is required before AC#4 can be
  considered. AC#2, AC#4, AC#5 and every DoD box stay open; no `v0.1.0` tag was cut, and
  `linux-64/repodata.json` on the prefix.dev channel still names no `universe-airlock`
  package.
- **A green publish verdict, and the package was on both destinations then declared.** A
  `workflow_dispatch` of `release.yml` from `main` at `e9ec1b34` (run 42903356074) finished
  green: `publish / build`, `publish / prefix-dev`, `publish / branch` and
  `publish / verdict` all succeeded, with no failure annotations. The OIDC token was minted,
  the upload POST was accepted, and reindexing followed. `publish / branch` rewrote
  `refs/heads/channel` as orphan commit `499c70f0`, whose `linux-64/repodata.json` records
  `universe-airlock-0.1.0-h41f06ac_0.conda` with sha256
  `b148b2152d4c096c24264f5e1c8f40419053c4d51a5f81f97c289efa9b95e0f8` and size 298583.
  prefix.dev's package API reports the same name, version `0.1.0`, platform `linux-64`, and
  one variant with that filename, digest and size. The public `linux-64/repodata.json` URL is
  served through a CDN that still returned the older index even when read with a no-cache
  header, so the package API — not the cached repodata file — is the evidence here. A
  temporary diagnostic workflow (runs 42904758077, 42904964211 and 42905156000) read that
  API; it was removed and the branch force-pushed back to `e9ec1b34`, so no diagnostic
  workflow remains in the repository.
- **What this closes and what it does not.** Every owner-side prerequisite named above is
  now met: the `prefix` environment, the `release.yml` workflow filename, the 405 `HEAD`
  probe, and the upload headers. AC#4's "prefix.dev by digest" half is proven. AC#4's
  "offline branch solve" half is still open: `pixi install --offline` from the branch channel
  asking only for `universe-airlock=0.1.0` still needs `libgcc >=16` and `__glibc >=2.17`,
  which the branch channel does not carry. AC#2 (an annotated `v0.1.0` tag from green
  gates), AC#5 (a clean Git+Pixi container transcript with the image name) and every DoD
  box stay open. No `v0.1.0` tag was cut or pushed.
- **Correction, recorded as a correction: the scope is now three destinations.** The owner
  observed that no GitHub Release existed, although `decision-127` calls `releases` implicit
  and `universe.toml` sets `release = true`. The earlier notes above describe a run that
  published to two destinations; they are true of that run, not of the current manifest.
  `decision-132` now has an amendment that widens `v0.1.0` to `prefix-dev`, the Git-branch
  channel, and GitHub Releases. `universe.toml` declares a `releases`-kind store, and the
  renderer gives it a `publish / releases` job for every `release = true` package without
  the package naming it. AC#4 and DoD #3 are reworded to cover all three and stay open:
  the green run above did not create a GitHub Release asset, and a re-watch after the
  renderer change is required. No `v0.1.0` tag was cut; AC#2, AC#5 and every DoD box stay
  open.
- **The GitHub Release asset exists, and the re-watch is red for a different reason.** After
  PR #95 merged `5f102cd2`, a `workflow_dispatch` of `release.yml` from `main` (run
  42914663367) had `publish / build`, `publish / releases` and `publish / branch` green.
  `publish / prefix-dev` failed with `publish.store.failed: store 'prefix-dev':
  https://prefix.dev/api/v1/upload/archont561/archont561: POST answered 409: A package with
  the filename 'universe-airlock-0.1.0-h41f06ac_0.conda' already exists in the channel with
  a different SHA256 hash`, so `publish / verdict` is red. The 409 is the recorded
  non-reproducible-artifact defect: a rebuild of the same name, version and build string
  carries different bytes, and the hosted channel refuses a changed digest unless the caller
  passes `--force`. The rendered publish commands do not pass it.
- **What the release job actually wrote.** GitHub now has a published rolling release
  `packages-linux-64 artifacts` under tag `packages-linux-64`, created 2026-11-10T22:26:50Z,
  with asset `universe-airlock-0.1.0-h41f06ac_0.conda`, sha256
  `dfc08b401414baebe182ad9a5971673d915d82bdd8ac4c9bb0b81adcb355e2b6`, size 298585. The
  `channel` branch at `aa853bdb` carries the same digest and size in
  `linux-64/repodata.json`. prefix.dev still holds the earlier digest
  `b148b2152d4c096c24264f5e1c8f40419053c4d51a5f81f97c289efa9b95e0f8` from run 42903356074,
  because the 409 refused this run's bytes. This proves AC#4's GitHub Release-asset half for
  one run; it does not prove that the three destinations agree on one digest, and a later
  re-watch would be refused by the releases store too unless `--force` or reproducibility
  is addressed. AC#2, AC#4's offline-solve half, AC#5 and every DoD box stay open. No
  `v0.1.0` tag was cut.
- **The owner chose rendered `--force` for the two artifact stores.** `ci render` now emits
  `publish-artifacts --store prefix-dev --force ...` and
  `publish-artifacts --store releases --force ...`; `publish --store branch` is unchanged.
  prefix.dev receives `?force=true` and the rolling release deletes and re-uploads the
  occupied asset. This is a **reversal, recorded as a reversal**: `decision-126`'s status note
  and the second amendment to `decision-132` scope it to the `v0.1.0` publish lane, and the
  CLI still refuses a changed digest without `--force`. It changes the hosted failure from 409
  to a permission question without fixing the non-reproducible build and without authorising
  every destination to be rewritten. The re-watch below is the evidence that the releases and
  branch destinations accepted the changed digest while prefix.dev did not. AC#2, AC#4, AC#5
  and every DoD box stay open. No `v0.1.0` tag was cut.
- **Attempt 1 of the forced re-watch is red because prefix.dev needs a broader owner-side
  permission.** After PR #97 merged `f86617cf`, `workflow_dispatch` run `42920755513` from
  `main` had green `publish / build`, `publish / releases` and `publish / branch` jobs.
  `publish / prefix-dev` failed with `publish.store.failed: store 'prefix-dev':
  https://prefix.dev/api/v1/upload/archont561/archont561?force=true: POST answered 403: Not
  authorized`, so attempt 1's `publish / verdict` was red. The URL proves `--force` reached
  the endpoint. The GitHub Release asset `universe-airlock-0.1.0-h41f06ac_0.conda` was
  replaced at `2026-11-10T23:09:58Z` with sha256
  `17337dbb208182060e4ccd002473ae0bf64482a1b0fe3813585124569a982ec8`, size 298582; the
  `channel` branch at `1a93b8df91bbbc736ec7a154b08a5c4434411d7d` carries the same digest and
  size in `linux-64/repodata.json`. prefix.dev did not accept attempt 1's bytes, so its earlier
  `b148b2152d4c096c24264f5e1c8f40419053c4d51a5f81f97c289efa9b95e0f8` variant remained.
  `https://prefix.dev/docs/prefix/api` states that `force=true` needs a scoped key with
  read/write/delete, and gives the same read-write-delete mode to a Repository Access entry
  for force-replacing an existing filename. The `Archont561 / universe / release.yml /
  environment prefix` publisher is consistent with a read/write entry that can upload a new
  filename but cannot force-replace one.
- **Attempt 2 is green after the owner removed the occupied prefix.dev variant manually.**
  The owner deleted the old `universe-airlock` variant from prefix.dev and reran the failed
  jobs of run `42920755513`; its `run_attempt` is 2 and its conclusion is `success` at
  `2026-11-10T23:23:11Z`. The rerun's `publish / prefix-dev` job downloaded artifact
  `13811507086` with expected digest
  `sha256:4f050f05a7de76ac3433fcfd7e7fa1c64d95ca784306d913026efd16548a07ff` and reported
  `"ok":true`, `"force":true`, `"bytes":298582`, and
  `"uploaded":["linux-64/universe-airlock-0.1.0-h41f06ac_0.conda"]`; `publish / verdict`
  saw `build`, `prefix-dev`, `branch`, and `releases` all `success`.
  The build job's audit fragment pins the package at sha256
  `17337dbb208182060e4ccd002473ae0bf64482a1b0fe3813585124569a982ec8`, size 298582, and the
  same package bytes are read back from the GitHub Release asset and the `channel` branch at
  `1a93b8df91bbbc736ec7a154b08a5c4434411d7d`. All three publish jobs consumed the same
  artifact (`sha256:4f050f05a7de76ac3433fcfd7e7fa1c64d95ca784306d913026efd16548a07ff`), so
  the three destinations received one digest. prefix.dev's public repodata still serves the
  older index; a package API read-back is not recorded here. This proves a clean prefix.dev
  channel accepts the forced upload. It does **not** prove that the current Repository Access
  entry can force-replace an occupied filename, and it does not fix the non-reproducible
  build. `decision-126`, `decision-132`, `UNI-11`, and the prose docs record the correction.
  AC#2, AC#4 (including the offline-solve half and a fresh prefix.dev read-back), AC#5 and all
  DoD boxes remain open. No `v0.1.0` tag was cut.
- **The owner widened the checkpoint to the main CLIs, recorded as a reversal.** After
  PR #99, the owner asked why only Airlock was published. `decision-132`'s main-CLI amendment
  adds `universe-kit` and the umbrella `universe` facade to the still-uncut `v0.1.0` scope;
  `universe.toml` now declares three `linux-64` Conda packages, and `ci render` emits build
  and publish commands for all three at `prefix-dev`, the `branch` store, and `releases`.
  The facade declares `depends_on = ["universe-kit", "universe-airlock"]`, and `package-cli`
  skips rattler-build's post-build test because its run dependencies are published in the same
  release. `UNI-15`, `UNI-16`, and `UNI-40` remain To Do: one `linux-64` pre-release does not
  complete four-platform packaging or the integrated v1 gate. AC#4 is reworded to cover all
  three packages and stays open until a tag run proves the offline solve and a fresh prefix.dev
  read-back for each. No `v0.1.0` tag was cut.
- **2026-11-11 — the three-package re-watch is green, and it exposed a branch-store defect.**
  After the owner removed the occupied prefix.dev variants again, `workflow_dispatch` run
  `43045274207` from `main` at `be4e7df3620cd71ce36344dd3706b9621d917d8a` started at
  `2026-11-11T15:08:04Z` and finished `success` at `15:12:31Z` on attempt 1. All five jobs are
  green: `publish / build`, `publish / prefix-dev`, `publish / branch`, `publish / releases`,
  `publish / verdict`. Every publish job consumed the same artifact
  (`sha256:c208a373343bd1feb0d37e45d53dff924b4ffbb4ca1b97da5b486667c71b3b4d`, 3987065 bytes).
  `publish / build` audited three `linux-64` packages: `universe-airlock` at
  `2e98b81d2155f5033bef5001dae2c6a3dbf5037d97e32e182f4af7e050bfab26` (298583 bytes),
  `universe-kit` at
  `763999409faee8b9972e7935d8e4f8d49599c4004e7eea8f80b5541304392c0b` (3489421 bytes), and the
  umbrella `universe` facade at
  `19e00dffc113f7d6d26e139b8d7cca3c72a995c20846f709b8aa4dca790f7217` (195020 bytes). The facade
  declares run dependencies on the other two packages.
- **What each destination actually holds after that run.** `publish / prefix-dev` reported
  three forced uploads with those byte counts; a fresh, cache-busted read of
  `https://prefix.dev/archont561/archont561/linux-64/repodata.json` (publication `87457`) reads
  back all three filenames with exactly those three sha256 digests and sizes. The plain URL
  still serves the older CDN-cached index from publication `82248`, so the cache-busted read is
  the evidence here. The three GitHub Release assets of the rolling `packages-linux-64`
  release read back the same three digests and sizes through the release API. The forced uploads
  succeeded because the occupied variants were removed again; this is **not** a grant of
  read-write-delete Repository Access, does not prove force-replacing an occupied name, and does
  not fix the non-reproducible build.
- **The branch destination is green but wrong.** `publish / branch` ran three `publish` steps,
  one per package, at commits `e76ab96425e3496e46ec1a665c4aba74146f2cd8`,
  `3162b194cdbf1fd1ccd412ffe36d4b58a0ea5c0b` and
  `1d30ef21701497dd196eeaa1599d7ec3ee24edc6`. Each `publish` force-pushes one orphan snapshot
  tree, so the last push replaced the other two: `refs/heads/channel` now holds only
  `universe-0.1.0-h41f06ac_0.conda` with the facade's digest, and its repodata names
  dependencies (`universe-kit`, `universe-airlock`) that the tree does not carry. An offline
  solve from that branch cannot succeed. A green verdict is not proof that all three packages
  reached all three destinations.
- **Correction, recorded as a correction: one command per snapshot store.** The main-CLI
  amendment widened the branch store to three packages without changing how `publish` pushes:
  `ci render` now emits a single `publish --store branch --remote ... --package universe-airlock
  --package universe-kit --package universe --json` step for a branch or static store, so one
  tree carries every package. Artifact stores keep one `publish-artifacts` step per package;
  their uploads are independent and `--force` still applies only to them. A Rust unit test, a
  BDD scenario and a Python scope test were red before the renderer changed, and the workflows
  were re-rendered with `ci-render`. `decision-132`, `decision-126`, `UNI-11`, `CHANGELOG.md`
  and the prose docs record the correction and the evidence.
- **What stays open after the first re-watch.** AC#2 (an annotated `v0.1.0` tag from green
  gates), AC#4 (the offline branch solve with network denied, for all three packages at all
  three destinations, after a re-watch with the corrected renderer), AC#5 (a clean Git+Pixi
  container transcript with the image name) and every DoD box remain open. The fresh prefix.dev
  read-back by digest for all three packages is recorded above; the branch half of AC#4 is not.
  No `v0.1.0` tag was cut.
- **2026-11-11 — the corrected re-watch is green, and force-replacement is proven.** The owner
  granted read-write-delete on the prefix.dev Repository Access entry and asked for a re-watch
  with no manual removal. `workflow_dispatch` run `43061441347` from `main` at
  `fa785a323d146974b742fe8a390748f56472b832` started at `2026-11-11T16:45:25Z` and finished
  `success` at `16:50:52Z` on attempt 1: `publish / build`, `publish / prefix-dev`,
  `publish / branch`, `publish / releases` and `publish / verdict` are all green. All four
  consuming jobs downloaded one artifact (`13872723957`,
  `sha256:322f9e051dd0668509b8f268d684c2767c0b4d7af530decdb619ce26c0a6e4f7`, 3989110 bytes).
  The build audited `universe-airlock` at
  `134f5e7dfd2d249f08195a2678e7ff6200f54b53fe04bc95c3229ff1ca8e651d` (298595 bytes),
  `universe-kit` at
  `63b958b773573e158ff30bdd682a0669311d58058a790c2783ae1c90902e9c12` (3491258 bytes), and the
  umbrella `universe` facade at
  `470f52e41e4dfaae20ce22d893cf1e098b2784c123ae623cdacab07c84667e69` (195021 bytes); the facade
  declares run dependencies on the other two.
- **All three destinations now agree on those three digests.** `publish / prefix-dev` reported
  three forced uploads (`298595`, `3491258`, `195021` bytes) with no `403` — the occupied
  filenames from run `43045274207` were replaced under changed digests with no owner-side
  removal, so this run proves force-replacement with the read-write-delete entry. A fresh,
  cache-busted read of the channel's `linux-64/repodata.json` (publication `87896`) reads back
  all three filenames with exactly those digests and sizes. The plain URL and publication
  `87577` still served the older CDN-cached index for several minutes after reindexing, so the
  cache-busted read is the evidence. The three GitHub Release assets of `packages-linux-64
  artifacts` read back the same digests and sizes through the release API.
- **The branch store is fixed.** `publish / branch` now runs one step, `Publish the channel
  snapshot`, and pushed orphan commit `7afdff10e32c28d368caaf07acc18a118d81ea74` with twelve
  files: all three `.conda` archives, all three audit receipts, and the generated metadata.
  Its `linux-64/repodata.json` lists all three packages with those three digests and the
  facade's two dependency edges now have matching records in the same tree. A local
  verification of the same one-command publish against a temporary remote produced one tree
  with all three packages before this run.
- **What this closes and what it does not.** AC#4's "all three packages at all three
  destinations by digest" is now verified for a run, including a fresh prefix.dev read-back
  and a branch tree that carries every package. AC#4 stays open on its offline half: a
  `pixi install --offline` from a local checkout of that branch with network denied has not
  been recorded, and the recipes declare `libgcc >=16` and `__glibc >=2.17` that the branch
  channel does not carry. AC#2 (the annotated `v0.1.0` tag from green gates), AC#5 (a clean
  Git+Pixi container transcript with the image named) and every DoD box remain open. The build
  is still not reproducible: the same name, version and build string carried different bytes
  in runs `43045274207` and `43061441347`. No `v0.1.0` tag was cut by that work. `decision-126`,
  `decision-132`, `UNI-11`, `CHANGELOG.md` and the prose docs record the correction.

- **2026-11-11 — the annotated `v0.1.0` tag is cut, and its tag run is green.** After the
  owner granted read-write-delete on the prefix.dev Repository Access entry, the tag `v0.1.0`
  was cut from `1de18891bd69f8d16b4d261cfdaa53525555eabd` and pushed. The annotated tag object
  is `d49d8fe587f1805095b70a3935dd4e73bf0d3019` and its subject equals `head -1
  docs/releases/0.1.0.md` verbatim. The push triggered two tag-triggered workflows: `universe`
  run `43073492717` (build, validate, view, audit, verdict all green) and `release` run
  `43073492712` (build, prefix-dev, branch, releases, verdict all green).
  https://github.com/Archont561/universe/actions/runs/43073492712 is the link DoD #1 names.
  `pixi run gates`, `pixi run repo-check` and `universe lint --json` pass with the tag present.
- **The three tag-run digests, and all three destinations reading them back.** The tag run's
  build audited `universe-airlock` at
  `7153aedcc8188a044524f6bc84639188e7d62491caf74d9630a4fcbd5859a3ab` (298600 bytes),
  `universe-kit` at
  `314d6941c8a1776f04954a117ea6d91cca3d97971ca6203e68d1c5a11d6233b2` (3491139 bytes), and the
  umbrella `universe` facade at
  `c4e1fc2792135a803fb3ec9a772d2fd9e2a529d323ccb792d30f7ef1636f9cec` (195023 bytes). Its common
  artifact is `sha256:b2f1706e9e9274f0c3fbb8a8be0db9a18a85f368d89aa6bda54d07111ee3a446`. The
  three GitHub Release assets of `packages-linux-64 artifacts` read back those three digests
  through the release API; the `channel` branch at
  `eb532021ccc386c875c720e6b67138b4b30c1152` is one snapshot with twelve files (three archives,
  three receipts, generated metadata) whose `linux-64/repodata.json` lists all three with those
  digests; and a fresh cache-busted prefix.dev repodata read (publication `88268`) resolves all
  three by digest. The plain repodata URL was briefly CDN-stale at the older publication.
  Because the build is still not reproducible, these tag-run bytes supersede the digests
  recorded above for runs `43045274207` and `43061441347`; those earlier entries describe their
  own runs, not the tag's published bytes. The three destination URLs are
  `https://prefix.dev/archont561/archont561`,
  `https://github.com/Archont561/universe/tree/channel`, and
  `https://github.com/Archont561/universe/releases/tag/packages-linux-64`. The
  clean-container restore transcript is **not** recorded: no container runtime is available
  here, and this owner-lane manifest declares no `[airlock.developer.restore]`, so
  `pixi run airlock-check` reports `no restore entrypoint is declared` and the tag tree carries
  no `.universe-airlock/restore.sh`. The offline branch solve from a local checkout with network
  denied is **not** proven: `pixi install --offline` against a clone of `channel` fails with
  `libgcc >=16, for which no candidates were found`. The recipes take run exports from
  `stdlib('c')` that the branch channel does not carry; adding conda-forge to the consumer
  project fails offline because there is no cached conda-forge repodata, and
  `[system-requirements]` accepts `libc` but rejects `libgcc`. A clean-container restore
  (`UNI-53` DoD #1), an offline solve (criterion 4), `UNI-55` sealed payloads, and the
  four-platform work (`UNI-15`, `UNI-16`, `UNI-40`) all remain open. No `v1` surface is
  claimed. `decision-126`, `decision-132`, `UNI-11`, `CHANGELOG.md` and the prose docs record
  the tag as a correction, not a reversal of scope. No further tag is cut.
- **The offline-solve failure has a cause in the recipe, and it is now refused there.** The
  three published `v0.1.0` packages carry `libgcc >=16` in their repodata entries. Nothing in
  the binaries needs it: they are Rust executables whose only undeclared run input is the C
  library that the virtual `__glibc` package already stands for. The dependency appears because
  each recipe lists `${{ stdlib('c') }}` among its build requirements, and that package declares
  a run export, so rattler-build writes it into `depends`. A static channel does not carry
  libgcc, so a consumer that clones `refs/heads/channel` and runs `pixi install --offline`
  fails at solve time with `libgcc >=16, for which no candidates were found`.
  Two measurements separate the cause from the symptom. First, reproduced here: the consumer
  project against an unmodified clone fails with exactly that error, and `CONDA_OVERRIDE_LIBGCC`
  and the deprecated `[system-requirements]` route both leave it unchanged, because neither
  makes a package the channel does not hold. Second, a copy of the same tree with only the
  `libgcc` entry removed from each `packages.conda` record solves and installs offline in one
  step, so the undeclared dependency is the whole difference.
  The fix is in `crates/{universe-airlock,universe-kit,universe-cli}/recipe/recipe.yaml`:
  `requirements.ignore_run_exports.by_name` names `libgcc`, and
  `crates/universe-conda/src/recipe.rs` gains `recipe.run_exports.leaked`, which refuses a
  recipe that takes `stdlib('c')` in `build` or `host` without that suppression. A Rust unit
  test and a BDD scenario were red before the rule existed — both failed asserting
  `recipe.run_exports.leaked` — and a second unit test pins the suppressed recipe as clean.
  The rule lives in the recipe linter rather than in alint or Vale because it is a fact about a
  recipe's contents, which is the question that linter already answers (`AGENTS.md` rule 13).
  **What this does not close:** the published `v0.1.0` artifacts were built before the fix and
  still carry `libgcc >=16`, so the offline solve against the real branch channel still fails.
  Closing criterion 4 needs a rebuilt, republished channel — a new build of the same version,
  which under the recorded `--force` reversal replaces the artifacts in place and changes their
  digests again. That is a release action, not a lint fix, and it is not taken here.
<!-- SECTION:NOTES.END -->
