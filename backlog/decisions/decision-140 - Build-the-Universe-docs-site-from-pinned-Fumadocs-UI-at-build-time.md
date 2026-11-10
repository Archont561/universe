---
id: decision-140
title: Build the Universe docs site from pinned Fumadocs UI at build time, with Universe-owned source embedded as assets
date: '2026-11-10'
status: proposed
---

## Status note

This is the owner's proposed shape for the documentation half of the browsable portal
(surface 6 of `decision-119`), recorded as a **reversal of `decision-6` direction**, not as an
accepted change to the release. `decision-6`, `doc-3`, `doc-5`, `UNI-5`, `UNI-6`, `UNI-25`, and
`UNI-26` remain in force until this record is accepted. Nothing here widens `decision-132`:
`v0.1.0` stays a `linux-64`-only Airlock packaging checkpoint with the browsable portal out of
scope.

## Context

The owner wants Universe to generate documentation for user universes with a better UI than
the hand-built portal, and explicitly does not want Universe to maintain its own documentation
framework. Fumadocs is a React documentation framework (Core + UI + MDX + CLI) that can emit a
static site (`output: 'export'` on Next.js, or prerendered SPA mode on React Router), with a
static Orama search index evaluated in the browser. It is MIT licensed, Copyright 2023 Fuma;
`fumadocs-ui` is MIT too.

`decision-6` instead specified SvelteKit 3 + Svelte 5 + UnoCSS, a ~150 KB prebuilt bundle
embedded through `crates/universe-assets`, template-route cloning plus `lol_html` hydration at
`pages build` time, and 45 hand-built components inventoried in `doc-5`. The current tree does
not implement that: `packages/portal/src` holds only `+page.svelte`, `+layout.svelte`,
`+layout.ts`, `index.ts`, and `schema.ts`; the embedded site is
`crates/universe-assets/placeholder/index.html`. `build.rs` already stages a static tree and
`include_dir!` embeds it, so the assets seam exists but its payload is a placeholder.

Two earlier objections matter here. `AGENTS.md` rule 9 and `.alint.yml`'s
`universe-forbidden-basenames` forbid vendored third-party license and notice files, while MIT
requires its notice to travel with substantial copies: committing a Fumadocs static export into
the repository is therefore not available without changing rule 9. Separately, `decision-6`
chose SvelteKit for `kit.paths.relative = true`, because a consumer site can be served at an
arbitrary URL depth with no Node.js runtime. Fumadocs on Next.js emits root-absolute
`/_next/...` URLs and normally needs `basePath` or `assetPrefix` fixed at build time.

`decision-4` is the enabling record: Rust and TypeScript communicate through **static JSON
payloads and HTML hydration**, with zero runtime FFI or WASM. Its status note says the schema
bridge is an accepted target, not an implemented one. A TypeScript loader that reads a
Universe-generated JSON registry at build time is that seam with hydration moved from consumer
run time to build time.

## Decision (when accepted)

1. **Universe owns only the source of a curated Fumadocs UI site.** The TS/MDX loaders, the
   React/MDX components for catalog and package views, and one pinned dependency manifest live
   in a single `packages/*` unit with the mandatory `src/`, `tests/`, `features/` layout. The
   recommended unit is the existing `packages/portal` path, renamed conceptually to the docs
   UI source, so `bun.lock`, `turbo.json`, `crates/package.json`, and the alint layout rules
   need no second unit. A new `packages/universe-ui` is an equally valid choice but creates
   more stale references; that choice is open.
2. **`crates/universe-assets` embeds that source tree, not built JS.** `build.rs` stages the
   package directory at compile time, exactly as it stages `portal-dist` today, and
   `include_dir!` exposes it. No third-party bytes are committed, rule 9 and the forbidden
   basename rules stay intact, and `universe-airlock` stays small (`AGENTS.md` rule 7). The
   embedded tree is drift-checked against its source directory so a stale asset cannot ship
   (`UNI-68`).
3. **A rendered docs job builds the site.** Generated from `universe.toml` through
   `ci render`, the job runs one `pixi run <task>` whose Python script extracts the embedded
   source, installs Fumadocs from a pinned, integrity-checked lockfile, and builds the static
   export. No ad-hoc shell appears in the generated workflow (`AGENTS.md` rule 10); no lockfile
   is hand-edited, so new dependencies arrive through the relock round trip. The job is a
   **network build** in a CI/release lane, not part of the offline `gates` boundary.
4. **The build's input is the JSON registry emitted by `universe-kit pages build`** —
   `catalog.json`, `packages/<name>/package.json`, and the channel indexes from `doc-3`. Rust
   keeps owning protocol and data; TypeScript owns presentation. This is `decision-4`'s static
   JSON seam, and it keeps `universe-core` neutral (`AGENTS.md` rule 4) and portals as views
   over immutable generations (rule 8).
5. **Loaders generate real pages, not cloned shells.** One MDX route per package with real
   `<title>` and OpenGraph metadata, a static Orama index over the generated content, and the
   `doc-5` data views (install snippets, artifact/platform matrix, dependency list,
   provenance, binary-audit table, verification evidence) as components bound to the JSON
   registry. Template-route cloning and `lol_html` hydration (`UNI-25`) are retired when this
   lands, recorded as a reversal rather than silently dropped.
6. **Attribution travels in the generated output, not the repository.** The docs build emits a
   pinned-version/SPDX attribution page or footer into the emitted site. That satisfies the
   MIT notice requirement for the published bundles without adding a license or notice file to
   Git, which rule 9 forbids.

```text
pages build (Rust)  ->  static channel + JSON registry (catalog.json, package.json, indexes)
        |
        v
docs job (rendered; one pixi run <task>, network lane)
   -> extract embedded Universe-owned source from universe-assets
   -> install pinned Fumadocs from an integrity-checked lock
   -> TS loaders read the JSON registry, generate MDX routes + static Orama index
   -> build the static export and emit it beside the channel
```

## Consequences

- The base-path problem changes shape rather than disappearing. A per-universe build can take
  `basePath` from the manifest, but SvelteKit's arbitrary-depth and `file://` portability was
  free and is now a declared input. Root, one-deep, three-deep, and `file://` serving must be
  proven by tests either with a relative `assetPrefix` or with a deterministic Rust
  relativizer applied to the export. This is the first acceptance criterion of `UNI-73`.
- Consumers who only browse the site or resolve packages still need no Node.js. Consumers who
  regenerate documentation locally now need Bun/Node and network; that is an explicit cost of
  running TypeScript loaders at build time and must be stated in the docs.
- Fumadocs is pinned by exact version and lockfile integrity. Upgrades are a relock round trip,
  not a hand edit, and the embedded source carries a freshness check.
- `decision-2`, `decision-4`, `decision-7`, `decision-8`, `doc-1`, `doc-3`, `doc-5`, and the
  `svelte5-unocss-ssg` skill describe SvelteKit/UnoCSS specifics that become historical. They
  are corrected as corrections, not rewritten to look prescient; `UNI-5`, `UNI-6`, `UNI-25`,
  and `UNI-26` need re-scoping once this record is accepted.
- External references checked when this was written: `https://www.fumadocs.dev/docs`,
  `https://www.fumadocs.dev/docs/next/deploying/static`,
  `https://www.fumadocs.dev/docs/react-router/deploying/static`,
  `https://www.fumadocs.dev/docs/ui`, and the repository LICENSE at
  `https://github.com/fuma-nama/fumadocs/blob/dev/LICENSE`.

## Open choices for the owner

1. Reuse `packages/portal` for the Fumadocs UI source, or add `packages/universe-ui`.
2. Declare `basePath` at build time and give up arbitrary-depth previews, or keep the
   arbitrary-depth invariant with a proven relative-path strategy.
3. Render the docs job for every user universe from `universe.toml`, or build docs only in
   Universe's own release lane.
