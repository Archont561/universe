---
id: UNI-54
title: Implement the Universe release controller
status: To Do
assignee: []
created_date: '2026-10-31'
updated_date: '2026-10-31'
labels:
  - release
  - cli
  - architecture
milestone: m-4
dependencies:
  - UNI-20
  - UNI-41
documentation:
  - backlog/decisions/decision-126 - Release-control-lives-in-Universe-with-attempts-promotion-and-a-resumable-journal.md
  - backlog/decisions/decision-123 - Convco-is-a-contributor-linter-not-the-release-authority.md
  - backlog/decisions/decision-125 - Two-setup-actions-with-a-one-line-mode-difference.md
priority: high
type: feature
ordinal: 54000
---

## Description
<!-- SECTION:DESCRIPTION.BEGIN -->
Own the release lifecycle in this workspace, as a library crate plus a `universe
release` verb group. The semantics are fixed by `decision-126`: recorded gate evidence,
a prepared-and-cut sequence with a dry-run that differs only in writes, a journal that
makes publication resumable rather than repeatable, mandatory destinations, and
artifact-scoped verification.

Two behaviours deserve emphasis because they are what make the lane trustworthy:

- **Attempts and promotion.** A prerelease attempt builds from a commit, creates a
  **draft** release, and uploads assets; a draft creates no tag, so a discarded attempt
  leaves nothing behind. Promotion publishes the draft — the moment the tag exists —
  and fans out without rebuilding. Bytes are identical; only pointers change.
- **Journal and resume.** Every destination records its outcome keyed by release and
  destination. A failed publish is repaired by re-running the failed jobs, and a second
  publish of the same key is an error rather than an overwrite.

Version computation follows `decision-123` and the policy in `decision-126`: the version
is a declared literal promoted by a human, parity is asserted across manifests, recipes,
and the changelog heading, and a release that lacks a version section refuses to
publish. No step invokes an external versioning tool, and no step rewrites version
literals automatically.
<!-- SECTION:DESCRIPTION.END -->

## Acceptance Criteria
<!-- AC.BEGIN -->
- [ ] #1 `release plan` refuses without recorded gate evidence for the commit being released, and records the controller identity as either a dev commit or a released version plus asset digest.
- [ ] #2 The state machine (`plan`, `prepare`, `cut`, `classify`, `build`, `publish`, `resume`, `promote`) is covered by fixtures, and a dry run differs from a real run only in writes.
- [ ] #3 A killed publish resumes to completion without republishing a destination already recorded in the journal; a repeated publish of a recorded destination fails.
- [ ] #4 Promotion asserts the attempt's digests, publishes the draft, and completes without rebuilding any artifact.
- [ ] #5 Declared scopes are parsed by longest prefix, an unmatched tag is a lint error, and a scoped release narrows both the package set and the destination set.
- [ ] #6 Version parity is enforced across `Cargo.toml`, `universe.toml`, every recipe's `context.version`, and the changelog section for the version.
<!-- AC.END -->

## Definition of Done
<!-- DOD.BEGIN -->
- [ ] #1 Universe's own first release is cut through these verbs, and the previous controller is archived only after that release proves the parity list above.
- [ ] #2 No Backlog record or source file references the retired controller's repository.
- [ ] #3 `pixi run gates` passes.
<!-- DOD.END -->
