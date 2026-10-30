---
id: doc-4
title: Ordered milestone roadmap and skills.sh rollout
type: roadmap
created_date: '2026-10-26 20:15'
updated_date: '2026-10-31'
tags:
  - roadmap
  - milestones
  - sprints
  - skills
  - release
  - approved
---

# Ordered milestone roadmap and skills.sh rollout

## Status

Decision-119 defines the integrated v1: Conda and a usable static Conda channel, Cargo crates,
Python wheels, npm packages, Airlock consumption, a browsable portal, and verified/pinned
GitHub Release/catalog data. The former headless `v0.1.0` / portal `v0.2.0` split is superseded.
Kit invokes configured external build/test commands; it does not implement package builders.

This roadmap now reconciles milestone outcomes, task assignments, true blocker edges, and a
proposed undated sprint train. The detailed graph and sprint mapping are in
[doc-112](doc-112%20-%20Ordered-milestone-DAG-and-sprint-train.md). Milestones are not task
parents; only task `dependencies` create blockers. The sprint train is a scheduling overlay,
not a new Backlog record type or date commitment.

## Ordered milestones

| Order | Milestone outcome | Tasks | Exit gate |
|---:|---|---|---|
| **M0** | Reproducible developer foundation | `UNI-28`, `UNI-36`, `UNI-39`, `UNI-49`, `UNI-50` | **Complete.** Baseline/backlog foundations, offline gates and restore, the offline audit gate, and the declarative repository-lint layer are all verified. |
| **M1** | Neutral schemas and adapter contracts | `UNI-30`, `UNI-31`, `UNI-41` | Neutral schemas, adapter contract, project and pinned release manifests tested. |
| **M2** | Kit commands, artifact orchestration, and audit | `UNI-7`–`UNI-10`, `UNI-21`, `UNI-22` | External command orchestration and deterministic artifact checks pass; no embedded builder. |
| **M3** | Conda static channel and consumer initialization | `UNI-11`–`UNI-13` | Real native client resolves/downloads package bytes from standard static channel paths. |
| **M4** | Trusted producer and release lanes | `UNI-14`, `UNI-15`, `UNI-20`, `UNI-23`, `UNI-34`, `UNI-54`, `UNI-56` | Pinned release intake, independent package lanes, installed-prefix evidence, isolated publishing. |
| **M5** | Airlock snapshot/bootstrap foundation | `UNI-32`, `UNI-33`, `UNI-38`, `UNI-51`, `UNI-52`, `UNI-53`, `UNI-55` | Verified transactional restore, a lock-verified mirror, generated entrypoints, and separate bootstrap/component packages. |
| **M6** | Native Cargo, Python-wheel, and npm adapters | `UNI-42`–`UNI-44` | Protocol fixtures and native clients pass without external egress. |
| **M7** | Portal from pinned catalog snapshots | `UNI-5`, `UNI-6`, `UNI-25`, `UNI-26`, `UNI-46` | Deterministic portal from the verified snapshot; browser tests pass; portal remains distinct from registries. |
| **M8** | Unified command facade and umbrella install | `UNI-40` | Thin `universe` dispatches to independently installable Kit and Airlock packages. |
| **M9** | Airlock native views and integrated v1 evidence | `UNI-45`, `UNI-47` | Four protocols work through scoped Airlock views; independent evidence review passes. |
| **M10** | Integrated Universe v1 release | `UNI-16` | All required packages, native indexes, portal, and static Conda channel publish together. |
| **M11** | Post-v1 adoption, extensions, and native developer transport | `UNI-17`, `UNI-18`, `UNI-27`, `UNI-35`, `UNI-37`, `UNI-48` | Post-v1 adoption/optional work is separated; sandbox removal waits for proven native transport. |

M5 and M6 depend on M1 and may progress in parallel with the M2–M4 producer path. The table
is a topological presentation, not an artificial serial dependency chain. M7 joins those tracks;
M8–M10 are integration/release gates. M11 is post-v1.

## Proposed sprints

The sprint train is undated because no cadence, capacity, or calendar dates were supplied. It is
an execution proposal, not a task hierarchy:

| Sprint | Focus | Tasks |
|---:|---|---|
| S0 | Close foundation/transport | `UNI-36`, `UNI-49`, `UNI-50` |
| S1 | Contracts | `UNI-30`, `UNI-31`, `UNI-41` |
| S2 | Kit core and artifact safety | `UNI-7`, `UNI-8`, `UNI-9`, `UNI-10`, `UNI-21`, `UNI-22` |
| S3 | Conda channel and Airlock foundation | `UNI-11`, `UNI-12`, `UNI-13`, `UNI-32`, `UNI-33`, `UNI-38`, `UNI-51`, `UNI-52`, `UNI-53` |
| S4 | Trusted releases and native registries | `UNI-14`, `UNI-15`, `UNI-20`, `UNI-23`, `UNI-34`, `UNI-42`, `UNI-43`, `UNI-44`, `UNI-54`, `UNI-55`, `UNI-56` |
| S5 | Portal and command facade | `UNI-5`, `UNI-6`, `UNI-25`, `UNI-26`, `UNI-40`, `UNI-46` |
| S6 | Airlock integration and evidence review | `UNI-45`, `UNI-47` |
| S7 | Integrated v1 release | `UNI-16` |
| S8 | Post-v1 adoption and native developer transport | `UNI-17`, `UNI-18`, `UNI-35`, `UNI-37`, `UNI-48` |
| S9 | Retire interim sandbox | `UNI-27` |

Dependencies inside and across sprints remain the task frontmatter source of truth. Work may
run in parallel within a sprint after its actual blockers are complete. Already-completed
`UNI-28`, `UNI-39`, `UNI-36`, `UNI-49`, and `UNI-50` are prerequisites, not scheduled sprint work.

## Post-v1 public Agent Skill rollout (`decision-9`)

The public `skills/universe-kit/SKILL.md` package must not be indexed before UNI-16 publishes
the integrated v1 and its supported command/install instructions are stable. Contributor
skills remain separate under `.agents/skills/`. UNI-17 validates `skills-ref`, version policy,
actual install commands, and public `skills.sh` installation after the v1 gate.

## Scope and sequencing history

Decision-119 supersedes decision-11's headless `v0.1.0` / portal `v0.2.0` split and replaces
decision-13's earlier adapter budget/sequencing while retaining its neutral-core/separate-adapter
architecture. The former M5 skills/dogfood work moves to M11; M5/M6/M8/M9 are re-scoped in the
ordered chain above. Generic assets, Agent Skills, STAC, and marketplace adapters are not v1
registry requirements.
