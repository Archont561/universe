---
id: doc-4
title: Milestone roadmap and skills.sh rollout
type: roadmap
created_date: '2026-10-26 20:15'
updated_date: '2026-10-27'
tags:
  - roadmap
  - milestones
  - skills
  - release
  - approved
---

# Milestone roadmap and skills.sh rollout

## Status

The accepted first stable release is an integrated Universe v1, not a sequence of a Conda-only `v0.1.0` followed by a portal `v0.2.0`. Decision-119 sets the v1 scope: Conda and standard static Conda publishing, Rust crates, Python wheels, npm packages, Airlock consumption, and a browsable portal generated from verified/pinned GitHub Release data. The portal is a catalog/discovery surface, not the registry. Kit invokes configured external build/test commands and handles collection, validation, indexing, and publication; it does not implement package builders.

This is the target plan, not a claim that those components are already implemented. Existing task IDs/statuses remain authoritative. Milestones describe product/theme outcomes; dependencies are only real blockers, with no sprint-task parent/child hierarchy.

## Milestone overview

| Milestone | Outcome | Key tasks | Exit gate |
| :--- | :--- | :--- | :--- |
| **M0** | Reproducible Pixi/Rust/Python monorepo foundation | `UNI-28`, `UNI-36` | Locked environment and current offline developer-transport boundary are verified from a clean checkout; owner CI/actionlint pass. |
| **M1** | Neutral schemas and ecosystem adapter contracts | `UNI-30`, `UNI-31`, `UNI-41` | Core remains free of ecosystem/UI/network behavior; independently testable adapter conformance fixtures establish the contract. |
| **M2** | Kit command foundations, artifact orchestration, testing and audit | `UNI-7`–`UNI-10`, `UNI-21`–`UNI-23` | Kit validates and plans; configured external commands produce fixture artifacts; Kit collects/verifies/audits them without embedding a builder. Security, archive, and installed-prefix checks pass. |
| **M3** | Artifact publication, native Conda static channel and consumer initialization | `UNI-11`–`UNI-13` | Standard Conda channel/subdirectory paths and package files resolve through a local static HTTP fixture; publication targets and consumer init/add flows are tested. |
| **M4** | Upstream release intake and trusted producer/release lanes | `UNI-14`, `UNI-15`, `UNI-20`, `UNI-34` | Pinned release inputs, fail-closed guards, isolated publishing, platform recipes, and release provenance work. This milestone is not itself the integrated stable v1 release. |
| **M5** | Post-v1 public Agent Skill and multi-tool dogfooding | `UNI-17`, `UNI-18` | The public skill is published only after the integrated v1 release gate; real multi-package usage is documented and tested. |
| **M6** | Browsable portal built from the pinned catalog and embedded assets | `UNI-5`, `UNI-6`, `UNI-25`, `UNI-26`, `UNI-46` | Static portal has catalog/package pages, embedded docs, hydration as specified, subpath/browser tests, and deterministic snapshot inputs. It joins the v1 release; it is not a separate `v0.2.0` product release. |
| **M8** | Airlock consumer and separate bootstrap | `UNI-32`, `UNI-33`, `UNI-38`, `UNI-45` | Sealed snapshot/verify/restore and scoped native-client execution work; bootstrap remains a distinct artifact; no universal HTTP interception or network fallback. |
| **M9** | First-class native package adapters beyond Conda | `UNI-42`–`UNI-44` | Cargo sparse index/crates, Python wheel index, and npm registry fixtures pass protocol and integrity tests; all remain behind neutral core contracts. |
| **M10** | Integrated Universe v1 release | `UNI-16`, `UNI-40`–`UNI-47` | Conda, Cargo, Python wheels, npm, Airlock consumption, the browsable portal, and the standard static Conda channel pass the end-to-end evidence gate and are published as one v1 product. |

## Real dependency flow

```text
UNI-28 ──► UNI-30 ──► UNI-31 ──► UNI-7 / UNI-8 / UNI-10
                                  └──► UNI-9 (external-command orchestration; no bundled builder)
UNI-10 ──► UNI-11 ──► UNI-12 ──► UNI-13
UNI-13 ──► UNI-14 / UNI-15
UNI-11 + UNI-15 ──► UNI-20
UNI-34 + UNI-38 ──► UNI-40 (thin `universe` CLI/umbrella package)
UNI-30 + UNI-31 ──► UNI-41 (manifest/release schema)
UNI-31 + UNI-41 ──► UNI-42 (Cargo) / UNI-43 (Python wheels) / UNI-44 (npm)
UNI-12 + UNI-32 + UNI-33 + UNI-41..44 ──► UNI-45 (Airlock native views and scoped execution)
UNI-5 + UNI-6 + UNI-12 + UNI-25 + UNI-41..44 ──► UNI-46 (release-driven portal)
UNI-12 + UNI-32..33 + UNI-38 + UNI-40..46 ──► UNI-47 (offline matrix and independent evidence review)
UNI-47 ──► UNI-16 (integrated v1 release)
```

The diagram indicates prerequisite relationships only. It does not create parent/child containment or require work to be completed in a single sprint. Full task-level dependencies remain on the individual backlog items.

## Post-v1 public Agent Skill rollout (`decision-9`)

The public consumer skill (`skills/universe-kit/SKILL.md` and optional root `skills.sh.json`) MUST NOT be exposed for `skills.sh` indexing until `UNI-16` has published the integrated Universe v1 and its supported producer/consumer/discovery surfaces. The former headless-`v0.1.0` prerequisite is superseded.

After v1:
1. Land the skill, references, and `skills.sh.json` (`UNI-17`).
2. Enforce `skills-ref validate` and metadata/version synchronization through named Pixi tasks and repository checks.
3. Embed `skills/universe-kit/` in `crates/universe-assets` so `universe-kit init --with-skill` works offline.
4. Verify indexing and installation across supported coding agents; then dogfood multi-package Conda workflows (`UNI-18`).

## Superseded sequencing

Decision-11's headless `v0.1.0` / portal `v0.2.0` release split is superseded by decision-119. Portal implementation remains in M6 and the static Conda indexer remains in M3, but both are required by M10. Generic assets, Agent Skills, and STAC remain deferred from v1. Decision-13's neutral core and adapter separation remains valid; its earlier adapter budget and sequencing do not.
