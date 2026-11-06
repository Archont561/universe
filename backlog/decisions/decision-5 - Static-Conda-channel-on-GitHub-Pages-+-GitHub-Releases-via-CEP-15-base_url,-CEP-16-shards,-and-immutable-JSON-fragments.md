---
id: decision-5
title: Static Conda channel and catalog portal via CEP-15/16 and immutable JSON fragments
date: '2026-10-26 20:15'
status: accepted
---

## Context

The original design proposed hosting repodata on GitHub Pages and serving package files from rolling GitHub Releases. **V1 distribution amendment (decision-119):** GitHub Release download links alone do not satisfy the native Conda channel contract. Conda package bytes must be published and retrievable at standard channel/subdirectory paths on static hosting/object storage; a portal or a list of release links is not a channel. This decision's metadata/CEP-15/CEP-16 design remains useful, while its release-only artifact path is superseded.

CEP-15 (`repodata_version: 2`) introduces `info.base_url` in `<subdir>/repodata.json`, allowing repodata and package artifacts to be served from a stable static object-store prefix. CEP-15 defines **one `base_url` string per `<subdir>/repodata.json`**, not per individual package version. V1 publishes the actual binaries at stable standard channel/subdirectory paths; immutable GitHub Release assets may be retained as release copies but are not the only package endpoint.

## Decision

1. **Canonical static Conda artifact paths**: Publish repodata, CEP-16 shards, and the actual `.conda` binaries into the standard channel/subdirectory layout on static hosting or object storage, with stable URLs verified by a real native-client install. GitHub Releases may carry immutable release copies and checksums, but are not the sole package-serving surface for v1. The previous rolling per-subdir GitHub Releases-only `base_url` design is retained as historical context and must not be used as a substitute for the static channel.
2. ** ~1 KB immutable JSON metadata fragments**: During `universe-kit audit`, each verified `.conda` archive emits `meta/<subdir>/<filename>.conda.json` containing `index.json`, `about.json`, `run_exports.json`, `sha256`, `md5`, `size`, upload timestamp, and `goblin` binary audit metadata. Fragments persist on a dedicated metadata branch (`universe-index`, or attached to a `packages-meta` release) so `universe-kit pages build` synthesizes the entire channel + docs portal in milliseconds with zero `.conda` binary downloads.
3. **Full modern Conda index emission (`universe-kit pages build`)**:
   - `<subdir>/repodata.json` + `<subdir>/repodata.json.zst` (`repodata_version: 2`, `info: { subdir, base_url }`, deterministic BTreeMap key ordering).
   - `<subdir>/repodata_shards.msgpack.zst` + `<subdir>/shards/<sha256>.msgpack.zst` (CEP-16 sharded repodata for near-instant `pixi` and `rattler` solves).
   - `<subdir>/run_exports.json` + `<subdir>/run_exports.json.zst` (for downstream `rattler-build` consumers).
   - Root `channeldata.json`.
   - Always initialize all configured + standard subdirs (`noarch`, `linux-64`, `linux-aarch64`, `win-64`, `osx-64`, `osx-arm64`) with valid empty `repodata.json` files so multi-platform `pixi lock` never 404s when a subdir initially has zero packages.
4. **Explicit CEP-15 client floor**: Generated portal copy and `README` guidance explicitly state the CEP-15 client floor (`pixi >= 0.40`, `rattler-build`, `conda >= 24.5` with `conda-libmamba-solver >= 24.7`, `mamba`/`micromamba >= 2.0`), noting that legacy `repodata_version: 1` Python solvers reject `repodata_version: 2`.

## Amendment — 2026-11-06: where a branch store keeps its fragments

Item 2 says fragments persist on a dedicated metadata branch. That is right for a store whose
bytes live somewhere else — a release host, an object store — because `pages build` must
synthesize a channel without downloading archives.

A **branch store is the exception, and it is an exception by construction**: `UNI-58` publishes
one orphan snapshot per publish, so the metadata and the bytes are the same tree, obtained by
the same `git clone`, addressed by the same commit. A second branch would be a second thing to
keep consistent, and could describe bytes the snapshot does not hold. `universe publish` and
`universe index` therefore write each fragment into that tree at
`meta/<subdir>/<filename>.conda.json` — the path this decision names, minus the separate
branch — copying the auditor's own text verbatim so a field a later Universe writes survives.
The path is outside the subdir a client scans, so a listing of `linux-64/` still shows one file
per artifact.

A `github-pages` store, when it exists, still owes item 2's dedicated `universe-index` branch:
its bytes are release assets and its metadata is not.

## Consequences

- Static hosting/object storage serves the native Conda channel tree; a separately generated portal provides discovery/docs. GitHub Release copies can support immutable release audit/transport, but do not by themselves satisfy channel availability.
- Rebuilding channel repodata and portal output requires reading only verified JSON fragments/catalog snapshots; package bytes are staged/published at their standard channel paths.
