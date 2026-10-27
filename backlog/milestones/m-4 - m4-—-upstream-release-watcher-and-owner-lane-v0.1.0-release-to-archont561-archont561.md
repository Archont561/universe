---
id: m-4
title: "M4 — Upstream release watcher and owner-lane v0.1.0 release to archont561/archont561"
---

## Description

Implement `universe-kit watch` (polling GitHub Releases/tags, PyPI, and `crates.io` across `recipes/*` with deduplicated issue/PR creation), finalize the 4-platform co-located Conda recipe (`crates/universe-kit/recipe/recipe.yaml` bundling `rattler-build` as a runtime dependency), and cut the headless `universe-kit v0.1.0` onto `https://prefix.dev/archont561/archont561` via OIDC Trusted Publishing with Sigstore attestations and GitHub Release binaries. Per decision-11 this release contains the committed offline portal placeholder rather than the SvelteKit SSG; no release job builds the browser portal, and the real portal ships as `v0.2.0` in M6.

Exit condition: headless `v0.1.0` builds natively on `linux-64`, `win-64`, `osx-64`, and `osx-arm64`, publishes attested `.conda` artifacts to `archont561/archont561`, and installs cleanly via `pixi exec -c https://prefix.dev/archont561/archont561 -c conda-forge universe-kit --version`.
