---
id: doc-112
title: Ordered milestone DAG and proposed sprint train
status: Proposed schedule; dependency DAG accepted for planning
created_date: '2026-10-27'
updated_date: '2026-10-31'
tags:
  - roadmap
  - dag
  - milestones
  - sprints
---

# Ordered milestone DAG and proposed sprint train

## Purpose and planning boundary

This is the canonical sequencing view for the integrated Universe v1 backlog. Milestones are
ordered below in one topological presentation; independent product tracks may proceed in
parallel when their task prerequisites are complete. **Only task `dependencies` are hard
blockers.** A milestone number or sprint grouping does not create a dependency edge.

The sprint train is a proposed ordered work plan with no calendar dates, duration, points, or
capacity assumption. It groups existing tasks for execution; it does not create task parents,
children, or a `sprint` field. Completed tasks are shown as prerequisites, not rescheduled work.

## Ordered milestones

| Order | Milestone | Contributing tasks | Prerequisite/outcome |
|---:|---|---|---|
| 0 | **M0 — Reproducible developer foundation** | `UNI-28`, `UNI-36`, `UNI-39`, `UNI-49`, `UNI-50` | **Complete.** Baseline (`UNI-28`, `UNI-39`), offline developer transport (`UNI-36`), the offline audit gate (`UNI-49`), and the declarative repository-lint layer (`UNI-50`) are all done. |
| 1 | **M1 — Neutral schemas and adapter contracts** | `UNI-30`, `UNI-31`, `UNI-41`, `UNI-57` | Establish the shared types, adapter boundary, project manifest, and pinned release manifest, then prove the manifest by rendering the workflow set from it. |
| 2 | **M2 — Kit commands, artifact orchestration, and audit** | `UNI-7`–`UNI-10`, `UNI-21`, `UNI-22` | Validate/plan, invoke configured external commands, and verify/audit outputs. |
| 3 | **M3 — Conda static channel and consumer initialization** | `UNI-11`–`UNI-13` | Publish verified Conda artifacts and indexes at standard static channel/subdir paths. |
| 4 | **M4 — Trusted producer and release lanes** | `UNI-14`, `UNI-15`, `UNI-20`, `UNI-23`, `UNI-34`, `UNI-54`, `UNI-56` | Establish recipes, release intake, credential separation, installed-package evidence, and provenance. |
| 5 | **M5 — Airlock snapshot/bootstrap foundation** | `UNI-32`, `UNI-33`, `UNI-38`, `UNI-51`, `UNI-52`, `UNI-53`, `UNI-55` | Implement verified restore/bootstrap, a lock-verified mirror, generated entrypoints, and independently installable Airlock packaging. |
| 6 | **M6 — Native Cargo, Python-wheel, and npm adapters** | `UNI-42`–`UNI-44` | Implement protocol-compatible native metadata and artifact endpoints. |
| 7 | **M7 — Browsable portal from pinned catalog snapshots** | `UNI-5`, `UNI-6`, `UNI-25`, `UNI-26`, `UNI-46` | Build/test/embed the portal and generate it from the same verified pinned release data. |
| 8 | **M8 — Unified command facade and umbrella installation** | `UNI-40` | Add the thin `universe` front door while preserving separate component packages. |
| 9 | **M9 — Airlock native views and integrated v1 evidence** | `UNI-45`, `UNI-47` | Consume all four protocols through scoped local views and independently review complete evidence. |
| 10 | **M10 — Integrated Universe v1 release** | `UNI-16` | Publish the complete producer, registries, Airlock, portal, and static Conda channel together. |
| 11 | **M11 — Post-v1 adoption, optional extensions, and native developer transport** | `UNI-17`, `UNI-18`, `UNI-27`, `UNI-35`, `UNI-37`, `UNI-48` | After v1, dogfood, publish the skill, assess optional extensions, prove native developer transport, then retire the interim sandbox. |

M5 and M6 depend on M1 and can progress in parallel with the M2–M4 producer path. The order
table is a clear topological linearization, not a claim that those independent tasks must wait
for every earlier-numbered milestone. M7 joins those tracks; M8–M10 are integration/release
gates. M11 is explicitly post-v1.

## Task dependency DAG

`A -> B` means B cannot complete until A completes. Milestone/sprint grouping is deliberately
absent from this graph.

```text
UNI-28 -> UNI-30 -> UNI-31 -> UNI-41 -> UNI-57   (the manifest's first consumer)
                 ├-> UNI-32 -> UNI-33
                 ├-> UNI-38
                 ├-> UNI-42 / UNI-43 / UNI-44
                 └-> UNI-7 / UNI-8
UNI-30 + UNI-31 -> UNI-10

UNI-7 + UNI-8 + UNI-31 + UNI-41 -> UNI-9 -> UNI-23 (also requires UNI-15)
UNI-10 + UNI-31 + UNI-41 -> UNI-11 -> UNI-12 -> UNI-13
UNI-10 -> UNI-21
UNI-10 -> UNI-22
UNI-7 + UNI-41 -> UNI-14
UNI-28 + UNI-41 -> UNI-34
UNI-9 + UNI-10 + UNI-34 + UNI-41 -> UNI-15
UNI-11 + UNI-15 + UNI-34 + UNI-41 -> UNI-20

UNI-30 -> UNI-5 -> UNI-6
UNI-6 + UNI-12 + UNI-41 -> UNI-25 -> UNI-26
UNI-5 + UNI-6 + UNI-12 + UNI-25 + UNI-41..44 -> UNI-46
UNI-15 + UNI-34 + UNI-38 -> UNI-40

UNI-12 + UNI-32 + UNI-33 + UNI-38 + UNI-41..44 -> UNI-45
UNI-12 + UNI-13 + UNI-14 + UNI-15 + UNI-20 + UNI-23 + UNI-25 + UNI-26
+ UNI-21 + UNI-22 + UNI-32 + UNI-33 + UNI-34 + UNI-38 + UNI-40..46
+ UNI-51..56 -> UNI-47 -> UNI-16

UNI-16 -> UNI-17 / UNI-18 / UNI-37
UNI-16 + UNI-33 + UNI-36 + UNI-45 -> UNI-48 -> UNI-27
UNI-28 -> UNI-35   (optional docs task; it does not block v1)

UNI-31 + UNI-41 -> UNI-51 -> (with UNI-32) -> UNI-52
UNI-41 -> UNI-53
UNI-20 + UNI-41 -> UNI-54
UNI-33 + UNI-38 -> UNI-55
UNI-34 + UNI-55 -> UNI-56
```

The frontmatter on each task is authoritative. If the diagram and a task's dependencies differ,
fix the task record and regenerate this view; do not add a dependency merely to make a diagram
look linear. `UNI-35` is optional and scheduled late for focus, not because v1 blocks it.

## Proposed sprint train

No dates or sprint lengths are assigned. Each task appears in one sprint only. Within a sprint,
work may proceed in parallel when its predecessor tasks are done.

| Sprint | Focus | Tasks | Ordering notes / sprint exit |
|---:|---|---|---|
| **S0** | Close foundation/transport | `UNI-36`, `UNI-49`, `UNI-50` | **Exit condition met.** The restored developer profile is verified, the audit gate is inside the offline boundary (decision-120), and repository shape is enforced declaratively (decision-121). `UNI-28` and `UNI-39` were complete before the train started. |
| **S1** | Contracts | `UNI-30`, `UNI-31`, `UNI-41`, `UNI-57` | In order: neutral models, adapter boundary, then project/release manifests; the renderer follows the manifest because a schema nothing renders from has never been falsified. |
| **S2** | Kit core and artifact safety | `UNI-7`, `UNI-8`, `UNI-9`, `UNI-10`, `UNI-21`, `UNI-22` | `UNI-9` waits for lint/planning and M1 contracts; hardening follows audit. |
| **S3** | Conda channel and Airlock foundation | `UNI-11`, `UNI-12`, `UNI-13`, `UNI-32`, `UNI-33`, `UNI-38`, `UNI-51`, `UNI-52`, `UNI-53` | Conda path is `11 -> 12 -> 13`; Airlock path is `32 -> 33`, with recipe `38` after contracts, then the mirror slices `51 -> 52` and the entrypoint renderer `53`. These tracks are parallel. |
| **S4** | Trusted release and native registries | `UNI-14`, `UNI-15`, `UNI-20`, `UNI-23`, `UNI-34`, `UNI-42`, `UNI-43`, `UNI-44`, `UNI-54`, `UNI-55`, `UNI-56` | Establish `UNI-34` before packaging `UNI-15`; then `UNI-20` and `UNI-23`. The three registry adapters can proceed in parallel after S1. The release controller `54` and the setup actions `56` follow the lanes; the bootstrap package `55` follows its recipe and verbs. |
| **S5** | Portal and public facade | `UNI-5`, `UNI-6`, `UNI-25`, `UNI-26`, `UNI-40`, `UNI-46` | Portal UI/tests precede hydration and release integration; `UNI-46` also waits for Conda/Cargo/Python/npm fixtures. Facade waits for Kit/Airlock packaging. |
| **S6** | Airlock protocol integration and evidence review | `UNI-45`, `UNI-47` | Verify four native clients through Airlock, then independently review the complete evidence matrix. |
| **S7** | Integrated v1 release | `UNI-16` | Release only after S6 and all task-level blockers pass. |
| **S8** | Post-v1 adoption and native transport | `UNI-17`, `UNI-18`, `UNI-35`, `UNI-37`, `UNI-48` | Post-release skill/dogfood/optional-extension work; native developer transport remains separate from sealed project consumption. |
| **S9** | Retire interim sandbox | `UNI-27` | Only after `UNI-48` proves the replacement and v1/Airlock prerequisites are met. |

## Change control

- Update this sprint train only when the dependency graph or priorities change; do not use it as
  a commitment to dates or capacity that have not been agreed.
- Add/remove task dependencies only for actual blockers, and keep milestones as outcomes.
- Keep v1 scope in decision-119: Conda/static Conda, Cargo, Python wheels, npm, Airlock,
  browsable portal, and pinned release artifacts. Generic assets, Agent Skills, STAC, and
  marketplace adapters remain post-v1/out of scope.
