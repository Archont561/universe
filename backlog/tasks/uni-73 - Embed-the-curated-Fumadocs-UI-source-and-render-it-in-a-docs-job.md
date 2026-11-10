---
id: UNI-73
title: Embed the curated Fumadocs UI source and render it in a docs job
status: To Do
assignee: []
created_date: '2026-11-10'
updated_date: '2026-11-10'
labels:
  - portal
  - docs
  - architecture
  - ci
  - testing
  - offline
milestone: m-7
dependencies:
  - UNI-12
  - UNI-57
  - UNI-68
documentation:
  - backlog/decisions/decision-140 - Build-the-Universe-docs-site-from-pinned-Fumadocs-UI-at-build-time.md
  - backlog/decisions/decision-4 - Static-file-and-codegen-Rust-to-TypeScript-bridge-(ts-rs-+-schemars-+-include_dir!)-with-zero-runtime-FFI-or-WASM.md
  - backlog/docs/architecture/doc-3 - Static-channel-and-SvelteKit-UnoCSS-portal-specification.md
priority: medium
type: feature
ordinal: 73000
---

## Description
<!-- SECTION:DESCRIPTION.BEGIN -->
`decision-140` (proposed) reverses `decision-6`'s direction for the documentation half of the
browsable portal: Universe owns the **source** of a curated Fumadocs UI site, embeds that
source in `crates/universe-assets`, and builds the static site in a rendered docs job against
the JSON registry that `universe-kit pages build` emits. TypeScript loaders read the registry
at build time and generate real per-package pages, a static search index, and the `doc-5` data
views. No Fumadocs bytes are committed, so rule 9 and the alint forbidden-basename rules stay
intact, and MIT attribution travels in the generated site instead of the repository.

This is deliberately a slice of the portal surface, not a rewrite of the release.
`decision-132` still keeps the browsable portal out of `v0.1.0`; `pages build` keeps emitting
the Conda channel and its registry JSON; `universe-core` stays neutral. The work is blocked by
`UNI-12` (the registry JSON exists), `UNI-57` (workflows are rendered), and `UNI-68`
(generated assets carry a freshness check).

Every acceptance criterion below starts red. In particular, the arbitrary-depth URL guarantee
`decision-6` got for free from SvelteKit's relative paths must be re-proven against a Fumadocs
export, not assumed from a framework document.
<!-- SECTION:DESCRIPTION.END -->

## Acceptance Criteria
<!-- AC.BEGIN -->
- [ ] #1 `crates/universe-assets` embeds the Universe-owned Fumadocs UI source tree, and a Rust test is red before the embedding exists: the embedded tree contains the loader and the pinned dependency manifest, contains no third-party license or notice basename, and matches the source directory byte-for-byte so a stale asset fails freshness (`UNI-68`).
- [ ] #2 A typed JSON-registry contract is emitted by `pages build` and consumed by a TypeScript loader, in the static-JSON seam of `decision-4` with no runtime FFI or WASM. A fixture with two packages is red before the loader exists and then produces two prerendered HTML pages whose `<title>` and `og:*` values come from the registry, not from a copied shell.
- [ ] #3 The docs job is rendered from `universe.toml` by `ci render`, runs one `pixi run <task>` whose Python script extracts the embedded source, installs Fumadocs from a pinned integrity-checked lockfile, and builds the export. `ci check` and `lint-actions` stay green; no lockfile is hand-edited and no ad-hoc shell appears in the generated workflow.
- [ ] #4 The emitted site is proven at root, one-deep, three-deep, and `file://` paths with zero external requests and zero 404s under the existing browser network guard. This criterion chooses between a declared `basePath` and a relative-path strategy by measurement; a strategy that fails any depth does not pass.
- [ ] #5 The generated site carries pinned-version/SPDX attribution for Fumadocs, while `alint check` still finds no forbidden license or notice file in the repository. `pages build` itself needs no Bun/Node or network, and `crate-check` confirms `universe-core` stays neutral.
- [ ] #6 `pixi run gates` passes with the Rust, Python, and browser suites, the rendered-workflow drift check, and 82 alint rules green.
<!-- AC.END -->

## Definition of Done
<!-- DOD.BEGIN -->
- [ ] #1 `decision-140` is accepted and `decision-6` is recorded as superseded by a reversal, with `doc-3`, `doc-5`, `UNI-5`, `UNI-6`, `UNI-25`, and `UNI-26` re-scoped or marked historical rather than left silently stale.
- [ ] #2 Both roadmap documents list this task, `backlog-check` and `repo-check` are green, and the docs that a reader follows say where the docs build runs and what a consumer needs to regenerate it locally.
- [ ] #3 The pull request is merged by rebase with local gates green first; the `v0.1.0` pre-release scope is unchanged.
<!-- DOD.END -->

## Notes
<!-- NOTES.BEGIN -->
**2026-11-10 — direction recorded, no code changed.** The owner proposed a Fumadocs UI
curated site whose source is bundled into `universe-assets` and rebuilt in a docs job with a
pinned Fumadocs download, so a TypeScript loader can run against a Universe-generated JSON
registry. This is recorded as `decision-140` with `status: proposed`; three owner choices
remain open (reuse `packages/portal` versus `packages/universe-ui`, declared `basePath`
versus a relative-path strategy, and a per-universe docs job versus an owner-lane-only one).
Neither `v0.1.0` nor the still-open `v0.1.0-rc1` rehearsal failure is affected: the publish
build still emits no `output/fragments`, and the prefix.dev trusted-publisher binding is still
unproven.
<!-- NOTES.END -->
