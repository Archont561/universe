---
id: UNI-5
title: Build portable SvelteKit 3 + Svelte 5 + UnoCSS static portal (@universe/portal)
status: To Do
assignee: []
created_date: '2026-10-26 20:15'
updated_date: '2026-10-27 19:00'
labels:
  - portal
  - channel
milestone: m-6
dependencies:
  - UNI-30
documentation:
  - backlog/docs/architecture/doc-3 - Static-channel-and-SvelteKit-UnoCSS-portal-specification.md
  - backlog/decisions/decision-6 - SvelteKit-3-+-Svelte-5-SSG-+-UnoCSS-for-@universe-portal-using-template-route-cloning-and-no-Storybook.md
  - backlog/decisions/decision-11 - Headless-v0.1.0-first,-browser-portal-deferred-to-v0.2.0.md
priority: high
type: feature
ordinal: 5000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
> Deferred from M1 to M6 by decision-11. The headless `v0.1.0` release embeds a committed
> offline placeholder at `crates/universe-assets/placeholder/index.html` via a `build.rs`
> fallback; the real SSG output built here takes precedence automatically once present, and
> the release re-wiring plus placeholder removal are tracked by `UNI-26`.

The initial package layout, Bun manifests/lock, shared config and schema package scaffolds, and package `VERSIONS` files are present. They are not a completed portal implementation. Continue with the functional scope below.

Implement the static documentation + Conda channel explorer app in `packages/portal` (`doc-3`, `decision-6`):
- Configure SvelteKit `^3.1.0` + Svelte `^5.57.0` with `@sveltejs/adapter-static`, `appDir: "assets"`, `paths: { relative: true }`, `prerender: { entries: ["/", "/packages/__pkg__/"] }`, and `trailingSlash = "always"`.
- Configure UnoCSS (`unocss/vite` `^66.11.0`) with `presetWind3()`, `presetIcons()` backed by offline `@iconify-json/lucide` + `@iconify-json/simple-icons`, `presetTypography()`, and self-hosted variable `.woff2` fonts (`Inter` + `JetBrains Mono`).
- Implement pure Svelte 5 `.svelte.ts` view-models (`catalog-filter.svelte.ts`, `install-snippet.svelte.ts`, `platform-matrix.svelte.ts`).
- Build the `/` catalog explorer (search, platform filter chips, channel quickstart bar, CEP-15 solver floor callout) and `/packages/[name]/` detail view (install command tabs, per-subdir artifact + `sha256` matrix, dependencies/`run_exports`, SPDX + `goblin` binary audit verification drawer, and `evidence_html` prose panel) supporting both inline `<script id="cu-*-data" type="application/json">` hydration and relative `fetch('./package.json')` / `fetch('./catalog.json')` navigation.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 `bun --cwd packages/portal run build` emits a deterministic static site in `packages/portal/build/` containing `index.html`, `packages/__pkg__/index.html`, and `assets/` (no `_app/` directory).
- [ ] #2 Zero external font, CSS, or icon CDN URLs exist anywhere in the emitted HTML/CSS/JS bundle.
- [ ] #3 Selecting a GitHub Pages CEP-15 channel endpoint in the install bar displays the required solver floor notice (`pixi >= 0.40`, `conda >= 24.5`, `mamba >= 2.0`).
- [ ] #4 Dynamic `--cu-accent` theming applies `universe.accent_color` cleanly in both light and dark modes.
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 `svelte-check` (`bun --cwd packages/portal run typecheck`) passes with zero warnings against `@universe/schema`.
<!-- DOD:END -->
