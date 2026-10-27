---
name: svelte5-unocss-ssg
description: Build and test portable SvelteKit 3.1 + Svelte 5 static SSG portals styled with UnoCSS for embedding inside Universe Kit (legacy universe-kit). Use when editing packages/portal or packages/config/src/unocss.ts after those packages are ported.
metadata:
  author: Archont561
  version: "0.1.0"
---

# SvelteKit 3.1 + Svelte 5 + UnoCSS Portable SSG Invariants

## Critical Configuration Rules

1. **SvelteKit 3.1 inline Vite configuration (`packages/portal/vite.config.ts`)**: SvelteKit 3.1 removed `svelte.config.js` (`config_file_unsupported`) and deprecated `KitConfig.alias`. Enable Vite 8's native `resolve: { tsconfigPaths: true }` and pass `adapter`, `appDir: "assets"`, and `paths: { relative: true }` directly to `sveltekit({ ... })` inside `vite.config.ts`, keep `prerender = true` and `trailingSlash = "always"` in `src/routes/+layout.ts`, extend `["$app/tsconfig", "@workspace/config/tsconfig/app.json"]` with `"paths": { "@/*": ["./*"] }` and `"types": ["$app/types", "bun"]` in `tsconfig.json`, and import package modules via `@/src/...` (never `../src/...` or `$lib`).
2. **Relative asset paths**: Never emit root-relative `/assets/...` or `/_app/...` URLs; generated portals deploy at arbitrary GitHub Pages subpaths (`/repo-name/`) and custom domains.
3. **Offline self-containment & Bun runtime**: All UnoCSS icons (`@iconify-json/lucide`, `@iconify-json/simple-icons`) and fonts must compile into local assets or CSS data URIs via `@workspace/config/unocss` with zero external CDN requests. Invoke `vite`, `svelte-kit`, and `svelte-check` via `bun x --bun ...` so prerendering runs inside Pixi's locked Bun runtime without leaking to host Node.
4. **Testing without Storybook**: Place reactive view logic in `.svelte.ts` modules tested via `bun test` + `@archont561/bun-test-utils`, and verify subpath asset resolution in `packages/portal/e2e/portal.e2e.test.ts` using `testServer` + `webPage`.
