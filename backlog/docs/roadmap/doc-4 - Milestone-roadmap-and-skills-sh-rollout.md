---
id: doc-4
title: Milestone roadmap and skills.sh rollout
type: roadmap
created_date: '2026-10-26 20:15'
updated_date: '2026-10-27 13:40'
tags:
  - roadmap
  - milestones
  - skills
  - release
  - approved
---

# Milestone roadmap and skills.sh rollout

## Status

Approved delivery plan sequencing `universe-kit` from empty repository bootstrap (`M0`)
through the first public Conda release — the headless `v0.1.0` (`M4`) — and post-release
`skills.sh` ecosystem rollout plus multi-tool dogfooding (`M5`).

Re-visioned 2026-10-27 by decision-11: the SvelteKit/UnoCSS browser portal originally scoped
in M1 is deferred to a new milestone M6 and ships as the `v0.2.0` minor release. `v0.1.0`
ships headless: solver-served CEP-15/CEP-16 channels without browser HTML, with a committed
offline placeholder keeping the `universe-assets` embed intact. decision-5, decision-6, and
doc-3 remain approved specifications; only their delivery order changed.

## Milestone overview

| Milestone | Scope | Key Tasks | Exit Gate |
| :--- | :--- | :--- | :--- |
| **M0** | Reproducible Pixi/Rust/Python monorepo foundation | `UNI-28`, `UNI-36` | `pixi install --locked` + `pixi run gates` pass from a clean checkout; `.agents/skills/` use local content hashes; owner CI passes `actionlint`; the offline developer-transport boundary is proven or explicitly accepted. |
| **M1** | Schemas and adapter contracts (headless) | `UNI-30`, `UNI-31` | Neutral Rust schemas and adapter conformance fixtures are tested while `universe-core` has no ecosystem/UI/network dependencies. Portal tasks `UNI-5`/`UNI-6` moved to M6 (decision-11). |
| **M2** | Core CLI: lint, DAG planner, build/test, `goblin` auditor | `UNI-7`, `UNI-8`, `UNI-9`, `UNI-10`, `UNI-21`, `UNI-22`, `UNI-23` | `universe-kit lint`, `plan`, `build`, `test`, and `audit` pass `rstest`, `rstest-bdd`, and `proptest` suites and build + audit multi-output fixtures end-to-end, including payload-leak, archive-hardening, and installed-prefix functional checks. |
| **M3** | Multi-target publishing, headless CEP-15/16 channel, consumer `init` | `UNI-11`, `UNI-12`, `UNI-13` | `publish-artifacts` (`prefix.dev`/S3/Anaconda/GH Releases), headless `pages build` (CEP-15 + CEP-16 solver data, no HTML), and `init`/`add` pass real `pixi install` resolution against a local HTTP static channel. |
| **M4** | Upstream release watcher & headless `v0.1.0` owner release | `UNI-14`, `UNI-15`, `UNI-16`, `UNI-20` | `universe-kit watch` passes `wiremock` BDD tests; 4-platform `.conda` packages embedding the in-crate placeholder portal dist ship to `prefix.dev/archont561/archont561` with OIDC attestations + GitHub Release binaries behind a fail-closed release guard and isolated publisher. |
| **M5** | Post-`v0.1.0` public `skills.sh` skill & universe dogfooding | `UNI-17`, `UNI-18` | `skills/universe-kit/SKILL.md` validated via `skills-ref` & indexed on `skills.sh`; live multi-package universe (`CloudCompare` + `ODM` / GIS tools) replaces per-tool `-conda` repos and resolves through the headless channel. |
| **M6** | Browser portal `v0.2.0`: SvelteKit SSG, `lol_html` hydration, embedded docs (decision-11) | `UNI-5`, `UNI-6`, `UNI-25`, `UNI-26` | `pages build` emits the full doc-3 tree with hydrated catalog and per-package pages; portal passes unit/PBT/snap + non-root `/subpath/` Playwright E2E under `networkGuard`; `v0.2.0` is released with the real embedded portal and a live GitHub Pages explorer while `v0.1.0` stays installable. |

## Dependency flow across tasks

```text
UNI-28 (Standalone Pixi/Rust/Python baseline)
  ├──► UNI-36 (Offline developer transport decision)
  └──► UNI-30 (Authoritative neutral schemas)
         └──► UNI-31 (Adapter boundary + conformance fixtures)
                ├──► UNI-7 (Recipe & manifest linter: marked-yaml, spdx, SHA/rev rules)
                ├──► UNI-8 (DAG planner + remote repodata diffing -> GHA matrix JSON)
                ├──► UNI-9 (rattler-build runner + native Windows MSVC discovery + isolated test)
                └──► UNI-10 (Streaming .conda + pure-Rust goblin ELF/PE/Mach-O auditor + fragments)
                       ├──► UNI-11 (Multi-target publisher: prefix.dev OIDC, S3, Anaconda, GH Releases)
                       └──► UNI-12 (Headless CEP-15/16 channel indexer: pages build, solver data only)
                              └──► UNI-13 (Consumer init, add, and zero-checkout OIDC workflows)
                                     ├──► UNI-14 (Upstream release watcher: GitHub, PyPI, crates.io)
                                     └──► UNI-15 (4-platform recipe.yaml + headless release.yml staging)
                                            └──► UNI-16 (Cut headless v0.1.0 to prefix.dev/archont561/archont561)
                                                   ├──► UNI-17 (Public skills/universe-kit/SKILL.md -> skills.sh)
                                                   └──► UNI-18 (Dogfood multi-package universe, headless channels)
```

### M6 browser-portal track (decision-11)

The portal track branches from the `UNI-30`/`UNI-31` schema and adapter contracts and runs
independently of the CLI work; it rejoins the release flow at `v0.2.0` after the headless release:

```text
UNI-30 + UNI-31 ──► UNI-5 (SvelteKit 3 + Svelte 5 + UnoCSS @universe/portal)
              └──► UNI-6 (bun-test-utils unit/PBT/E2E + real crates/universe-assets embed)
UNI-6 + UNI-12 ──► UNI-25 (lol_html hydration, catalog.json/package.json in pages build)
UNI-16 + UNI-25 ──► UNI-26 (Restore portal-bundle stage; cut portal v0.2.0)
```

### Cross-repository hardening tasks (decision-10)

The hardening controls from decision-10 attach to existing tasks rather than
forming a parallel track:

```text
UNI-28 ──► UNI-36 (offline developer transport decision)
UNI-9  ──► UNI-23 (installed-prefix functional tests + corresponding-source bundles)
UNI-10 ──► UNI-21 (artifact identity + payload-leak audit rules)
        └─► UNI-22 (archive extraction caps + HTTPS/redirect source-fetch hardening)
UNI-11, UNI-15 ──► UNI-20 (fail-closed release guard + isolated no-checkout publisher)
```

## Post-`v0.1.0` public Agent Skill rollout (`decision-9`)

Per explicit project constraint, the public consumer skill (`skills/universe-kit/SKILL.md` + optional root `skills.sh.json`) MUST NOT be exposed on `main` for `skills.sh` indexing until `UNI-16` has published the headless `universe-kit v0.1.0` to `https://prefix.dev/archont561/archont561`. The decision-11 re-vision does not change this gate: the skill describes headless-first workflows and gains portal documentation in M6.

Once `v0.1.0` is live:
1. Land `skills/universe-kit/SKILL.md` + `references/{universe-toml,recipe-patterns,publishing-targets,troubleshooting-audit}.md` + `skills.sh.json` (`UNI-17`).
2. Enforce `skills-ref validate skills/universe-kit` and `metadata.version` synchronization through named Pixi tasks backed by Python repository checks.
3. Embed `skills/universe-kit/` in `crates/universe-assets` so `universe-kit init --with-skill` works offline.
4. Trigger initial `skills.sh` indexing via `npx skills add Archont561/universe --list` and verify installation across Claude Code / Cursor / Codex (`npx skills add Archont561/universe`).

## Re-vision history

- **2026-10-27 — decision-11 (headless-first split).** Portal work (`UNI-5`, `UNI-6`) moved
  from M1 to a new M6; `UNI-12` rescoped to the headless channel indexer and its `lol_html`
  half split into `UNI-25`; `UNI-26` added to restore the real `portal-bundle` stage and cut
  `v0.2.0`; `UNI-15`/`UNI-18`/`UNI-20` updated for placeholder-based headless staging. A
  committed placeholder in `crates/universe-assets/placeholder/`, selected by a `build.rs`
  fallback (built SSG output always wins), keeps the `include_dir!` embed compiling without
  touching the git-ignored SSG output directory, the recipe, or the workflow topology.
