---
id: UNI-9
title: Implement universe-kit build and test wrappers with native Rust Windows MSVC discovery
status: To Do
assignee: []
created_date: '2026-10-26 20:15'
updated_date: '2026-10-26 20:15'
labels:
  - architecture
  - testing
milestone: m-2
dependencies:
  - UNI-8
documentation:
  - backlog/docs/architecture/doc-2 - Universe Kit-CLI-and-manifest-specification.md
  - backlog/decisions/decision-3 - Hybrid-Rattler-library-+-subprocess-architecture-and-pure-Rust-rustls-dependency-policy.md
priority: high
type: feature
ordinal: 9000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Implement `universe-kit build` and `universe-kit test` (`doc-2`, `decision-3`):
- Resolve `rattler-build` deterministically: `--rattler-build-bin` -> `UNIVERSE_KIT_RATTLER_BUILD` -> sibling of `std::env::current_exe()` (`$CONDA_PREFIX/bin` / `%CONDA_PREFIX%\Library\bin`) -> `PATH`, checking minimum version compatibility (`>=0.58`).
- On `cfg(windows)` (with cross-platform testable abstraction), detect unset `VCINSTALLDIR` and discover Visual Studio / MSVC Build Tools environment variables natively in-process via `cc::windows_registry`, eliminating external `ilammy/setup-dev-cmd` or PowerShell `vswhere` scripts in consumer workflows.
- Run `rattler-build build` with universe channels, `variant_config.yaml`, `--target-platform`, `--output-dir`, `--skip-existing`, and `--no-test`.
- Run `universe-kit test --package-file <pkg.conda>` delegating to isolated `rattler-build test --package-file` and recording structured test timing/outcome receipts.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Sibling executable lookup prefers `$CONDA_PREFIX` `rattler-build` over stale system `PATH` binaries and fails with an actionable diagnostic if `rattler-build` is missing or below minimum SemVer.
- [ ] #2 Command-recording fake tests verify exact CLI flags passed to `rattler-build build` and `rattler-build test` for single-output, `noarch`, and multi-output `staging` recipes.
- [ ] #3 Integration test invokes real `rattler-build` from Pixi to build and test a minimal fixture recipe into a `.conda` archive.
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 `rstest` unit tests and real `rattler-build` integration smoke tests pass under `cargo nextest`.
<!-- DOD:END -->
