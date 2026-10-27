---
id: decision-9
title: Dual-tier Agent Skills strategy and post-v1 skills.sh publication gate
date: '2026-10-26 20:15'
status: accepted
---

> **Scope amendment:** the public skill gate is now after the integrated v1 release, not a Conda-only `v0.1.0`. CLI/install instructions must match the published v1 interface; the contributor-skill policy is unchanged.

## Context

AI coding agents work on `universe-kit` in two completely different roles:
1. **Contributors building `Archont561/universe`** (working on Rust crates, Svelte 5 + UnoCSS SSG, `Backlog.md` tasks, TDD/BDD specs, and CEP-15/16 channel indexing).
2. **End users operating consumer universes** (asking an agent to initialize a Conda universe, migrate a legacy `<TOOL>-conda` repo, author a `recipe.yaml` v1 recipe, configure OIDC `prefix.dev` or GitHub Pages CEP-15 channels, or debug a `goblin` RPATH audit failure).

`skills.sh` indexes public GitHub repositories containing top-level `skills/<skill-name>/SKILL.md` automatically when anyone runs `npx skills add <owner/repo>`. If `skills/universe-kit/SKILL.md` were exposed before the integrated Universe v1 was published and its CLI/install instructions stabilized, external agents installing the skill could execute non-existent install commands or hallucinate pre-release flags.

## Decision

1. **Internal contributor skills (`.agents/skills/` + `skills-lock.json`) — Active from M0**:
   - Vendor shared `Archont561/skills` (`archont561` dispatcher, `backlog`, `session`, `tdd`, `bdd`, `clean-code-principles`, `refactor`, `audit`, `grill-me`) and `prefix-dev` skills (`agent-skill-rattler-build`, `agent-skill-conda-forge`) into `.agents/skills/` tracked by `skills-lock.json`.
   - Author two concise repo-local contributor skills under `.agents/skills/`: `svelte5-unocss-ssg` (enforcing relative paths, UnoCSS offline icons, and template-route cloning invariants) and `cep-channel` (enforcing CEP-15 `repodata_version: 2`, per-subdir `base_url`, and CEP-16 shard invariants).
   - `.agents/skills/` is ignored by `skills.sh`'s public scanner unless explicitly requested, keeping contributor plumbing private to the repo.
2. **Public consumer skill (`skills/universe-kit/SKILL.md` + `skills.sh.json`) — Gated until after integrated v1 ships (M5)**:
   - Author the end-user skill at `skills/universe-kit/` (`SKILL.md`, references, and `skills.sh.json`) only after the integrated v1 is live and its supported install commands work (or keep it unindexed until M5).
   - Enforce `Agent Skills` specification compliance in CI (`pixi run ci`) via `skills-ref validate skills/universe-kit` (installed from `conda-forge`) and enforce `metadata.version` synchronization in `scripts/repo_check.py`.
   - Embed `skills/universe-kit/` inside `crates/universe-assets` so `universe-kit init --with-skill` scaffolds `.agents/skills/universe-kit/` offline in consumer repositories alongside `npx skills add Archont561/universe`.

## Consequences

- Contributor agents have full `Archont561`, `rattler-build`, Svelte 5 SSG, and CEP-15/16 guardrails starting from the standalone baseline (`UNI-28`, M0).
- External users discovering `Archont561/universe` on `skills.sh` after integrated v1 receive a verified, `skills-ref`-validated skill whose exact CLI commands and installation instructions work immediately.
