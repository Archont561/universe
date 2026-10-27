---
id: doc-110
title: Interim development transport with pixi-sandbox
type: guide
created_date: '2026-10-27T09:20:00Z'
updated_date: '2026-10-27T09:36:00Z'
---

# Interim development transport with pixi-sandbox

**Status: initialized and locally checked on 27 October 2026; no developer transport packed, restored, or published.** This is developer tooling for this Rust repository, not an implementation of Universe Airlock's future init/exec/cache/registry commands.

## Selected use

Use **Archont561/pixi-sandbox 0.9.0**, pinned in the shared Pixi **utils** feature and pixi.lock for the configured platforms. The producer channel is `https://prefix.dev/archont561/archont561`; package source is explicit. Root Node tooling remains npm-managed and Python remains the repository automation language.

Start with one reviewed **universe-dev / default / linux-64** bundle, with Cargo vendoring enabled. The default environment already includes rust/utils/python/typescript, so do not export five duplicate focused prefixes by default. The planned transport ref is `sandbox/universe-dev-linux-64`.

Current pixi-sandbox carries original conda archives/wheels and installs them natively with Rattler/UV; it is not an installed-prefix copier. Cargo vendoring carries the locked workspace's crate sources. This supports fixed-project development while Universe Airlock's reusable catalogs/registry views are still unimplemented. Source locks remain canonical; the native tool's supported lock schema is v7, matching this repository's current pixi.lock.

## Initialization and repository policy

The native initializer owns reviewed config plus generated restore/workflow files. Preserve native generated workflow renders as **inactive source templates under config/pixi-sandbox/**, and a root **restore.sh** entrypoint. Active repository CI uses a separately maintained, manual, named-Pixi-task publisher. This avoids enabling the upstream relock/upgrade bot or introducing direct-shell run steps into the active workflows.

The root restore entrypoint is generated consumer glue, not a replacement for Python automation under scripts/. It restores a previously published/transported developer branch; initialization alone does not create or publish that branch. Existing ci.yml/release.yml, prefix environment/OIDC policy, hooks, identities, and VERSION-only application scope remain unchanged.

Python-dispatched tasks expose the native CLI, initialization, drift/plan checks, standalone bootstrap and explicitly gated publication. Native init/check use the same non-default template paths; running raw `pixi-sandbox init` without those paths would generate different active workflows and is not the project regeneration entrypoint.

Publication must be opt-in/manual, operate only on the trusted main-branch workflow/repository, and never run as part of local setup/validation. Any standalone executable embedded in a transport must be a verified real release binary, not a Pixi global shim. `config/sandbox-bootstrap.json` pins all five release asset sizes/digests, cross-checked against the release SHA256SUMS; downloads/receipts stay under ignored .pixi/. Download/cache artifacts belong under ignored .pixi/, not source exports.

## Proof and limitations

Initialization/plan/drift checks are distinct from packing, restoring or publishing a real developer transport. Record only checks actually executed. A clean, egress-denied restored build is needed before claiming this specific repository is offline-development verified.

The following are **not automatically supplied** merely by shipping a Pixi environment and Cargo vendor tree:

- npm package payloads for Backlog/skills and other JavaScript tools;
- cargo-deny's advisory database;
- additional downloaded cargo-dist/tool assets;
- rattler-build's independently solved build/test dependencies;
- host linker, system libraries or application services not present in the selected environment.

Do not claim `setup`, the entire CI pipeline, conda packaging, or arbitrary projects are offline-ready based only on successful initialization. Vendor/restore wiring must preserve `.cargo/config.toml`, use the native activation contract, and account for this repository's detached-environment settings.

## Lessons adopted for future Universe Airlock work

- Separate online acquisition from local-only installation and protocol serving.
- One typed authoritative manifest; schema evolution must not let old verifiers silently ignore new required artifact sets.
- Verify transport bytes and materialized results; state transaction boundaries explicitly.
- Semantic Git interfaces, fake/real runner contract tests, and tiny fixtures rather than the repository's huge toolchain environment.
- Independently reusable objects/shards, real peak-restore-disk budgets, destination-filesystem staging, and explicit retention/GC limits.
- Owned generated files with non-writing drift checks and reviewed upgrades.
- Real native-client, denied-egress and cold-cache/negative-control proofs, not just offline flags.

Do not import the full upstream Bun/xtask/automation stack, every runtime dependency, or its environment-centric format as Universe Airlock's permanent artifact model. Generic assets/Playwright remain distinct types, not mandatory conda wrappers. Using the development tool does not add a root license file or change the standalone repository's distribution policy.

## Project commands and generated source

```sh
pixi run --locked -e utils sandbox --version
pixi run --locked -e utils sandbox-init          # regenerate native owned files on POSIX
pixi run --locked -e utils sandbox-init --check  # native non-writing drift audit
pixi run --locked -e utils sandbox-check         # drift + plan + restore.sh syntax
pixi run --locked -e utils sandbox-plan          # reviewed JSON plan / CI matrix output
pixi run --locked -e utils sandbox-bootstrap     # pinned verified standalone tool cache
```

- [Reviewed bundle configuration](../../pixi-sandbox.toml).
- [Generated Linux restore entrypoint](../../restore.sh).
- [Native publisher reference](../../config/pixi-sandbox/publish-sandbox.yml) and [native relock reference](../../config/pixi-sandbox/relock.yml): inactive, exact native renders; do not enable without a separate policy review.
- [Manual active publisher](../../.github/workflows/publish-sandbox.yml): dispatch with publish=false checks the plan only. Actual publication requires publish=true on the trusted repository's main branch, matching workflow/source commit and a clean checkout. Only that job gets contents:write; no prefix.dev/OIDC rights are added.
- [Python orchestration](../../scripts/sandbox.py), available as named Pixi tasks or `pixi run scripts <name>`.

Do not call raw native init without the project's paths: use sandbox-init. The current Linux launcher is maintained on a POSIX host; Windows initialization would render a different shell flavor, so the wrapper refuses it. Other configured platform package versions are locked, not natively exercised here.

## Executed local checks (27 October 2026)

- Native init completed, then native init --check passed with both the package-managed CLI and the pinned standalone release binary.
- Plan contains one universe-dev/default/linux-64 target on ubuntu-24.04, Cargo vendoring enabled, destination sandbox/universe-dev-linux-64.
- Generated restore.sh passes bash -n. The standalone bootstrap executes --version under an empty environment.
- Full local `pixi run --locked ci` passed **66 automation tests and 4 Rust tests**, formatting/Clippy, cargo-deny and active-workflow/action-pin policy checks.
- Publisher guards and raw-asset checksum/cache-tamper behavior have regression coverage. This is not an executed pack/restore/denied-egress proof for the Universe Airlock developer profile.

## References

- [Interim tooling decision](../decisions/decision-116%20-%20Use-pixi-sandbox-for-interim-developer-transport.md), [tooling](doc-107%20-%20Rust-workspace-and-development-tooling.md), [release boundaries](doc-108%20-%20Releases-and-trusted-publishing.md), [future lazy catalogs](doc-109%20-%20Git-backed-catalogs-and-on-demand-command-execution.md).
- Release: https://github.com/Archont561/pixi-sandbox/releases/tag/v0.9.0
- Reviewed upstream source: https://github.com/Archont561/pixi-sandbox/tree/1903b088a0896126dee82e2a8b6b007942303ffc
