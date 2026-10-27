---
id: doc-104
title: Ecosystem adapters and external assets
type: specification
created_date: '2026-10-25T23:49:00Z'
updated_date: '2026-10-27T09:20:00Z'
---

# Ecosystem adapters and external assets

**Status: adapter contracts/proof obligations; none are implemented in Universe Airlock yet.** Rust remains the application/orchestration language. Native clients or narrowly scoped helpers preserve ecosystem semantics rather than duplicating their solvers.

## Conda and optional Pixi input

Use Rattler-backed acquisition/resolution of original conda archives and authoritative effective channel metadata, including repodata hotfixes. A projectless conda catalog must work without a Pixi process/input and without installing a prefix during acquisition. Pixi is an optional locked-project input and downstream consumer, not the canonical archive format.

Preserve channel/label identity, build strings, channel priority, dependency versus constraint semantics, virtual packages, and `noarch`. Filter the effective upstream records; re-indexing only embedded `info/index.json` can lose hotfixes. A pinned index helper or embedded Rattler indexer can be chosen during implementation; neither is currently a product dependency.

For Pixi projects, cover all selected environments/solve groups for the declared target, not merely the default environment. HTTP mode uses tested local-channel/mirror routing without fallback. Files mode uses real file channels and, when needed, a typed relocated manifest/lock pair validated by the native client's locked/offline behavior.

## PyPI dependencies inside Pixi

A mixed conda/wheel environment is not complete if its PyPI nodes are ignored. Initially accept compatible, hashed wheels tied to a Python/ABI profile. Preserve extras, markers, `Requires-Python`, and relevant conda-to-PyPI mapping.

HTTP mode must redirect both index and artifact origins. Files mode needs a tested local wheelhouse/index or a specifically verified client-cache derivative. Reject unavailable wheel tags, sdists, editable dependencies, and unsupported source build backends rather than silently omitting or rebuilding them under the original identity.

## npm

Keep root development tooling npm-managed; product npm support is a separate future adapter. For a product bundle, preserve lock v2/v3 placement, aliases, peers, optional/dev flags, platform selectors, original tarballs, upstream SRI, and original packuments. Filter versions/tags to the admitted set; do not move `latest` to an arbitrary older version.

- **HTTP:** a read-only registry subset with local packuments/tarballs. Source-origin relocation must preserve lock graph and integrity.
- **Files:** not a `file://` npm registry and not just a directory of tarballs. Seed/transfer a version-pinned native npm cache containing persistent metadata and tarballs, using a recorded logical registry origin. No target listener runs.

A cache origin is a lookup identity, not network permission. Native caches are replaceable derivatives. Their compatibility must be proven using the exact Node/npm/helper fingerprint, cold state, locked install, and a new curated root. Lifecycle scripts are disabled during acquisition/seeding; approved native builds are a separate verification level.

## Cargo

Keep original `.crate` archives, native SHA-256, and real registry index records: renamed/build/dev/target dependencies, features/features2, yanked state, links, and rust-version. `cargo metadata` is not a registry-index format.

For initial locked replay, retain every registry entry in Cargo.lock, including non-host/optional entries, rather than pruning to the active Linux build graph. Certify native builds only for the selected target/profile.

- **HTTP:** sparse index/downloads with source replacement, locked installs/builds, and OS-enforced loopback-only networking.
- **Files:** Cargo local-registry source replacement with offline/frozen behavior; ordinary project-specific vendoring is optional, not the canonical shared catalog.

Original Cargo.lock should remain unchanged when source replacement suffices. Git/alternate registries and external path dependencies require an explicit supported extension or rejection; do not fabricate index semantics or relabel yanked packages.

## Generic downloadable assets

Plain HTTP archives, executables, models, headers, and other downloads are first-class artifact types. Do not force them into conda/npm/Cargo identities. Record origin, size/digest, target constraints, safe extraction/layout policy, provenance, and how the consuming application is configured to use them.

Package-manager tool executables supplied through conda do not automatically include their native registry dependencies or script-downloaded assets.

## Playwright

Treat Playwright browser distributions and auxiliaries as separate pinned assets associated with the locked Playwright version and target OS/architecture. Account for the required browser revision and ancillary components, expected cache/layout/permissions, and supported environment variables such as the browser-path selection.

OS shared libraries, fonts, sandbox capabilities, and host dependencies are a different concern from browser archives. They need a declared native prerequisite/install strategy and native validation. Downloading all assets on Ubuntu does not prove Windows/macOS installation or browser launch compatibility. Approved browser-launch tests must run with egress blocked; skipped downloads or disabled scripts are not a successful application test.

## On-demand native views

The [exec/catalog design](doc-109%20-%20Git-backed-catalogs-and-on-demand-command-execution.md) keeps resolution with supported native clients, uses per-invocation protocol routing, and lazily retrieves exact admitted archives. It is not TLS interception or support for arbitrary build-script downloads. Retain original channel/registry identities and effective metadata; do not publish metadata for unavailable objects.
