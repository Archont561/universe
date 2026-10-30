---
id: UNI-57
title: Render the workflow set from the manifest with universe-workflow
status: In Progress
assignee: []
created_date: '2026-10-31'
updated_date: '2026-10-31'
labels:
  - ci
  - release
  - schemas
milestone: m-1
dependencies:
  - UNI-41
documentation:
  - backlog/decisions/decision-124 - Generated-workflows-are-split-by-trigger-family.md
  - backlog/decisions/decision-125 - Two-setup-actions-with-a-one-line-mode-difference.md
  - backlog/decisions/decision-127 - Manifest-facets-packages-stores-and-airlock-scopes.md
  - backlog/docs/doc-108 - Releases-and-trusted-publishing.md
priority: high
type: feature
ordinal: 57000
---

## Description

<!-- SECTION:DESCRIPTION.BEGIN -->
Implement the rendering engine in `universe-workflow`: manifest plus embedded templates in,
workflow files out, with a drift check that fails when the two disagree.

This is the first consumer of the manifest contract, which is why it belongs to `m-1` rather
than to the release lane. A schema nobody renders from is a schema that has never been
falsified: `UNI-41` can describe `[[packages]]` and `[[stores]]`, but only the renderer proves
that the description is sufficient to derive a job. The task deliberately does not wait for a
publisher — it renders what will publish, it does not publish.

It was extracted from `UNI-13` on 2026-10-31, because that task also depends on the Conda
publisher (`UNI-11`, `UNI-12`) and the engine does not. Keeping the two together made a fully
specified, cheap component wait behind a channel that does not exist, and it left `UNI-41`
asserting a rendering behaviour owned by a task three milestones downstream.

The bootstrap problem is solved by the invocation surface, not by a hand-written file: a
workflow cannot generate the workflow it runs under, but Pixi can. `pixi run ci-render` builds
the workspace and renders the committed set, so this repository's own first
`universe-release.yml` is produced by the same code path that produces every later one
(`decision-124` §Consequences). Nothing in `.github/workflows/` is hand-authored once that
file exists.

Two properties make the drift check worth having: rendering is deterministic (same manifest,
same templates, byte-identical output), and the renderer reports the derived
store→package mapping alongside the jobs, so a mapping that is wrong is visible rather than
inferred from a workflow's YAML.
<!-- SECTION:DESCRIPTION.END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 `universe ci render` derives the workflow set from `universe.toml` alone and writes it; `universe ci check` re-renders and fails on any byte difference, naming the drifted file. Neither verb reaches the network. — 2026-10-31: `universe ci render|check` in `universe-kit`, exit 0/1/2 split into matched/drifted/usage; eleven CLI tests in `crates/universe-kit/tests/ci.rs` cover both verbs, `--stdout`, `--root`, `--json`, and a hand-edited file.
- [x] #2 Rendering is deterministic: rendering the same manifest twice produces byte-identical output, asserted by digest, so a no-op re-render never manufactures a diff. — 2026-10-31: `rendering_is_byte_identical_across_runs` compares the whole `RenderOutput`, and the CLI test re-renders the written tree and compares bytes.
- [x] #3 `ci render` prints the derived store→package mapping alongside the jobs it produces, so the mapping is inspectable rather than inferred, and `--stdout` shows a render without writing it. — 2026-10-31: `--json` carries `destinations` (store name → package names, sorted so it is stable); `--print-required-checks` prints the pull-request check list in settings order; `--stdout` writes nothing, asserted by the absence of `.github/workflows` afterwards.
- [x] #4 Templates live in `crates/universe-assets` and are embedded at compile time, so rendering needs no template checkout and no network; minijinja uses the `<<`/`>>` delimiters with `auto_escape = none`, and a fixture asserts a rendered file still contains an untouched `${{ }}` expression. — 2026-10-31: `templates/workflow.yml.j2` is `include_str!`d, `auto_escape` is `None`, and `the_generated_yaml_keeps_github_expressions_untouched` asserts `${{ github.ref }}` and `${{ toJSON(needs) }}` survive verbatim. Correction to this criterion: only the *variable* delimiters may move to `<<`/`>>` — minijinja refuses identical block and variable delimiters — so block tags stay `{% %}` and the requirement's intent (a workflow expression survives rendering) holds.
- [ ] #8 The renderer's behaviour is expressed as scenarios before more of it is written: `.feature` files under `crates/universe-workflow/tests/features/`, step definitions next to them, executed by `rstest-bdd` under `pixi run test` with no network. `rstest-bdd` is a dev-dependency, so scenarios failing must not be satisfiable by the shipped binary.
- [ ] #5 Every workflow variant the renderer can emit (prefix.dev-only, github-pages-only, multi-target, watch enabled/disabled) passes `actionlint`, and `ci check` accepts its own output against the same manifest.
- [ ] #6 `pixi run ci-render` and `pixi run ci-check` name the component binary rather than the facade, so rendering does not depend on `universe` sibling resolution landing first; `cargo run` builds before running, so a task cannot silently use a stale binary.
- [ ] #7 This repository's own first `universe-release.yml` is produced by `pixi run ci-render` and committed, and `ci-check` then holds it inside `pixi run check`, so a manifest change that would alter a job fails the gates instead of drifting in review.
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 Rendered workflow combinations are asserted in `TESTS/`, and the fixtures fail when a template changes without the manifest changing.
- [ ] #2 The `ci-check` task is in `pixi run check` from the commit that adds the first rendered workflow onwards.
<!-- DOD:END -->
