---
id: UNI-79
title: Render every checkout without persisted credentials
status: In Progress
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
- [x] #1 Every `actions/checkout` the template emits carries `with: persist-credentials: false`, in each rendered file and each variant fixture. — `checkout_and_pixi` emits the flag as an input, so every job inherits it from one place; `no_rendered_checkout_leaves_the_token_in_the_work_tree` counts checkouts against occurrences across every rendered file, and was red first at 5 checkouts to 0 flags.
- [x] #2 A job that must push authenticates on the step that pushes rather than inheriting a persisted credential; the mirror and lock-repair steps are asserted to still supply their own token, so the flag does not quietly break them. — this was a real break, not a hypothetical: `airlock_jobs` rendered the mirror and entrypoint-render jobs with `contents: write` and no token in the environment, and the mirror pushes through `GitTransport` with a credential built from the store's declared secret. Both jobs now export it (`a_job_that_pushes_carries_its_own_token`). The lock-repair lane is a hand-written workflow (`relock.yml`) and already sets `persist-credentials: true` deliberately, so it is untouched.
- [x] #3 The template test asserts the clause for every checkout in the rendered output, so a second job added later cannot be the one that omits it. — the assertion is a count against the rendered output rather than a fixed list of jobs, so a job added later is covered without editing the test.
- [ ] #4 **Open.** zizmor is not installed in this environment (`GITHUB_TOKEN`-free, no network), so the finding count is not reproducible here. What is proven is the input it keys on: no rendered `actions/checkout` lacks `persist-credentials: false`. The count belongs in a follow-up comment once someone runs zizmor against the re-rendered files.
<!-- AC.END -->

## Definition of Done
<!-- DOD.BEGIN -->
- [x] #1 The change goes through the template and `pixi run ci-render`, never by editing a generated file, and `pixi run ci-check` is green. — the edit is in `checkout_and_pixi` in `universe-workflow`; `ci-render` rewrote both files and `ci-check` accepts them.
- [x] #2 `pixi run gates` passes, including `actionlint` over the re-rendered workflows.
- [x] #3 `docs/WORKFLOWS.md` states why the flag is rendered rather than left to each job. — "No rendered checkout leaves the token in the work tree", including the jobs that must push and how they authenticate instead.
<!-- DOD.END -->
