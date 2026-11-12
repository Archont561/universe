---
id: UNI-79
title: Render every checkout without persisted credentials
status: To Do
assignee: []
created_date: '2026-11-13'
updated_date: '2026-11-13'
labels:
  - ci
  - infrastructure
milestone: m-2
dependencies: []
documentation:
  - backlog/tasks/uni-57 - Render-the-workflow-set-from-the-manifest.md
priority: medium
type: bug
ordinal: 79000
---

## Description
<!-- SECTION:DESCRIPTION.BEGIN -->
GitHub issue #123 reports that both rendered workflows check out with
`actions/checkout@<sha> # v7.0.1` and no `with: persist-credentials: false`. Verified on
`40bcf1ff`: `universe.yml`, `release.yml` and `universe-airlock.yml` all carry the bare call,
while this repository's hand-maintained workflows (`ci.yml`, `security.yml`,
`publish-sandbox.yml`) set the flag on every checkout. The same repository therefore holds two
conventions, and the generated half is the weaker one.

The cost is concrete rather than theoretical. `persist-credentials` leaves the workflow token in
`.git/config` for every later step, so any step that runs project code — a build script, a test,
a package install — can push with the job's token. A job that genuinely needs to push sets its
own token on the step that pushes, which is what the mirror and lock-repair lanes do.

A consumer running zizmor sees `artipacked` on both generated files and cannot fix it: the files
are byte-compared by `ci check` and `airlock render --check`, so editing one turns the drift
check red. The workaround recorded in the issue is a repository-wide suppression of a real
security lint over generated files, which is the outcome this task exists to remove.
<!-- SECTION:DESCRIPTION.END -->

## Acceptance Criteria
<!-- AC.BEGIN -->
- [ ] #1 Every `actions/checkout` the template emits carries `with: persist-credentials: false`, in each rendered file and each variant fixture.
- [ ] #2 A job that must push authenticates on the step that pushes rather than inheriting a persisted credential; the mirror and lock-repair steps are asserted to still supply their own token, so the flag does not quietly break them.
- [ ] #3 The template test asserts the clause for every checkout in the rendered output, so a second job added later cannot be the one that omits it.
- [ ] #4 zizmor's `artipacked` reports no finding on the rendered files, and the suppression a consumer needed is no longer required — recorded in the task's notes with the finding count before and after.
<!-- AC.END -->

## Definition of Done
<!-- DOD.BEGIN -->
- [ ] #1 The change goes through the template and `pixi run ci-render`, never by editing a generated file, and `pixi run ci-check` is green.
- [ ] #2 `pixi run gates` passes, including `actionlint` over the re-rendered workflows.
- [ ] #3 `docs/WORKFLOWS.md` states why the flag is rendered rather than left to each job.
<!-- DOD.END -->
