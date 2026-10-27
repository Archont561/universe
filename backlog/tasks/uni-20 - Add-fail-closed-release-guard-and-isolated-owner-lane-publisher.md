---
id: UNI-20
title: Add fail-closed release guard and isolated owner-lane publisher
status: To Do
assignee: []
created_date: '2026-10-27 09:20'
updated_date: '2026-10-27 13:40'
labels:
  - release
  - ci
milestone: m-4
dependencies:
  - UNI-11
  - UNI-15
documentation:
  - backlog/decisions/decision-8 - Owner-lane-vs-consumer-lane-GitHub-Actions-workflows-and-zero-checkout-OIDC-publishing.md
  - backlog/decisions/decision-10 - Adopt-ODM-conda-release-governance-and-supply-chain-hardening-patterns.md
  - backlog/decisions/decision-11 - Headless-v0.1.0-first,-browser-portal-deferred-to-v0.2.0.md
  - backlog/decisions/decision-1 - Two-tier-product-and-channel-boundary-(archont561-archont561-ships-universe-kit;-consumer-universes-target-arbitrary-channels).md
priority: high
type: feature
ordinal: 20000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
`Archont561/ODM-conda` demonstrates a release pipeline that fails closed until a human flips explicit acknowledgements, and keeps the credentialed job unable to execute repository code. decision-8 records this as the target for Universe, but no application `release.yml` exists yet. Implement that target when application release automation is authorized:
- **Rust release guard** (a `universe-core` release module plus thin CLI surface, replacing any ad-hoc shell): require the release tag to point at the exact trusted default-branch HEAD, require a clean checkout with no untracked files, verify repository identity against typed metadata, and validate the tag/version/build-number scheme against the recipe and `repo-check` data. Block the release while any explicit human acknowledgement in a reviewed manifest is `false` (the ODM set `publishable`, `license_reviewed`, `source_completeness_reviewed`, `native_validation_passed` is the starting point; adapt names to universe-kit and keep every default `false`).
- **Isolated publisher job**: the `id-token: write` job must not check out or execute repository code. It downloads the already-built artifact set, verifies exact package/source counts and `SHA256SUMS --strict`, uploads corresponding source and audit files to the GitHub Release *before* any binary is uploaded to `prefix.dev`, then runs the pinned `rattler-build upload prefix --generate-attestation`, and only attaches the binary release asset after a successful channel upload. Narrow permissions per job; keep the protected `prefix` GitHub Environment.
- **Release event, not bare tag**: publishing runs from a published, non-draft, non-prerelease GitHub Release; tags stay internal build markers. No automatic version bump is inferred from Conventional Commits for release content (autorelease keeps its observe-mode role from decision-8).
- A failed upload must never leave a channel binary without its source bundle, and filename collisions fail loudly instead of overwriting.
- Per decision-11, the first guarded release (`v0.1.0`) is headless: the artifact set contains the committed placeholder portal dist rather than generated SSG output. The guard and publisher job boundaries are unchanged; browser portal assets re-enter the same ordering rules in `UNI-26` for `v0.2.0`.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 With every acknowledgement `false`, the guard exits non-zero on an attempted release; flipping each gate is independently tested, and a dirty tree, tag/HEAD mismatch, unknown tag scheme, or wrong repository identity each produce a distinct error (unit tests in a temp Git repo, mirroring ODM's `test_git_guard.py`).
- [ ] #2 The credentialed publishing job contains no `actions/checkout` and no invocation of repository scripts; actionlint plus a structural workflow test assert this, and the job's permissions contain only `contents: write` + `id-token: write` scoped to the protected environment.
- [ ] #3 A wiremock/integration run proves ordering: source bundle and audit assets are uploaded before the prefix.dev upload call, and a prefix upload failure prevents the binary GitHub Release attachment while leaving no partial channel state.
- [ ] #4 `release.yml` triggers only on the published-release event (plus the existing manual repair path); a pushed tag without a release performs no publishing, verified by workflow tests/actionlint.
- [ ] #5 The guard is reusable by consumer-lane generated workflows (UNI-13) so downstream universes get the same acknowledgement and clean-HEAD discipline without copying code.
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 The acknowledgement manifest, guard CLI, and workflow changes ship with rstest-bdd coverage and an ADR update (decision-8/decision-10) recording the final job boundary and the reason for the publisher-shell exception.
<!-- DOD:END -->
