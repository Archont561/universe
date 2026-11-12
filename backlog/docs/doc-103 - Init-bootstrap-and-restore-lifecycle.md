---
id: doc-103
title: Init bootstrap and restore lifecycle
type: specification
created_date: '2026-10-25T23:49:00Z'
updated_date: '2026-10-27T09:20:00Z'
---

# Init bootstrap and restore lifecycle

**Status: agreed workflow; the entrypoint and the bootstrapper both exist, the Pixi handover and
the sealed lane do not.**
`universe airlock render` (2026-11-06, `UNI-53`) writes the committed POSIX and PowerShell
entrypoints from `[airlock.developer.restore]`, drift-checks them, and hands over to a
bootstrapper with `restore --commit <pin>`. `universe-airlock-bootstrap` (2026-11-08, `UNI-33`)
now answers `version`, `verify`, `restore`, and `doctor`: it verifies a closed local payload —
manifest, commit, every artifact's size and digest, authenticity — and materialises it into an
immutable environment that it activates by swapping one symlink (`decision-136`). A failure
leaves the previous environment active and untouched.

Two parts of the loop below are still missing, and the document should not be read as claiming
them: installation into a **bundled Pixi** environment (the payload shape is what `UNI-51`'s
mirror publishes — note that as of 2026-11-12 the bootstrap installs a prefix itself in Rust, so
what is missing there is the Pixi-managed lane and the lock format, not installation as such) and
the **sealed lane** (a signature verifier and a compiled-in key, `UNI-55`;
until then `verify` refuses any signature rather than accepting one it cannot check, and sealed
restores refuse an unsigned payload). Proposed command names still identify responsibilities
rather than a finalized parser/API.

**2026-11-12: the payload half of "publish immutable payload data" now has a producer.**
`universe airlock pack` writes `.universe-airlock/payload` — `universe-release.json` naming every
released artifact with its digest, size and the stores it goes to, plus one file per artifact —
from what the build produced and the archives' own `info/index.json`. Two choices are worth
recording because they constrain the loop: the payload's pin is the `[airlock.developer.restore]`
commit (the same value the entrypoint verifies, so the two cannot disagree), and the directory is
fixed rather than derived from the entrypoint's `dir` (the bootstrapper resolves that path itself
and the rendered script passes no `--payload`). A directory holding a file the descriptor does not
name is refused rather than cleaned, because the consumer rejects unexpected files and a signature
over the previous manifest would be left describing bytes that are gone. The restore that follows
still copies artifacts into an activated environment; replaying a lock into a *prefix* remains
`UNI-52`.

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
