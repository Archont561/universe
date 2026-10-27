---
id: UNI-23
title: Ship installed-package functional tests and corresponding-source bundles
status: To Do
assignee: []
created_date: '2026-10-27 09:20'
updated_date: '2026-10-27 09:20'
labels:
  - testing
  - release
milestone: m-2
dependencies:
  - UNI-9
documentation:
  - backlog/decisions/decision-10 - Adopt-ODM-conda-release-governance-and-supply-chain-hardening-patterns.md
  - backlog/docs/architecture/doc-2 - Universe Kit-CLI-and-manifest-specification.md
priority: medium
type: feature
ordinal: 23000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
ODM-conda treats "tests pass in the source tree" as insufficient. Two mechanisms are worth porting to the universe-kit build/test wrappers (UNI-9) and release flow:
- **Tests inside the artifact, run from the installed prefix**: functional test scripts are packaged under `share/universe-kit/<package>/` (alongside a `SOURCE.txt` and `upstream.json`) and executed through `rattler-build test` after the `.conda` is reinstalled into a fresh isolated environment. The tests generate their own small fixtures (no proprietary datasets, no network), check the executable actually runs, its linkage resolves (`ldd`/platform equivalent with no "not found"), and relocation works from the installed prefix rather than the build directory.
- **Corresponding-source bundles**: every release gets a deterministic source archive built from an explicit file allowlist (upstream pinned sources plus packaging recipes, scripts, manifests, locks, docs — never `.git`, caches, credentials, or built binaries), a generated `SHA256SUMS`, and the release-specific source URL recorded in the installed `SOURCE.txt`. Repeated generation must be byte-identical, and the bundle is uploaded to the GitHub Release *before* the binary reaches the channel (ordered in UNI-20's publisher).
- License notices generated at build time land in both `share/licenses/<package>/` and Conda's `info/licenses/*`, preserving upstream bytes verbatim.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 A fixture recipe's packaged functional test fails the build when the installed-prefix executable is broken or has an unresolved linkage, and passes when the artifact is healthy; the test runs in a reinstalled environment, not the build tree.
- [ ] #2 `universe-kit build` refuses success when a recipe declares tests but no test file is present in the installed package, and the packaged tests require no network access (verified with the same network-guard approach used by the portal E2E suite where feasible).
- [ ] #3 A source-bundle command produces an allowlist-filtered tarball plus `SHA256SUMS`; two consecutive runs are byte-identical, and a test mutating an excluded path proves it never appears in the archive.
- [ ] #4 The installed `SOURCE.txt` contains the exact release source URL derived from typed metadata, and license notice files exist in both install locations for a multi-license fixture.
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 The flow is documented in `doc-2` and exercised by an end-to-end nextest suite on at least linux-64, with platform-specific linkage checks abstracted for the other five subdirs.
<!-- DOD:END -->
