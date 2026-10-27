---
id: UNI-12
title: Implement fragment-driven headless CEP-15/CEP-16 channel indexer (universe-kit pages build)
status: To Do
assignee: []
created_date: '2026-10-26 20:15'
updated_date: '2026-10-27 13:40'
labels:
  - channel
  - testing
milestone: m-3
dependencies:
  - UNI-10
documentation:
  - backlog/docs/architecture/doc-3 - Static-channel-and-SvelteKit-UnoCSS-portal-specification.md
  - backlog/decisions/decision-5 - Static-Conda-channel-on-GitHub-Pages-+-GitHub-Releases-via-CEP-15-base_url,-CEP-16-shards,-and-immutable-JSON-fragments.md
  - backlog/decisions/decision-11 - Headless-v0.1.0-first,-browser-portal-deferred-to-v0.2.0.md
priority: high
type: feature
ordinal: 12000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Implement `crates/universe-core::channel_index` and the headless solver-data half of
`universe-kit pages build` (`doc-3`, `decision-5`, `decision-11`). Per decision-11 the
browser HTML layer is deferred to `v0.2.0` and lives in `UNI-25`; this task emits only the
Conda solver contract:
- Read `meta/<subdir>/*.conda.json` fragments (+ `universe.toml` and optional
  `recipes/*/package.toml` metadata) without requiring `.conda` binary archives on disk.
- Emit for every configured + standard subdir (`noarch`, `linux-64`, `linux-aarch64`,
  `win-64`, `osx-64`, `osx-arm64`), even when empty:
  - `<subdir>/repodata.json` + `<subdir>/repodata.json.zst` (`repodata_version: 2`,
    `info.subdir`, `info.base_url = "https://github.com/<owner>/<repo>/releases/download/<prefix>-<subdir>"`
    or custom `base_url`).
  - `<subdir>/repodata_shards.msgpack.zst` + `<subdir>/shards/<sha256>.msgpack.zst`
    (CEP-16 sharded repodata).
  - `<subdir>/run_exports.json` + `<subdir>/run_exports.json.zst`.
  - Root `channeldata.json` and `.nojekyll`.
- No portal extraction, HTML cloning, or `lol_html` hydration runs in headless mode; the
  embedded placeholder from `universe-assets` is left untouched and `catalog.json` /
  per-package HTML generation is implemented in `UNI-25`.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 `proptest` proves `pages build` produces byte-identical `repodata.json`, `repodata.json.zst`, and `repodata_shards.msgpack.zst` across arbitrary permutations of input fragment discovery order.
- [ ] #2 Empty subdirs always contain valid `repodata_version: 2` files so multi-platform `pixi lock` never receives HTTP 404.
- [ ] #3 End-to-end integration test starts local HTTP servers (one serving `_site/` as Pages and one serving `.conda` files as simulated GitHub Releases `base_url`) and proves real `rattler` / `pixi` resolves, downloads via `base_url` redirect, and installs a package from the generated static channel.
- [ ] #4 The headless output directory contains only Conda solver artifacts (`repodata*`, shards, `run_exports*`, `channeldata.json`, `.nojekyll`); a test asserts no portal extraction or HTML hydration is invoked before `UNI-25` lands.
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 `rstest-bdd` (`pages_build.feature` headless scenarios), `proptest`, and real local HTTP Conda resolution tests pass under `cargo nextest`.
<!-- DOD:END -->
