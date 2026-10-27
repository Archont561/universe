---
id: decision-5
title: Static Conda channel on GitHub Pages + GitHub Releases via CEP-15 base_url, CEP-16 shards, and immutable JSON fragments
date: '2026-10-26 20:15'
status: accepted
---

## Context

Users must be able to host both a human-readable package documentation site and a working Conda channel directly on GitHub Pages while storing large binary `.conda` packages (which routinely exceed 50–300 MB for C++/GIS tools like CloudCompare and ODM) on GitHub Releases—without hitting GitHub Pages' 1 GB site limit or re-downloading gigabytes of `.conda` archives on every index rebuild.

CEP-15 (`repodata_version: 2`) introduces `info.base_url` in `<subdir>/repodata.json`, allowing repodata hosted on GitHub Pages (`https://<owner>.github.io/<repo>/<subdir>/repodata.json`) to redirect package downloads to an external URL prefix. However, CEP-15 defines **one `base_url` string per `<subdir>/repodata.json`**, not per individual package version. If packages were scattered across versioned tag releases (`/releases/download/v2.13/...`, `/releases/download/v2.14/...`), a single `<subdir>/repodata.json` could not address historical versions simultaneously.

## Decision

1. **Rolling per-subdir GitHub Releases**: When publishing to a `github-pages` target, `.conda` binaries upload to deterministic per-subdir GitHub Releases (`packages-linux-64`, `packages-linux-aarch64`, `packages-win-64`, `packages-osx-64`, `packages-osx-arm64`, `packages-noarch`, configurable via `release_tag_prefix`). Because `.conda` filenames `<name>-<version>-<build>.conda` are unique across versions, all versions for `<subdir>` coexist under a single stable `base_url`:
   `https://github.com/<owner>/<repo>/releases/download/packages-<subdir>`
   (`universe-kit` automatically garbage-collects oldest builds per package if configurable retention or GitHub's 1,000-asset release ceiling is approached.)
2. ** ~1 KB immutable JSON metadata fragments**: During `universe-kit audit`, each verified `.conda` archive emits `meta/<subdir>/<filename>.conda.json` containing `index.json`, `about.json`, `run_exports.json`, `sha256`, `md5`, `size`, upload timestamp, and `goblin` binary audit metadata. Fragments persist on a dedicated metadata branch (`universe-index`, or attached to a `packages-meta` release) so `universe-kit pages build` synthesizes the entire channel + docs portal in milliseconds with zero `.conda` binary downloads.
3. **Full modern Conda index emission (`universe-kit pages build`)**:
   - `<subdir>/repodata.json` + `<subdir>/repodata.json.zst` (`repodata_version: 2`, `info: { subdir, base_url }`, deterministic BTreeMap key ordering).
   - `<subdir>/repodata_shards.msgpack.zst` + `<subdir>/shards/<sha256>.msgpack.zst` (CEP-16 sharded repodata for near-instant `pixi` and `rattler` solves).
   - `<subdir>/run_exports.json` + `<subdir>/run_exports.json.zst` (for downstream `rattler-build` consumers).
   - Root `channeldata.json`.
   - Always initialize all configured + standard subdirs (`noarch`, `linux-64`, `linux-aarch64`, `win-64`, `osx-64`, `osx-arm64`) with valid empty `repodata.json` files so multi-platform `pixi lock` never 404s when a subdir initially has zero packages.
4. **Explicit CEP-15 client floor**: Generated portal copy and `README` guidance explicitly state the CEP-15 client floor (`pixi >= 0.40`, `rattler-build`, `conda >= 24.5` with `conda-libmamba-solver >= 24.7`, `mamba`/`micromamba >= 2.0`), noting that legacy `repodata_version: 1` Python solvers reject `repodata_version: 2`.

## Consequences

- Any public GitHub repository becomes a zero-cost, multi-version Conda channel + documentation portal with unlimited binary storage on GitHub Releases and sub-second `pixi` resolution via CEP-16 shards.
- Rebuilding channel repodata and HTML docs requires reading only kilobytes of JSON fragments.
