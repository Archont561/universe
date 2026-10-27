---
id: m-6
title: "M6 — Browsable portal from pinned catalog snapshots"
---

## Description

The `packages/config`, `packages/schema`, `packages/portal` and `crates/universe-assets` scaffolds are foundations only; behavioral exit conditions remain outstanding.

Deliver the human-facing catalog and docs portal as a required component of the integrated v1, not a separate `v0.2.0` release. Build the portable SvelteKit 3 + Svelte 5 + UnoCSS static portal (`@universe/portal`, UNI-5), complete unit/property/snapshot/subpath browser suites and real asset embedding (UNI-6), and generate hydrated catalog/package pages from verified, pinned release manifests and a catalog snapshot (UNI-25/UNI-46). This milestone does not own registry protocol behavior: native clients use the separately generated package indexes and artifacts.

The portal build must be deterministic and offline once its pinned snapshot and local dependencies are present. GitHub Release discovery/acquisition is an explicitly connected input stage; it is not performed as an unpinned network lookup during static-site generation.

Exit condition: the portal emits the doc-3 browsing tree from pinned fixtures and release data, passes unit/property/snapshot and non-root `/subpath/` Playwright E2E with network guard enabled, and is integrated into the v1 artifact/release workflow. The portal page links alone do not satisfy the static Conda channel or any native registry protocol.