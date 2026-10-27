---
id: doc-108
title: Releases and trusted publishing
type: guide
created_date: '2026-10-25T23:49:00Z'
updated_date: '2026-10-27T09:36:00Z'
---

# Releases and trusted publishing

**Status: automation configured; no remote release/prefix publication has been performed.** Application-release plumbing does not implement dependency bundle publication.

## Exact identities

Repository: `Archont561/aripack`. Binary/crate: `universe-airlock`. Channel: `archont561/archont561`. Recipe: `crates/universe-airlock/recipe.yml`. Publishing workflow: `.github/workflows/release.yml`. GitHub environment: **`prefix`**.

## Separate workflows and responsibilities

1. **ci.yml:** development checks, external Rust tests, tooling validation, dependency policy, and coverage.
2. **release.yml:** native application archives, rattler-build conda packages, validation, and guarded publication.
3. **Current publish-sandbox.yml:** manual interim developer-transport publication using pixi-sandbox. No upstream relock/upgrade bot is active.
4. **Future Universe Airlock init-generated bundle workflow:** pinned initial Universe Airlock download then delegated product plan/fetch/package/verify/offline-test/publish operations. It is not generated or implemented yet.

All run steps in the repository workflows invoke named `pixi run` tasks. Actions and bootstrap tools are pinned in committed config. No separate ad-hoc CI tool installation path replaces the workspace lock.

## Application release flow

Configured native jobs use Linux GNU x64, Windows MSVC x64, and macOS ARM64. Linux is the only platform exercised locally; foreign recipe rendering/dependency solving is not native Windows/macOS execution or offline-product validation.

Each native job checks the version/tag, formats/lints/tests/denies dependencies, builds a cargo-dist archive/checksum, extracts it and checks `universe-airlock --version`, builds the explicit rattler recipe, and natively tests the installed package. Native validation records bind expected files/platforms to a Git revision. Global assembly requires every configured native record, verifies bytes, merges dist manifests, and produces release metadata/checksums.

`release-local` and global assembly require a clean committed source tree. The present uncommitted scaffold is not a releasable tagged build. `dist-build` and `conda-build` are non-publishing development checks.

Unix archives are compressed tarballs; Windows archives are ZIP files. Extra raw executable assets are not required. The repository cleanup removes existing export/archive files, not this agreed future distribution format.

The initial Linux conda recipe uses a glibc 2.39 sysroot and declares its glibc/libgcc runtime requirements; Windows runtime requirements are declared separately. GNU archives are not musl/static portability guarantees. Dependency legal metadata may be generated for a future package only after a distribution policy is selected; generated packaging outputs are not source artifacts.

## Trusted publication and failure behavior

The publish job uses GitHub environment `prefix`, contents-write for GitHub assets, and id-token-write for prefix.dev OIDC. Upload to `archont561/archont561` uses rattler-build trusted publishing rather than a long-lived prefix token.

Guard publication by repository, version tag, workflow, environment, native evidence, commit, and digests. Do not overwrite immutable released assets or use unsafe skip-existing behavior to hide byte mismatches. Stage GitHub assets as a draft, upload prefix packages, then announce the GitHub release.

GitHub and prefix publication is **not atomic across services**. A prefix failure may leave a draft GitHub release and some already-published packages; recovery requires review/comparison, not an overwrite shortcut.

## Owner-side setup still required

- Create/select `archont561/archont561` and configure prefix.dev Repository Access for GitHub owner `Archont561`, repository `aripack`, workflow `release.yml`, environment `prefix` (case-sensitive), with narrowly scoped read/write access.
- Create/protect the GitHub `prefix` environment, preferably with reviewers/tag deployment restrictions; protect release tags.
- Enable/authorize Codecov for the repository. CI produces cargo-llvm-cov LCOV, uploads the explicit report through the pinned OIDC action, and keeps coverage artifacts even when fork uploads are skipped. `codecov.yml` currently uses automatic project baseline/1% tolerance and 85% patch coverage.
- Select an explicit distribution policy before public release. The standalone source tree currently carries no root license grant or package-license metadata.

## Compressed bootstrap download boundary

The future generated online workflow may curl a pinned archive, verify an expected SHA-256, extract, and run `universe-airlock --version`. Pin the digest in reviewed/trusted input; downloading an unchecked checksum next to an unchecked archive does not independently authenticate a publisher. This is the only intended bootstrap block before application delegation, not a second restore implementation.

The target-side restore entrypoint/bootstrap lifecycle is described separately in the init/restore specification. There is no published release URL or implemented initializer to exercise end-to-end yet.

## Interim developer transport publication

The [pixi-sandbox integration](doc-110%20-%20Interim-development-transport-with-pixi-sandbox.md) adds a separate manual Git-transport publisher for sandbox/universe-dev-linux-64. It uses repository contents permission, not prefix.dev OIDC, and does not alter release.yml or environment prefix. Native upstream relock/upgrade workflow renders are inactive references, not enabled bots. Initialization/checks do not push, publish, or prove an offline restored build.
