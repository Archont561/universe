---
id: UNI-18
title: Dogfood a multi-package Conda universe with Universe Kit for CloudCompare and ODM
status: To Do
assignee: []
created_date: '2026-10-26 20:15'
updated_date: '2026-10-27 13:40'
labels:
  - channel
  - portal
  - release
milestone: m-5
dependencies:
  - UNI-16
documentation:
  - backlog/docs/architecture/doc-2 - Universe Kit-CLI-and-manifest-specification.md
  - backlog/docs/architecture/doc-3 - Static-channel-and-SvelteKit-UnoCSS-portal-specification.md
  - backlog/docs/roadmap/doc-4 - Milestone-roadmap-and-skills-sh-rollout.md
  - backlog/decisions/decision-11 - Headless-v0.1.0-first,-browser-portal-deferred-to-v0.2.0.md
priority: medium
type: task
ordinal: 18000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Prove `universe-kit` end-to-end against the real-world geospatial/3D C++ workload that motivated it (`doc-2`, `doc-3`, `doc-4`):
- Scaffold a multi-package universe repository with `universe-kit init --pages --with-skill`.
- Migrate `CloudCompare` (`recipes/cloudcompare/{recipe.yaml,package.toml}`, replacing all `CloudCompare-conda/scripts/*.py` and custom workflows) and onboard a second multi-platform recipe (`ODM` / geospatial C++ tool) in the same repository.
- Verify DAG planning (`universe-kit plan --changed-or-missing`), 4-platform builds, isolated package tests, pure-Rust `goblin` binary/RPATH audits, OIDC `prefix.dev` publishing, and headless GitHub Pages CEP-15/CEP-16 static channel deployment. Per decision-11 the SvelteKit/UnoCSS documentation portal does not exist yet at this point; its live deployment and browser verification are covered by `UNI-26` in M6.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The multi-package consumer universe contains zero custom Python/Bash CI scripts—only `universe.toml`, `pixi.toml`, `recipes/*`, and `# @generated` workflows.
- [ ] #2 Both packages build, pass `universe-kit audit`, and publish cleanly to `prefix.dev` and the repository's GitHub Pages CEP-15 static channel + rolling `packages-<subdir>` GitHub Releases.
- [ ] #3 `pixi install` resolves and installs packages directly from the live headless GitHub Pages URL (`https://<owner>.github.io/<universe-repo>`) using only `repodata`/shards/`channeldata`; browser portal rendering (catalog search, install snippets, binary audit drawers) is accepted separately under `UNI-26` once M6 ships.
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 Any ergonomics gaps discovered during migration are captured as follow-up `UNI-*` tasks in `backlog/`.
<!-- DOD:END -->
