---
id: UNI-17
title: Author and publish public skills/universe-kit/SKILL.md package to skills.sh post-v0.1.0
status: To Do
assignee: []
created_date: '2026-10-26 20:15'
updated_date: '2026-10-26 20:15'
labels:
  - skills
  - release
milestone: m-5
dependencies:
  - UNI-16
documentation:
  - backlog/docs/roadmap/doc-4 - Milestone-roadmap-and-skills-sh-rollout.md
  - backlog/decisions/decision-9 - Dual-tier-Agent-Skills-strategy-and-post-v0.1.0-skills.sh-publication-gate.md
priority: medium
type: feature
ordinal: 17000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Strictly after `UNI-16` (`v0.1.0` live on `prefix.dev/archont561/archont561`), expose the public end-user Agent Skill for `skills.sh` discovery (`decision-9`, `doc-4`):
- Create `skills/universe-kit/SKILL.md` (<500 lines progressive disclosure) + `references/{universe-toml,recipe-patterns,publishing-targets,troubleshooting-audit}.md` + root `skills.sh.json`.
- Cover bootstrap (`universe-kit init`), legacy `<TOOL>-conda` migration, `recipe.yaml` v1 + multi-output `staging` recipes, arbitrary channel targets (`prefix.dev` OIDC, GitHub Pages CEP-15/16 + GitHub Releases, S3, Anaconda), and `goblin` RPATH/header audit remediation.
- Embed `skills/universe-kit/` into `crates/universe-assets` so `universe-kit init --with-skill` scaffolds `.agents/skills/universe-kit/` offline.
- Enable `skills-ref validate skills/universe-kit` in `pixi run ci` and `metadata.version` synchronization in `pixi run repo-check` + `autorelease.yml`.
- Trigger and verify `skills.sh` indexing via `npx skills add Archont561/universe`.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 `skills-ref validate skills/universe-kit` passes in `pixi run ci` and `pixi run repo-check` enforces `metadata.version` parity with `Cargo.toml`.
- [ ] #2 `universe-kit init --with-skill` writes byte-identical `.agents/skills/universe-kit/` files offline.
- [ ] #3 `npx skills add Archont561/universe --list` discovers `universe-kit` cleanly and installs into Claude Code / Cursor / Codex agent directories.
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 Public `README.md` documents `npx skills add Archont561/universe` alongside `pixi global install`.
<!-- DOD:END -->
