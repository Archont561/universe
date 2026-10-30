---
id: UNI-57
title: Render the workflow set from the manifest with universe-workflow
status: Done
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
- [x] #8 The renderer's behaviour is expressed as scenarios before more of it is written: `.feature` files under `crates/universe-workflow/tests/features/`, step definitions next to them, executed by `rstest-bdd` under `pixi run test` with no network. `rstest-bdd` is a dev-dependency, so scenarios failing must not be satisfiable by the shipped binary — 2026-10-31: five scenarios in `render.feature` run through `rstest-bdd` in `universe-workflow::render_bdd`, `pixi run test` reports 162 passing with 162/162 including `rendering_is_repeatable`, and the run is offline (`cargo` resolves against the restored vendored transport). `rstest*` are dev-dependencies only, so the shipped binary cannot satisfy a step.
- [x] #5 Every workflow variant the renderer can emit (prefix.dev-only, static/github-pages-only, multi-target, watch enabled/disabled) passes `actionlint`, and `ci check` accepts its own output against the same manifest. — 2026-10-31: `TESTS/test_rendered_workflows.py` renders five variants — hosted channel only, git-branch channel only, both, nothing published, and lock adapters present — asserts the exact file set each produces, runs `actionlint` over it, and then requires `ci check` to accept the same output. Two corrections to the criterion: the static-channel store kind is spelled `static` (the `branch` kind is a directory grafted onto a git branch), and the *watch* file is unreachable rather than a variant — `derived_files` never emits it, so it is asserted as absent by construction instead of claimed as linted. Rendering this repository's own manifest is what exposed three renderer defects: `runs-on:` was empty for every job because `Job` carried no runner, a job's own steps were emitted before `actions/checkout` and `setup-pixi`, and no file ended with a newline. The first two are now asserted by a scenario that reads every rendered file back from disk, and the third by a unit test.
- [x] #6 `pixi run ci-render` and `pixi run ci-check` name the component binary rather than the facade, so rendering does not depend on `universe` sibling resolution landing first; `cargo run` builds before running, so a task cannot silently use a stale binary. — 2026-10-31: both tasks are `cargo run --quiet --locked -p universe-kit -- ci render|check` in `pixi.toml`; the facade resolves sibling binaries and would have made a render depend on that succeeding.
- [x] #7 This repository's own first `universe-release.yml` is produced by `pixi run ci-render` and committed, and `ci-check` then holds it inside `pixi run check`, so a manifest change that would alter a job fails the gates instead of drifting in review. — 2026-10-31: `pixi run ci-render` wrote `.github/workflows/universe.yml` and `.github/workflows/universe-publish.yml`, both committed. Correction to the criterion: the set is split by trigger family per `decision-124`, so there is no `universe-release.yml` — the pull-request/push file is `universe.yml` and the tag-triggered file is `universe-publish.yml`. `ci-check` is the last entry of `pixi run check` and the task passes. Known gap, owned by `UNI-7`/`UNI-8`/`UNI-9`: the rendered jobs call `universe plan|lint|build|verify`, which do not exist yet, so these files are correct but not yet runnable.
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [x] #1 Rendered workflow combinations are asserted in `TESTS/`, and the fixtures fail when a template changes without the manifest changing. — 2026-10-31: `TESTS/test_rendered_workflows.py` covers the variants; a template change without a matching manifest change is caught by `ci-check` inside `pixi run check`, and both defect classes were confirmed by mutating the renderer and watching the suites go red.
- [x] #2 The `ci-check` task is in `pixi run check` from the commit that adds the first rendered workflow onwards. — 2026-10-31: added in the same change as the rendered files.
<!-- DOD:END -->
