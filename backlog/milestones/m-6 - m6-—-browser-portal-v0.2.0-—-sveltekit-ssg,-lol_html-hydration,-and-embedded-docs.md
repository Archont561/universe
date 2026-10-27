---
id: m-6
title: "M6 — Browser portal v0.2.0: SvelteKit SSG, lol_html hydration, embedded docs"
---

## Description

The `packages/config`, `packages/schema`, and `packages/portal` workspace layout plus the placeholder-aware `crates/universe-assets` crate have been scaffolded, with each package version tracked in `VERSIONS`. These are foundations only; the behavioral exit conditions below remain outstanding.

Deliver the human-facing half of the universe-kit positioning as the `v0.2.0` minor
release, after the headless `v0.1.0` has shipped (decision-11). Build the portable
SvelteKit 3 + Svelte 5 + UnoCSS static portal (`@universe/portal`, `UNI-5`), complete
its unit/property/snapshot/subpath E2E suites so the real generated SSG output takes over
from the committed in-crate placeholder `crates/universe-assets/placeholder/index.html` via
the `build.rs` preference order (`UNI-6`), add the
`lol_html` hydration and `catalog.json` / `packages/<name>/package.json` generation split out
of the headless `UNI-12` indexer (`UNI-25`), and restore the real `portal-bundle` release
stage before cutting `v0.2.0` (`UNI-26`).

The channel-indexing, publishing, and release-governance machinery from M2–M4 is reused
unchanged; this milestone only adds the browser layer on top of the same static channel tree
specified in doc-3.

Exit condition: `universe-kit pages build` emits the full doc-3 tree (solver artifacts plus
hydrated `index.html`, `packages/__pkg__/`, and per-package pages) with byte-identical,
offline-only output, the portal passes every `@archont561/bun-test-utils` suite including the
non-root `/subpath/` Playwright E2E under `networkGuard`, and `v0.2.0` is published to
`prefix.dev/archont561/archont561` with the embedded portal and a live GitHub Pages catalog
explorer.
