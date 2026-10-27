---
id: decision-118
title: Enable pixi-sandbox lock repair for Cargo, Pixi, and Bun
date: '2026-10-27'
status: accepted
---

## Context

Universe now has external Cargo dependencies, a Bun workspace, and a Pixi environment that owns Bun. Hand-editing or copying another repository's lockfiles does not produce a coherent lock for this workspace. The pixi-sandbox relock bot is already able to validate and repair Pixi/Cargo locks; the package workspace adds a Bun lock that needs the same PR repair path.

## Decision

- Enable the pixi-sandbox relock bot for pull requests that change Cargo, Pixi, Bun, or package manifests.
- Keep the native pixi-sandbox-generated relock workflow as a reviewed reference under `config/pixi-sandbox/`. The active `.github/workflows/relock.yml` is a project-owned extension: it also installs/checks Bun and commits `bun.lock` alongside `Cargo.lock` and `pixi.lock`.
- The bot must validate the repaired lock set and dispatch the configured CI/publisher checks on the repaired commit before its review check passes.
- Keep Cargo registry dependencies out of `universe-core` when they are ecosystem-specific. Neutral serialization/error dependencies may live in core; portal asset embedding belongs to `universe-assets`.
- Continue to distinguish developer transport from sealed Universe Airlock snapshots.

## Consequences

The relock workflow is activated by a pull request; it does not update uncommitted local lockfiles. Do not describe any Cargo/Pixi lock as current until the bot has committed its repair and the repaired-head checks pass. Cargo vendoring is enabled for the developer transport so the eventual locked registry graph can be transported with that environment. This does not establish offline npm or full build reproducibility by itself.

## Status

The active workflow and package scaffolds are present on the working branch. The Cargo/Pixi lock refresh is pending a pull-request run; no generated Cargo/Pixi lock update is claimed yet.
