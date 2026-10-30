---
id: DECISION-122
title: One product, two editions
status: accepted
created_date: '2026-10-30'
updated_date: '2026-10-31'
---

## Context

Universe presents several executable surfaces: a connected producer CLI, a finite offline
consumer, and a bootstrap artifact. The repository has described these as "Products" in
`README.md` and encoded them as `Product::Kit`, `Product::Airlock`, and
`Product::Bootstrap` in `universe-core`.

That framing invites the question of whether the producer and the consumer should be two
separate products, with two brands, two version lines, and two sets of documentation. The
question is worth answering once, in writing, instead of once per milestone.

## Decision

**One product, two editions.** Universe is a single product. Its editions differ in trust
posture and surface, not in brand or in data format.

| Layer | Value | Stability |
| --- | --- | --- |
| Brand | Universe | fixed |
| Editions | Universe (full) · Universe Airlock (disconnected) | two, fixed |
| Commands | `universe` · `universe-airlock` | public contract |
| Distribution packages | `universe` (umbrella) · `universe-kit` · `universe-airlock` | channel naming |
| Crates | `universe-*` | internal |

Four consequences:

1. **"Editions" replaces "Products".** `README.md` presents two rows, not three.
2. **Bootstrap is a phase of Airlock, not an edition.** It remains a separate *artifact*
   because `decision-119` says so and because `decision-115` distinguishes bootstrap
   readiness from sealed offline readiness. Naming it a third product was the error.
3. **Component packages stay separately installable.** `decision-119` requires it and it
   protects the consumer edition: an Airlock install that carried producer code would fail
   the dependency-graph requirement in `UNI-34`. A single merged binary was considered and
   rejected on that basis.
4. **One tag binds every artifact.** Independent versioning would reintroduce a
   compatibility matrix that nobody can test, and would break the property that a setup
   action pinned to one SHA installs exactly one artifact set.

**One universe holds many ecosystems.** A universe is a catalog; `EntryKind` already covers
software, data, models, and skills. Splitting a universe per ecosystem would mean three
manifests, three workflows, and no way to express a site that needs five things from three
ecosystems. Organization inside a universe is by adapter namespace, not by product split.

## Conditions that would change this decision

The answer is falsifiable, so it does not get relitigated on taste:

1. Airlock must carry a different license, or must be independently auditable as a
   standalone artifact.
2. Airlock acquires a materially different release cadence, for example security-only
   releases validated for disconnected hosts.
3. Airlock is consumed by an audience that must never see producer documentation.
4. The two stop sharing a catalog and snapshot format — a verifiable refactor, and a sign
   that something has already gone wrong.

None of these hold. Absent one, a split buys nothing and costs a shared schema.

## Consequences

- `README.md` `## Products` becomes `## Editions` with two rows and one sentence about the
  bootstrap phase.
- `Product` in `universe-core` reduces to the two editions. Doing this before the first
  publication is free; afterwards it is a breaking change to a documented envelope field.
  **Done on 2026-10-31**, before any publication: the enum is `Kit | Airlock`, and a binary
  that is not its edition's executable names itself through `render_version_as` /
  `execute_version_as` / `run_main_as`. The bootstrapper therefore reports the Airlock edition
  with `binary: "universe-airlock-bootstrap"`, which is the honest answer to "what is this".
- Naming hazard, recorded rather than solved: the word "universe" is generic and taken on
  PyPI and npm, while "Airlock" is the distinctive mark. Documentation leads with Airlock
  where disambiguation matters, and always distinguishes "the Universe channel" from "your
  universe".
- Documentation must not introduce Universe Cloud, Universe Enterprise, or Universe Hub.
  Each would be a product split this decision exists to avoid.
