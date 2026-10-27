---
id: UNI-26
title: Restore portal-bundle release stage and cut v0.2.0 with the embedded browser portal
status: To Do
assignee: []
created_date: '2026-10-27 13:40'
updated_date: '2026-10-27 13:40'
labels:
  - release
  - portal
  - ci
milestone: m-6
dependencies:
  - UNI-16
  - UNI-25
documentation:
  - backlog/docs/architecture/doc-1 - Monorepo-and-toolchain-architecture.md
  - backlog/docs/roadmap/doc-4 - Milestone-roadmap-and-skills-sh-rollout.md
  - backlog/decisions/decision-8 - Owner-lane-vs-consumer-lane-GitHub-Actions-workflows-and-zero-checkout-OIDC-publishing.md
  - backlog/decisions/decision-10 - Adopt-ODM-conda-release-governance-and-supply-chain-hardening-patterns.md
  - backlog/decisions/decision-11 - Headless-v0.1.0-first,-browser-portal-deferred-to-v0.2.0.md
priority: high
type: feature
ordinal: 26000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Reverse the headless packaging concessions made for `v0.1.0` (decision-11) and ship the
browser portal as `v0.2.0`:
- Delete the committed placeholder directory `crates/universe-assets/placeholder/` and its
  fallback branch in `build.rs` once the generated `@universe/portal` SSG output is a
  required build input, updating `tests/scripts/portal-placeholder.test.ts` (or its successor)
  to assert the placeholder no longer ships. `packages/portal/build/` stays git-ignored
  throughout; no ignore-rule changes are involved.
- Restore the `portal-bundle` job in `release.yml` to run
  `bun run --filter '@universe/portal' build` once on `ubuntu-latest`, upload the result
  as the `portal-dist` artifact, and feed it to all four `rattler-build` platform jobs via
  `UNIVERSE_KIT_PORTAL_DIST`; the headless `v0.1.0` flow relies on the in-crate placeholder
  and must never run the SvelteKit toolchain.
- Re-wire the `ci.yml` `package` job and `crates/universe-kit/recipe/recipe.yaml` source
  paths (the v0.1.0 recipe points at the placeholder directory per `UNI-15`) back to the real
  portal tree, including `assets/` and per-package routes, and verify identical embedded
  portal bytes across all four platform packages.
- Cut `v0.2.0` through the existing autorelease → release flow behind the `UNI-20` fail-closed
  guard and isolated publisher, with OIDC attestations, GitHub Release binaries, and an
  external install smoke test, then verify the live GitHub Pages catalog explorer.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 No headless-placeholder content remains in the `universe-assets` embed of any released `v0.2.0` package; a repository test distinguishes generated SSG output from the placeholder and fails if the placeholder is shipped.
- [ ] #2 The restored `portal-bundle` job builds the real SSG and the four `.conda` packages embed byte-identical portal trees (hash comparison across `linux-64`, `win-64`, `osx-64`, `osx-arm64`).
- [ ] #3 `v0.2.0` is live on `prefix.dev/archont561/archont561` with attestations and a GitHub Release, and the GitHub Pages URL serves the working catalog search, install snippets, and binary audit drawers (the portal acceptance criteria relocated from `UNI-18`).
- [ ] #4 The already-published `v0.1.0` channel stays installable and its `repodata`/shards are not mutated by the `v0.2.0` publication.
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 README, doc-1, and doc-3 are updated to retire the headless-only caveats, and decision-11 records the completed `v0.2.0` follow-up with the release identifiers.
<!-- DOD:END -->
