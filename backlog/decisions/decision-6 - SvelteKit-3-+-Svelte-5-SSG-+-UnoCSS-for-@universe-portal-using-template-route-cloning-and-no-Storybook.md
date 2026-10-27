---
id: decision-6
title: SvelteKit 3 + Svelte 5 SSG + UnoCSS for @universe/portal using template-route cloning and no Storybook
date: '2026-10-26 20:15'
status: accepted
---

## Context

`universe-kit` bundles a pre-built static documentation + channel explorer frontend at compile time (`crates/universe-assets`) and emits a self-contained static site when a consumer runs `universe-kit pages build`. This imposes strict constraints:
1. Consumer sites deploy at arbitrary URL depths (`https://org.github.io/my-universe/`, custom root domains `https://conda.example.com/`, or local `python -m http.server`), and the URL subpath is unknown when `universe-kit` itself is compiled.
2. Consumer package lists are unknown at `universe-kit` compile time, yet every package (`packages/<name>/index.html`) needs a direct deep-linkable static HTML page with SEO/OpenGraph tags and zero Node.js runtime on consumer machines.
3. Styling must be 100% pure JS (no native `.node` addons that complicate sandbox/lockfile portability) and self-contained offline (no external Google Fonts or Iconify runtime CDN fetches).
4. Component testing must run cleanly under `bun test` + `@archont561/bun-test-utils` without introducing a second test runner.

## Decision

1. **SvelteKit 3 + Svelte 5 (`@sveltejs/adapter-static`) portable SSG configuration**:
   - Set `kit.appDir = "assets"` (avoiding `_app/`, which GitHub Pages Jekyll drops if `.nojekyll` is ever omitted, while still emitting `.nojekyll`).
   - Set `kit.paths = { relative: true }`, `prerender = true`, and `trailingSlash = "always"` so asset links resolve relatively (`./assets/...` at root, `../../assets/...` inside `packages/<name>/index.html`) across any base path.
2. **Template-route cloning + `lol_html` hydration**:
   - At `universe-kit` compile time, SvelteKit prerenders two shell routes: `/` (`build/index.html`) and `/packages/__pkg__/` (`build/packages/__pkg__/index.html`).
   - At consumer `universe-kit pages build` time, Rust extracts the embedded `build/` directory, removes `packages/__pkg__/`, writes `catalog.json` and `packages/<name>/package.json`, and clones `packages/__pkg__/index.html` into `packages/<name>/index.html` for every package—using `lol_html` to rewrite `<title>`, `<meta property="og:*">`, and `<script id="cu-pkg-data" type="application/json">` for instant zero-waterfall rendering while preserving client-side SvelteKit navigation via `fetch('./package.json')` / `fetch('./catalog.json')`.
3. **UnoCSS (`unocss/vite` `^66.11.0`) instead of Tailwind CSS**:
   - Use `presetWind3()`, `presetIcons()` backed by offline `@iconify-json/lucide` and `@iconify-json/simple-icons` (inlined as zero-network CSS data URIs), `presetTypography()` (for rendered `pulldown-cmark` smoke-test `evidence_html`), `transformerDirectives()`, and `transformerVariantGroup()`.
   - Self-host variable `.woff2` fonts (`Inter` + `JetBrains Mono`) in `packages/portal/static/fonts/`.
4. **Reject Storybook**:
   - Do not install Storybook (`10.6+`), whose Svelte 5 testing story relies on `@storybook/addon-vitest` (introducing Vitest alongside `bun test` and adding ~150+ MB of dependency weight).
   - Keep reactive UI logic in pure Svelte 5 `.svelte.ts` view-model modules (`catalog-filter.svelte.ts`, `install-snippet.svelte.ts`, `platform-matrix.svelte.ts`) unit/property-tested directly in `bun test`, paired with Playwright `webPage` E2E/visual verification (and an optional dev-only `/_harness` route excluded from production SSG builds).

## Consequences

- One ~150 KB pre-built portal bundle embedded inside `universe-kit` works identically at `/`, `/my-universe/`, `/a/b/c/`, and `file://`-served previews with zero external network calls.
- Consumer repositories never install Node.js, Bun, SvelteKit, or UnoCSS to generate rich HTML package documentation.
- Frontend tests stay 100% unified on `bun test` + `@archont561/bun-test-utils`.
