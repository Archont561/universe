---
id: UNI-16
title: Cut and verify universe-kit v0.1.0 release to prefix.dev/archont561/archont561
status: To Do
assignee: []
created_date: '2026-10-26 20:15'
updated_date: '2026-10-26 20:15'
labels:
  - release
milestone: m-4
dependencies:
  - UNI-14
  - UNI-15
documentation:
  - backlog/docs/architecture/doc-1 - Monorepo-and-toolchain-architecture.md
  - backlog/docs/roadmap/doc-4 - Milestone-roadmap-and-skills-sh-rollout.md
  - backlog/decisions/decision-1 - Two-tier-product-and-channel-boundary-(archont561-archont561-ships-universe-kit;-consumer-universes-target-arbitrary-channels).md
  - backlog/decisions/decision-8 - Owner-lane-vs-consumer-lane-GitHub-Actions-workflows-and-zero-checkout-OIDC-publishing.md
priority: high
type: task
ordinal: 16000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Configure OIDC Trusted Publishing on `https://prefix.dev/archont561/archont561` for `Archont561/universe` (`release.yml`, `environment: prefix`) and cut the initial `v0.1.0` milestone release (`milestone-v0.1.0` -> `autorelease.yml` -> `v0.1.0` -> `release.yml`):
- Verify native builds across `linux-64`, `win-64`, `osx-64`, and `osx-arm64`.
- Verify OIDC publication + Sigstore attestation upload (`--generate-attestation`) on `prefix.dev/archont561/archont561`.
- Verify GitHub Release creation with `.conda` artifacts, standalone CLI archives, and `SHA256SUMS`.
- Smoke-test clean external installation via `pixi exec -c https://prefix.dev/archont561/archont561 -c conda-forge universe-kit --version`.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 `universe-kit 0.1.0` `.conda` packages for `linux-64`, `win-64`, `osx-64`, and `osx-arm64` are live and attested on `https://prefix.dev/archont561/archont561`.
- [ ] #2 GitHub Release `v0.1.0` contains all 4 `.conda` packages, standalone archives, and verified `SHA256SUMS`.
- [ ] #3 Fresh `pixi global install -c https://prefix.dev/archont561/archont561 -c conda-forge universe-kit` resolves both `universe-kit` and runtime `rattler-build` cleanly.
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 Installation smoke check passes from an isolated directory outside the monorepo checkout.
<!-- DOD:END -->
