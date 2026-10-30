---
id: UNI-11
title: Implement multi-target artifact publisher (prefix.dev OIDC, S3, Anaconda, Quetz, Artifactory, and GitHub Releases)
status: To Do
assignee: []
created_date: '2026-10-26 20:15'
updated_date: '2026-10-26 20:15'
labels:
  - channel
  - release
  - testing
milestone: m-3
dependencies:
  - UNI-10
  - UNI-31
  - UNI-41
documentation:
  - backlog/docs/architecture/doc-2 - Universe Kit-CLI-and-manifest-specification.md
  - backlog/decisions/decision-1 - Two-tier-product-and-channel-boundary-(archont561-archont561-ships-universe-kit;-consumer-universes-target-arbitrary-channels).md
  - backlog/decisions/decision-5 - Static-Conda-channel-on-GitHub-Pages-+-GitHub-Releases-via-CEP-15-base_url,-CEP-16-shards,-and-immutable-JSON-fragments.md
priority: high
type: feature
ordinal: 11000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Implement `crates/universe-core::publish` and `universe-kit publish-artifacts` (`doc-2`, `decision-1`, `decision-5`):
- Dispatch `.conda` uploads across arbitrary `[[targets]]` declared in `universe.toml` (zero hardcoded owner channel defaults):
  - `prefix-dev`: `rattler_upload` / OIDC GitHub Actions ambient token exchange + `--generate-attestation` support, or `PREFIX_API_KEY`.
  - `anaconda`, `s3`, `quetz`, `artifactory`, `filesystem`: `rattler_upload` + `rattler_index` local filesystem indexing.
  - `github-pages`: idempotent creation/lookup of rolling `<release_tag_prefix>-<subdir>` GitHub Releases (`packages-linux-64`, `packages-win-64`, `packages-osx-64`, `packages-osx-arm64`, `packages-linux-aarch64`, `packages-noarch`) via typed `reqwest` GitHub REST v3 client, asset upload with SHA-256 skip-if-identical idempotency, configurable `max_builds_per_version` retention pruning (protecting against GitHub's 1,000-asset release cap), and atomic `.conda.json` fragment commit to `index_branch` (`universe-index`).
- Support `--dry-run` printing every planned upload and pruning action without mutating remote state.

The adapter request/result boundary — capabilities, the plan/build/audit requests, the six
outcomes, and produced artifacts — lives in `crates/universe-core/src/adapter.rs` (`UNI-31`).
This task consumes those records instead of defining a per-command shape of its own.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 `wiremock` tests verify arbitrary `prefix.dev` URLs/channels, S3 endpoints, Anaconda owners, and GitHub Releases API interactions (`create release`, `list assets`, idempotent `upload asset`, retention pruning).
- [ ] #2 Re-running `publish-artifacts` on an already-uploaded `.conda` with identical `sha256` succeeds idempotently without failing or corrupting release assets; mismatched `sha256` for an existing filename is refused unless `--force` is set.
- [ ] #3 `--dry-run` performs zero mutating HTTP requests (verified via `wiremock` request expectations).
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 `rstest-bdd` (`publish.feature`) and `wiremock` integration suites pass under `cargo nextest`.
<!-- DOD:END -->
