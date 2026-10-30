---
id: doc-108
title: Releases and trusted publishing
type: guide
created_date: '2026-10-25T23:49:00Z'
updated_date: '2026-10-27T12:00:00Z'
---

# Releases and trusted publishing

**Status: application release and channel publication are not implemented.** This document records current capabilities and intended future responsibilities; it is not evidence that a release pipeline or trusted publisher is configured.

## Current repository state

- Repository: `Archont561/universe`; application crate: `universe-airlock`.
- The Airlock Conda recipe scaffold is `crates/universe-airlock/recipe/recipe.yaml`, with variants alongside it. It has not been built with `rattler-build` or validated in a package environment.
- `ci.yml`, `release.yml`, and `autorelease.yml` do not currently exist. No prefix.dev channel or GitHub `prefix` environment has been configured, and no application release or package publication has occurred.
- `relock.yml` is a lock-repair workflow, not a product release or package publication workflow. `publish-sandbox.yml` is separate interim developer transport and does not publish Universe Airlock.
- Pixi packaging tasks and recipe scaffolds are developer-side preparation, not a complete release process. The repository's lockfiles still require relocking before lock-dependent checks can be claimed reproducible.

## Intended separation of responsibilities

The accepted design direction in [decision-108](../decisions/decision-108%20-%20Separate-CI-release-and-bundle-workflows.md) separates:

1. CI quality gates and package validation.
2. Application release artifacts and any eventual Conda publication.
3. A future init-generated bundle workflow for Universe Airlock product operations.

These remain planned responsibilities. A future application release design may use `cargo-dist` for native archives and `rattler-build` for Conda packages, but neither successful build nor trusted publication is established by the current scaffolds.

## Before enabling publication

- Build and test the recipes on supported native platforms; verify package contents, runtime dependencies, version metadata, and installed CLI behavior.
- Refresh `Cargo.lock` and `pixi.lock`, then run all lock-dependent checks from the refreshed committed state.
- Implement and test release workflows with exact tag/revision checks, artifact provenance and digest verification, least-privilege credentials, and failure/retry behavior.
- Decide the canonical prefix.dev channel and configure its trusted-publishing permissions only after the repository workflow and GitHub environment names are settled.
- Confirm root and package licensing and third-party notice handling before distributing artifacts.
- Hold the release signing key only in the release lane, in a job that runs no untrusted code, and rehearse its rotation — a bootstrap signed by both keys — before it is needed (`decision-131`).

## Interim developer transport

The [pixi-sandbox integration](doc-110%20-%20Interim-development-transport-with-pixi-sandbox.md) is a separate manual Git-transport path for developer environments. It uses neither prefix.dev OIDC nor an application release workflow. Initialization and local checks do not publish anything or prove a restored offline build.
