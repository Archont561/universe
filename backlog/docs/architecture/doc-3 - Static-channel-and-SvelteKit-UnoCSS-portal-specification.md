---
id: doc-3
title: Static channel and SvelteKit UnoCSS portal specification
type: specification
created_date: '2026-10-26 20:15'
updated_date: '2026-10-26 20:15'
tags:
  - architecture
  - channel
  - cep-15
  - cep-16
  - sveltekit
  - unocss
  - approved
---

# Static channel and SvelteKit UnoCSS portal specification

## Status

Approved specification for the dual-purpose static Conda channel + documentation portal output emitted by `universe-kit pages build`, the Rust <-> TypeScript schema contract, and the `@universe/portal` SvelteKit 3 + Svelte 5 + UnoCSS architecture (`decision-4`, `decision-5`, `decision-6`, `decision-7`).

**Delivery order (decision-11, 2026-10-27):** the specification stays approved and unchanged, but its two layers ship in different releases. The CEP-15/CEP-16 channel invariants and solver-facing tree below ship headless in `v0.1.0` via `UNI-12` (no generated HTML); the SvelteKit/UnoCSS portal architecture, `lol_html` hydration, and per-package pages ship in `v0.2.0` via milestone M6 (`UNI-5`, `UNI-6`, `UNI-25`, `UNI-26`). Until then `crates/universe-assets` embeds a committed offline placeholder instead of the SSG output described here.

## Emitted static site + Conda channel tree

A single `universe-kit pages build --fragments meta/ --output _site/` invocation produces a directory that simultaneously serves humans in a browser and Conda solvers (`pixi`, `rattler-build`, `mamba >= 2.0`, `conda >= 24.5`):

```text
_site/
├── .nojekyll
├── index.html                                # Hydrated catalog explorer + quickstart install bar
├── catalog.json                              # Full universe metadata + package summary cards
├── channeldata.json                          # Conda channel-wide summary index
├── assets/                                   # Vite hashed JS/CSS + self-hosted woff2 fonts
├── packages/
│   └── <pkg-name>/
│       ├── index.html                        # Cloned from __pkg__ shell + lol_html hydrated
│       └── package.json                      # Version matrix, deps, run_exports, SPDX, goblin audit, evidence_html
├── noarch/
│   ├── repodata.json                         # CEP-15 v2 repodata (always emitted, even if empty)
│   ├── repodata.json.zst
│   ├── repodata_shards.msgpack.zst           # CEP-16 sharded repodata index
│   ├── shards/<sha256>.msgpack.zst           # Per-package msgpack.zst shards
│   ├── run_exports.json
│   └── run_exports.json.zst
├── linux-64/                                 # Same structure per subdir...
├── linux-aarch64/
├── win-64/
├── osx-64/
└── osx-arm64/
```

## CEP-15 & CEP-16 channel invariants (`decision-5`)

1. **`repodata_version: 2` + per-subdir `info.base_url`**:
   ```json
   {
     "info": {
       "subdir": "linux-64",
       "base_url": "https://github.com/<owner>/<repo>/releases/download/packages-linux-64"
     },
     "packages": {},
     "packages.conda": {
       "cloudcompare-2.13.2-h1234567_0.conda": {
         "build": "h1234567_0",
         "build_number": 0,
         "depends": ["libgcc >=13", "qt6-main >=6.7,<6.8.0a0"],
         "license": "GPL-2.0-or-later",
         "md5": "...",
         "name": "cloudcompare",
         "sha256": "...",
         "size": 48291042,
         "subdir": "linux-64",
         "timestamp": 1761500000000,
         "version": "2.13.2"
       }
     },
     "removed": [],
     "repodata_version": 2
   }
   ```
   - Solvers fetch tiny metadata from GitHub Pages (`https://<owner>.github.io/<repo>/linux-64/repodata_shards.msgpack.zst` or `repodata.json.zst`) and resolve package binary URLs by joining `{info.base_url}/{filename}` directly against GitHub Releases CDN (`objects.githubusercontent.com`).
2. **Deterministic byte output**:
   - All maps (`packages`, `packages.conda`, `shards`, `channeldata`) serialize through `BTreeMap` with canonical key ordering so `pages build` produces byte-identical output for any permutation of input `.conda.json` fragment files (enforced via `proptest`).
3. **Zero-404 empty subdir guarantee**:
   - Even if a universe currently ships only `linux-64` and `win-64` packages, `pages build` emits valid empty `repodata.json(.zst)` + `repodata_shards.msgpack.zst` for `noarch`, `linux-64`, `linux-aarch64`, `win-64`, `osx-64`, and `osx-arm64` so multi-platform `pixi lock` never fails with HTTP 404.
4. **Fragment contract (`meta/<subdir>/<filename>.conda.json`)**:
   - Emitted by `universe-kit audit --emit-fragment`, containing `schema: 1`, `filename`, `subdir`, `size`, `sha256`, `md5`, `index_json`, `about_json`, `run_exports_json`, `audit_summary` (architecture verified, RPATH clean, license files list, inspected binaries count, shared library dependencies), and optional `test_summary`.

## Intended Rust <-> TypeScript bridge (`@universe/schema`, `decision-4`)

This section describes the target architecture only. The current Rust core has no `schema/portal.rs` source and the schema workspace currently contains hand-maintained portal payload contracts. Once the Rust models and generator exist, they are intended to export via `ts-rs` to generated bindings:
- `CatalogJson`:
  - `schema: u32`
  - `universe: UniverseHeader` (`name`, `title`, `tagline`, `description`, `repository`, `accent_color`, `generated_at`)
  - `channels: Vec<ChannelEndpoint>` (`label`, `url`, `kind`, `cep15_floor_notice: bool`)
  - `stats: UniverseStats` (`total_packages`, `total_artifacts`, `subdirs: Vec<String>`)
  - `packages: Vec<PackageCardSummary>` (`name`, `latest_version`, `summary`, `license`, `subdirs: Vec<String>`, `category`, `featured`, `updated_at`, `audit_passed: bool`)
- `PackageDetailJson`:
  - `schema: u32`
  - `name`, `summary`, `description`, `homepage`, `repository`, `doc_url`, `license`, `license_family`
  - `latest_version: String`
  - `install: InstallInstructions` (pre-computed `pixi add`, `pixi global install`, `conda install`, `mamba install`, and `pixi.toml` channel snippets for each configured channel endpoint)
  - `versions: Vec<VersionReleaseRow>` (grouped by version -> per-subdir artifact download URL, `sha256`, `size_bytes`, `build_string`, `build_number`, `timestamp`, `depends`, `constrains`, `run_exports`)
  - `audit: AuditReportView` (`status`, `spdx_expression`, `license_files`, `binaries: Vec<BinaryAuditRow>` showing format `ELF64`/`PE32+`/`MachO64`, architecture, RPATH status, and top dynamic library imports)
  - `evidence_html: Option<String>` (rendered from `package.toml` `docs.evidence_markdown` via `pulldown-cmark` + `ammonia` HTML sanitization)

## SvelteKit 3 + Svelte 5 + UnoCSS architecture (`@universe/portal`, `decision-6`)

### 1. Portable relative SSG configuration (`packages/portal/vite.config.ts`)

SvelteKit `3.1+` removes `svelte.config.js` (`config_file_unsupported`) and accepts kit options directly inside `sveltekit({ ... })` in `packages/portal/vite.config.ts` alongside `@workspace/config/unocss` and `#lib/*` subpath imports:

```typescript
import adapter from "@sveltejs/adapter-static";
import { sveltekit } from "@sveltejs/kit/vite";
import UnoCSS from "unocss/vite";
import { defineConfig } from "vite";

export default defineConfig({
  plugins: [
    UnoCSS(),
    sveltekit({
      adapter: adapter({
        pages: "build",
        assets: "build",
        fallback: undefined,
        precompress: false,
        strict: true,
      }),
      appDir: "assets",
      paths: {
        relative: true,
      },
    }),
  ],
});
```

Root `+layout.ts` exports `export const prerender = true;` and `export const trailingSlash = 'always';`.

### 2. UnoCSS setup (`packages/portal/uno.config.ts`)

- Integrated via `unocss/vite` (`unocss = "^66.11.0"`).
- Presets:
  - `presetWind3()` — utility classes + CSS custom property theming (`--cu-accent` overridden dynamically from `universe.toml` `pages.accent_color`).
  - `presetIcons()` — offline icon collections `@iconify-json/lucide` (UI chrome: search, copy, shield-check, package, terminal, external-link) and `@iconify-json/simple-icons` (`linux`, `apple`, `windows`, `github`, `condaforge`), compiled into inline CSS data URIs with zero runtime network requests.
  - `presetTypography()` — clean prose styling for `evidence_html` and package long descriptions.
- Transformers: `transformerDirectives()` and `transformerVariantGroup()`.
- Self-hosted variable `.woff2` fonts (`Inter` for UI, `JetBrains Mono` for CLI snippets, hashes, and dependency specs) under `packages/portal/static/fonts/`.

### 3. Template-route hydration (`lol_html`) & dual-mode data loading

To work both on direct cold loads (`https://org.github.io/my-universe/packages/cloudcompare/`) and client-side SvelteKit transitions (`/` -> `/packages/cloudcompare/`):
1. `universe-kit pages build` clones `build/packages/__pkg__/index.html` -> `_site/packages/<name>/index.html`.
2. `lol_html` rewrites:
   - `<title>` and `<meta name="description">` / `<meta property="og:*">` with the package's real name, version, and summary.
   - `<script id="cu-pkg-data" type="application/json">...</script>` with the serialized `PackageDetailJson` (escaping `</script>` sequences safely as `<\/script>`).
3. In `src/routes/packages/[name]/+page.ts`:
   - During SvelteKit compile-time prerender (`name === '__pkg__'`), returns a typed placeholder shell model.
   - In the browser, checks `document.getElementById('cu-pkg-data')` if matching `params.name` on initial load (zero network waterfall), otherwise runs `await fetch('./package.json')` on client-side route navigation.
4. Similarly, `src/routes/+page.ts` reads inline `<script id="cu-catalog-data" type="application/json">` on initial `/` load or `await fetch('./catalog.json')` on client-side navigation back to `/`.

### 4. Portal UI features & pure `.svelte.ts` view-models

Reactive state lives in pure Svelte 5 rune modules (`src/lib/state/*.svelte.ts`) directly testable in `bun test` without Storybook:
- `catalog-filter.svelte.ts`: instant search across `name`, `summary`, `license`, and `category`; filter chips by platform (`linux-64`, `win-64`, `osx-64`, `osx-arm64`, `linux-aarch64`, `noarch`) and category; URL query-param synchronization.
- `install-snippet.svelte.ts`: switches install commands across `pixi`, `pixi global`, `mamba`, and `conda` across all configured channels (`prefix.dev`, GitHub Pages CEP-15 URL, `anaconda.org`), displaying the CEP-15 solver version notice (`pixi >= 0.40`, `conda >= 24.5`, `mamba >= 2.0`) whenever the GitHub Pages channel tab is selected.
- `platform-matrix.svelte.ts`: renders per-version, per-subdir artifact tables with copyable `sha256` digests, human-formatted file sizes, direct GitHub Release `.conda` download links, dependency trees, `run_exports` badges, and `goblin` binary audit verification drawers.
