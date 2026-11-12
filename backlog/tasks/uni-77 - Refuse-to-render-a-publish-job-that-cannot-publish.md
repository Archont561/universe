---
id: UNI-77
title: Refuse to render a publish job that cannot publish
status: To Do
assignee: []
created_date: '2026-11-13'
updated_date: '2026-11-13'
labels:
  - architecture
  - ci
  - release
milestone: m-2
dependencies: []
documentation:
  - backlog/tasks/uni-57 - Render-the-workflow-set-from-the-manifest.md
  - backlog/decisions/decision-128 - Rendered-workflows-expose-one-verdict-job-per-file.md
priority: high
type: bug
ordinal: 77000
---

## Description
<!-- SECTION:DESCRIPTION.BEGIN -->
GitHub issue #121 reports that a writable store of kind `npmjs`, `github-packages`, `pypi`,
`crates-io` or `command` renders a complete publish job — checkout, pixi, rust cache,
download-artifact — with no publish step, and that the file's verdict job then passes. Verified
on `40bcf1ff`: `publish_verb` returns `None` for those kinds, and `publish_jobs` takes the
`continue` *after* the job has been assembled, so the job is pushed with zero steps that do
anything.

The in-source comment defends the `None`: rendering a command that would fail at run time is
worse than rendering none. That reasoning stops one step short. A job that fails is
investigated; a job that succeeds while publishing nothing is a required check turning green
over an empty release, and the failure it hides is the whole point of the lane. The ordering in
`publish_jobs` is what turns the defensible choice into the indefensible one.

The fix belongs at the earliest gate, not at render time: a manifest that declares a writable
store nothing can publish to is a manifest error, and `universe lint` already exists to say so
before a workflow is written. Rendering then never sees the case, and a consumer learns the
problem while editing `universe.toml` rather than while reading a green check.

Adding a `StoreKind` must force this decision rather than defaulting into it, which is why the
match is exhaustive and why the acceptance criteria pin the exhaustiveness rather than a list of
five names.
<!-- SECTION:DESCRIPTION.END -->

## Acceptance Criteria
<!-- AC.BEGIN -->
- [ ] #1 Manifest validation rejects a writable store whose kind has no publish implementation, naming the store, its kind and the verb that is missing — an error, not a warning, because the rendered job would be required and green.
- [ ] #2 The diagnostic says what to do: implement the verb, or stop declaring the store writable, or remove it.
- [ ] #3 `ci render` cannot emit a publish job with zero publish steps for any `StoreKind`; the match that decides the verb is exhaustive over the enum, so a new kind fails to compile until it is given a verb or named as unsupported.
- [ ] #4 A new store kind added without a publish verb is caught by a test rather than by review: a fixture iterating every kind asserts that each either renders at least one publish step or is refused by validation.
- [ ] #5 A file whose verdict job would pass over a job that published nothing is not reachable, and the verdict logic is asserted to treat no-op jobs as absent rather than successful if such a job is ever constructed.
<!-- AC.END -->

## Definition of Done
<!-- DOD.BEGIN -->
- [ ] #1 The reproducer from the issue is a fixture: a manifest with one package and an `npmjs` writable store, asserting `ci render` exits non-zero and names the store.
- [ ] #2 `pixi run gates` passes, and this repository's own manifest still renders the same `release.yml` bytes, since it declares no such store.
- [ ] #3 `docs/WORKFLOWS.md` and `docs/MANIFEST.md` state which store kinds publish today and what a manifest declaring another one is told.
<!-- DOD.END -->
