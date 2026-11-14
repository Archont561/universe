---
id: UNI-77
title: Refuse to render a publish job that cannot publish
status: Done
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
download-artifact — with no publish step, and that the file's verdict job then passes.

**Correction to the issue's mechanism, from rendering it.** On `40bcf1ff` no such job is
emitted at all: `publish_jobs` tests `publish_verb` and `continue`s before `jobs.push`, so the
assembled job is dropped. The issue's own reproducer never reaches the renderer either — an
`oidc` npmjs store and a Conda package publishing to it are both refused by validation first.
What does reproduce, with a `cargo` package and a writable `crates-io` store:

```text
release.yml:  build → verdict        (no publish job, no publish step)
```

The file exists, the build runs, the verdict job passes, and nothing is published. The defect
the issue names is real and its severity is right; only its description of the shape is wrong,
and that changes the fix. There is no zero-step job to delete — the problem is that a
*manifest* declaring an unimplemented destination still produces a green `release.yml`.

The fix belongs at the earliest gate, not at render time: a manifest that declares a writable
store nothing can publish to is a manifest error, and manifest validation already exists to say
so before a workflow is written. Rendering then never sees the case, and a consumer learns the
problem while editing `universe.toml` rather than while reading a green check.

Adding a `StoreKind` must force this decision rather than defaulting into it, which is why the
match is exhaustive and why the acceptance criteria pin the exhaustiveness rather than a list of
five names.
<!-- SECTION:DESCRIPTION.END -->

## Acceptance Criteria
<!-- AC.BEGIN -->
- [x] #1 Manifest validation rejects a writable store whose kind has no publish implementation, naming the store, its kind and the verb that is missing — an error, not a warning, because the rendered job would be required and green. — `manifest.store.unpublishable` in `validate_store_publishes`; `a_writable_store_nothing_can_publish_to_is_refused` is red first.
- [x] #2 The diagnostic says what to do: implement the verb, or stop declaring the store writable, or remove it. — the message names all three, and `a_read_only_store_of_an_unimplemented_kind_is_only_a_warning` pins the half that stays legal: reading from a registry works long before writing to it does.
- [x] #3 Whether a kind publishes is stated once, in `StoreKind::publishes` in `universe-core`, and the renderer's verb table derives from it, so the validator and the renderer cannot disagree about which kinds publish. — `publish_verb` now returns `None` only where `publishes()` is false and otherwise splits branch-and-tree from artifact upload.
- [x] #4 A new store kind added without a publish verb is caught by a test rather than by review: one fixture iterates `StoreKind::ALL` and asserts the renderer's verb table agrees with `publishes` for every kind. — `every_store_kind_either_publishes_or_is_refused_before_rendering`; `StoreKind::ALL` exists so a property of every variant can be asserted, and a new variant does not compile into the list without an edit.
- [x] #5 A `release.yml` whose verdict would pass over a lane that published nothing is not reachable through the CLI: the manifest is refused before a file is rendered, and the rendered file is asserted to contain no job for the store. — `a_writable_store_with_no_verb_renders_no_publish_job` renders a manifest the validator would refuse, and asserts no job carries the store's name, so the renderer is safe even when called without validation.
<!-- AC.END -->

## Definition of Done
<!-- DOD.BEGIN -->
- [x] #1 The reproducer from the issue is a fixture: a manifest with one package and an `npmjs` writable store, asserting `ci render` exits non-zero and names the store. — the issue's exact manifest never reached the renderer (its `oidc` auth and Conda-to-npmjs pairing are refused first), so the fixture is the minimal one that *did* reproduce at `40bcf1ff`: a `cargo` package with a writable `crates-io` store, which rendered a green `build → verdict` release file with nothing published. The correction is recorded in the Description.
- [x] #2 `pixi run gates` passes, and this repository's own manifest still renders the same `release.yml` bytes, since it declares no such store. — the rendered bytes did change, but from `UNI-79`'s checkout flag rather than from this task; this manifest declares no unimplemented kind.
- [x] #3 `docs/WORKFLOWS.md` and `docs/MANIFEST.md` state which store kinds publish today and what a manifest declaring another one is told. — the `docs/WORKFLOWS.md` bullet names the five kinds and the reason the refusal is an error.
<!-- DOD.END -->
