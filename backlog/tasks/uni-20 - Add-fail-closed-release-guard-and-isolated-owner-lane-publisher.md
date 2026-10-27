---
id: UNI-20
title: Add fail-closed release guard and isolated owner-lane publisher
status: To Do
assignee: []
created_date: '2026-10-27 09:20'
updated_date: '2026-10-27'
labels:
  - release
  - ci
milestone: m-4
dependencies:
  - UNI-11
  - UNI-15
  - UNI-34
  - UNI-41
documentation:
  - backlog/docs/architecture/doc-111 - V1-package-layout-and-offline-readiness.md
  - backlog/decisions/decision-8 - Owner-lane-vs-consumer-lane-GitHub-Actions-workflows-and-zero-checkout-OIDC-publishing.md
  - backlog/decisions/decision-10 - Adopt-ODM-conda-release-governance-and-supply-chain-hardening-patterns.md
  - backlog/decisions/decision-119 - Full-multi-ecosystem-v1-and-static-catalog-distribution.md
priority: high
type: feature
ordinal: 20000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Implement the fail-closed owner release guard and isolated publisher boundary. Require the
release tag to identify the exact trusted default-branch HEAD, a clean checkout, repository
identity, version/build policy, reviewed acknowledgements, expected artifact set, manifest,
and strict checksums. Human acknowledgements default to false and each gate has an actionable
diagnostic.

The credentialed publisher must not check out or execute repository code. It downloads the
already-built artifacts, verifies source/audit bundles and digests, then publishes through
separate target-specific lanes. Conda channel package files and repodata are published to the
standard static paths; Cargo/Python/npm registries publish their native metadata/assets; the
verified portal/catalog snapshot is attached/published from the same release identity. OIDC
permissions and protected environments are narrowly scoped. A failed publish must not advance
a stable catalog reference or leave an advertised partial artifact set.

Publishing runs from an authorized non-draft release event (plus explicit repair path), not a
bare tag. The former headless `v0.1.0` placeholder / portal `v0.2.0` sequencing is superseded;
placeholder assets are never accepted as v1 release content.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Each acknowledgement false, dirty tree, tag/HEAD mismatch, unknown tag scheme, wrong repo, missing artifact, mutable pin, or checksum mismatch blocks release with a distinct tested diagnostic.
- [ ] #2 Credentialed jobs contain no checkout/repository script execution; workflow tests and actionlint verify minimal permissions and protected environments.
- [ ] #3 Integration tests prove target-specific publication ordering; failure prevents catalog advancement and leaves no advertised partial channel/registry state.
- [ ] #4 Conda package bytes are published at the standard static channel paths, not only as GitHub Release links; the portal/catalog digest matches the verified manifest.
- [ ] #5 Release event restrictions and explicit repair behavior are tested; tag-only events cannot publish.
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 Guard, acknowledgement manifest, and isolated workflow pass unit/BDD/structural tests and are documented in the release design record.
<!-- DOD:END -->
