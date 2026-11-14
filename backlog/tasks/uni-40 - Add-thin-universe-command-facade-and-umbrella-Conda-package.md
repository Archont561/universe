---
id: UNI-40
title: Add the thin `universe` command facade and umbrella Conda package
status: To Do
assignee: []
created_date: '2026-10-27'
updated_date: '2026-10-31'
labels:
  - architecture
  - packaging
milestone: m-8
dependencies:
  - UNI-15
  - UNI-34
  - UNI-38
priority: high
type: feature
ordinal: 40000
---

## Status note

**Status note (2026-11-13).** Criterion 5 — component resolution — is now proven and ticked,
which was the half of this task a consumer could feel. The task stays `To Do`: criteria 1, 2, 3,
4 and 7 still want a clean-channel install, the umbrella's runtime dependency range, and fresh
environment dispatcher tests, none of which one `linux-64` workspace dispatch proves. The
`--help` half of GitHub issue #125 is `UNI-81`.

**Scope widened, recorded as a reversal.** The main-CLI amendment to `decision-132` adds the
umbrella `universe` facade to the still-uncut `v0.1.0` publish lane. Its recipe declares
`universe-kit` and `universe-airlock` as run dependencies; `package-cli` skips rattler-build's
post-build test because those dependencies are published in the same release, while the
workspace gates prove the facade binary. This task remains To Do: a clean-channel install and
offline dispatcher tests are not proven by one `linux-64` dispatch. The `v0.1.0` tag was cut
on 2026-11-11 (see the correction below).


## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Complete the thin `universe-cli` workspace crate and its Conda recipe, whose package name is
`universe`. Its binary routes ordinary producer commands to `universe-kit` and
`universe airlock …` to `universe-airlock`, forwards arguments/exit status, and owns no
product logic. The `universe` Conda package depends on the separately installable
`universe-kit` and `universe-airlock` packages; the bootstrap remains a separate artifact of the
consumer edition and a phase rather than a third product (`decision-122`). One tag binds every
artifact: the components are separately installable, not independently versioned. Preserve
component recipes and direct component binaries for users who do not want the umbrella.

The facade scaffold is present in the workspace; this task completes compatibility, package
metadata, release versioning, clean-channel installation, and conformance behavior.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 `universe <kit-args>` forwards to `universe-kit`; `universe kit <args>` removes the explicit namespace; `universe airlock <args>` forwards to `universe-airlock`.
- [ ] #2 Arguments (including non-UTF8 values where supported), child exit codes, and missing-component diagnostics are tested; facade does not duplicate Kit/Airlock business logic.
- [ ] #3 Conda package `universe` installs the facade and pulls the component packages as runtime dependencies with a bounded pre-1.0 range of the same minor (`>=0.1,<0.2`); all components remain independently installable but share one version.
- [ ] #4 Fresh environment tests verify the umbrella command dispatches after install and that bootstrap is not accidentally bundled as producer/consumer implementation.
- [x] #5 Component resolution order is explicit and tested: an override directory, then the directory of the running executable, then `PATH`, then a diagnostic that names the missing component and a remediation command. Resolving from `PATH` alone is the defect this criterion exists to prevent, because it lets a stray component shadow the installed one. — 2026-11-13: **GitHub issue #125 was this criterion's observed symptom.** Filed against a pixi-installed `universe` 0.1.0 invoked by absolute path without its bin directory on `PATH`, it failed with `universe: cannot run universe-kit: No such file or directory (os error 2)` because `main` called `Command::new(component)` with the bare name. `Search` now carries the three places as data (`UNIVERSE_COMPONENT_DIR`, then `current_exe().parent()`, then `PATH`) so the order is testable without a build, and `resolve` returns a `MissingComponent` naming every directory searched plus the three remedies. Five unit tests pin the order and the message, one covers a directory and a non-executable file not being mistaken for a component, and `tests/resolution.rs` runs the **real** binary by absolute path with `PATH=/nonexistent` for the issue's exact case — red first, printing that very message. The order tests were also mutation-checked: inverting the search to `PATH`-first fails exactly the two that assert priority. The issue's related `--help` gap is tracked separately as `UNI-81`, because it is the same first-run surface but not this criterion.
- [x] #6 `Product` in `universe-core` reduces to the two editions, and the bootstrap is reachable only as an Airlock phase (`decision-122` §Consequences). A conformance test asserts no third product value can be constructed. — 2026-10-31: enum is `Kit | Airlock`; the bootstrapper passes its own binary name through `render_version_as`.
- [ ] #7 Recipe and workspace version metadata are synchronized at release, and the package installs from the project's static channel.
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 Offline dispatcher tests pass; clean-channel package install/smoke tests pass in the connected release acceptance lane.
- [ ] #2 No registry, catalog, or Airlock protocol implementation is added to `universe-cli`.
**Status note — correction (2026-11-11).** The annotated `v0.1.0` tag was cut from `1de18891bd69f8d16b4d261cfdaa53525555eabd` and pushed; its tag-triggered `release.yml` run `43073492712` is green, including `publish / verdict`. https://github.com/Archont561/universe/actions/runs/43073492712 This records the tag as evidence, not as a completion of this record's criteria: the tasks above remain To Do. The offline branch solve with network denied, a clean-container restore transcript, and the four-platform and integrated v1 lanes are still open. `UNI-59` records the tag run's digests and the remaining gaps. No further tag is cut.
<!-- DOD:END -->
