---
id: UNI-25
title: Hydrate SvelteKit portal HTML via lol_html in universe-kit pages build
status: To Do
assignee: []
created_date: '2026-10-27 13:40'
updated_date: '2026-10-27 13:40'
labels:
  - portal
  - channel
  - testing
milestone: m-6
dependencies:
  - UNI-6
  - UNI-12
documentation:
  - backlog/docs/architecture/doc-3 - Static-channel-and-SvelteKit-UnoCSS-portal-specification.md
  - backlog/decisions/decision-5 - Static-Conda-channel-on-GitHub-Pages-+-GitHub-Releases-via-CEP-15-base_url,-CEP-16-shards,-and-immutable-JSON-fragments.md
  - backlog/decisions/decision-6 - SvelteKit-3-+-Svelte-5-SSG-+-UnoCSS-for-@universe-portal-using-template-route-cloning-and-no-Storybook.md
  - backlog/decisions/decision-119 - Full-multi-ecosystem-v1-and-static-catalog-distribution.md
priority: high
type: feature
ordinal: 25000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Implement the browsable portal HTML/catalog generation as a distinct layer beside the standard
static Conda channel (`UNI-12`), consistent with the integrated v1 scope (`doc-3`, `decision-5`,
`decision-6`, `decision-119`):
- Extract the embedded `crates/universe-assets` portal files (the real SSG output delivered by
  `UNI-6`, replacing the committed headless placeholder), remove the `packages/__pkg__/`
  template route from the emitted tree, and clone it once per discovered package.
- Write root `catalog.json` and `packages/<name>/package.json`, rendering
  `docs.evidence_markdown` with `pulldown-cmark` and sanitizing the result with `ammonia`
  (zero raw HTML/script passthrough).
- Hydrate `index.html` and every cloned `packages/<name>/index.html` via `lol_html`:
  `<title>`, OpenGraph meta tags, and safely escaped
  `<script id="cu-*-data" type="application/json">` data islands, supporting both inline-island
  hydration and relative `fetch('./package.json')` / `fetch('./catalog.json')` navigation under
  arbitrary non-root GitHub Pages subpaths.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 `proptest` proves hydrated `catalog.json` and `packages/*/index.html` are byte-identical across arbitrary permutations of input fragment discovery order, complementing the solver-artifact determinism already guaranteed by `UNI-12`.
- [ ] #2 `lol_html` safely escapes `</script>` and other markup in package descriptions and evidence prose without XSS or broken DOM parsing, including adversarial fixture payloads.
- [ ] #3 Every hydrated page renders from both the inline JSON island and the relative `fetch(...)` data path under a non-root `/org-universe/` prefix, verified by the `@archont561/bun-test-utils` Playwright E2E suite under `networkGuard` with zero external requests.
- [ ] #4 `ammonia` strips all raw HTML/script from rendered `evidence_markdown` while preserving allowed documentation formatting, with unit tests for each sanitizer rule.
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 Hydration scenarios in `rstest-bdd` (`pages_build.feature`), `proptest` permutation suites, and the subpath E2E suite pass under `cargo nextest` and `bun test`.
<!-- DOD:END -->
