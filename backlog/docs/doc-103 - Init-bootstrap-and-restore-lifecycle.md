---
id: doc-103
title: Init bootstrap and restore lifecycle
type: specification
created_date: '2026-10-25T23:49:00Z'
updated_date: '2026-10-27T09:20:00Z'
---

# Init bootstrap and restore lifecycle

**Status: agreed workflow; not implemented by the version-only CLI.** Proposed command names identify responsibilities, not a finalized parser/API.

## Corrected user experience

The initializer runs on the project/export side. The target user runs only a generated restore entrypoint; they do not first install Universe Airlock manually or invoke an initializer on the offline machine.

```text
project maintainer
  -> Universe Airlock init
     -> project configuration and persistent request data
     -> restore entrypoint(s)
     -> fixed, thin bundle workflow

online bundle workflow
  -> download pinned Universe Airlock, verify archive checksum, extract, check version
  -> plan / fetch / package / verify / offline-test responsibilities
  -> publish immutable payload data
  -> publish bootstrap content and advance stable bootstrap ref last

target user
  -> generated shell/PowerShell restore entrypoint
  -> select platform and clone bootstrap ref
  -> verify and start native bootstrap executable
  -> bootstrap uses bundled Pixi
  -> install Universe Airlock from bundled/local original packages through Pixi
  -> Pixi-managed Universe Airlock is available with pinned catalog commitments
  -> lazily consume through an approved Git source, or import/prefetch a sealed profile closure
```

Where repository transport is available, the entrypoint clones the designated bootstrap ref. A filesystem-only transfer must supply equivalent pinned bootstrap/payload content without introducing an Internet dependency; precise removable-media packaging is still to be finalized.

## What init owns

The intended project metadata root is `.universe-airlock/`. It holds configuration, target/profile selection, curated dependency requests, and sync/publication data. It is persistent data, not a collection of per-dependency executable workflow files.

The initializer creates an initial restore script/workflow/config set. Adding an admitted dependency later updates data and uses the existing trusted workflow. A questionnaire may help a human at initialization time, but CI must be noninteractive and consume explicit configuration.

Shell entrypoint formats, exact config filenames, and generated workflow filename are not frozen. Bash/zsh/fish integration should be optional and explicit, never an unconditional shell-profile edit. `.airlock` was discussed as a possible target-state location; the target-side state directory remains undecided.

## Bootstrap contract

The bootstrap is a small, native, independently launchable program. It carries enough verified metadata to locate/check the payload and establish the bundled Pixi/managed-Universe Airlock environment. It must not require a preinstalled language ecosystem just to begin restoring.

Bundle original conda packages/metadata needed for Pixi to create the environment on the target. Do not confuse this with transferring a preinstalled Pixi prefix. Pixi owns the installed Universe Airlock environment and package update lifecycle.

A minimal protocol-sharing boundary may eventually be useful between bootstrap and application; this does not authorize linking the complete future core, adapters, HTTP server, or TUI into the bootstrap. The current workspace has no bootstrap executable or protocol crate.

## Target restore responsibilities

1. Pin the bootstrap revision/manifest and selected immutable payload commit IDs.
2. Check platform/ABI/profile and available storage; fail with actionable errors if unsupported.
3. Acquire/import only the selected snapshot's objects and reconstruct transport shards safely.
4. Verify hashes and publisher/trust evidence before execution/materialization.
5. Use bundled Pixi and local package metadata to establish Pixi-managed Universe Airlock.
6. Generate the target-local views/configuration and restore the declared locked environment or admitted scenario.
7. Activate new state only after verification; preserve recoverable prior state on failure.

No target-side bootstrap step may fall back to public registries, `npx`, rustup downloads, remote container images, or undeclared tools when required content is missing.

## Later additions, synchronization, and updates

Curated requests remain project data. An explicit sync makes selected request/config changes available to CI; fetching a request is not the same as publishing a verified snapshot. Immutable existing payload may be reused, but stable bootstrap state must never point to incomplete publication.

Self-update is not implemented. Proposed ownership rule: Pixi-managed installs update through Pixi; a future standalone-owned install may use an explicit, receipt-aware update flow. Neither path silently replaces an executable owned by another installer.

## Thin bootstrap and readiness refinement

Bootstrap needs only its tool-install closure and pinned catalogs, not all project dependencies. It can register a verified Universe Airlock launcher in an existing user-bin directory or emit explicit activation; it cannot mutate an already-running parent shell. Preserve Pixi ownership and avoid bootstrap/install circularity. Approved-Git-connected lazy execution and sealed full-airgap readiness are separate modes; see [catalog execution](doc-109%20-%20Git-backed-catalogs-and-on-demand-command-execution.md). The current root restore.sh instead belongs to the [interim pixi-sandbox developer transport](doc-110%20-%20Interim-development-transport-with-pixi-sandbox.md), not an implemented Universe Airlock initializer.
