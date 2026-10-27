---
id: UNI-17
title: Author and publish the public Universe Kit skill to skills.sh after v1
status: To Do
assignee: []
created_date: '2026-10-26 20:15'
updated_date: '2026-10-27'
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
After UNI-16 publishes the integrated Universe v1 and its supported install/CLI commands
are stable, expose the public consumer skill for `skills.sh` discovery. Keep contributor
skills in `.agents/skills/` separate from the public skill package. The skill must describe
the actual supported Conda, Cargo, Python-wheel, npm, Airlock, and portal surfaces without
claiming deferred generic-assets/Agent-Skills/STAC registry support.

Validate with `skills-ref`, synchronize its metadata version, embed the public skill only if
required by the implemented `init --with-skill` workflow, and verify `npx skills add` from a
clean environment.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 `skills-ref validate skills/universe-kit` passes and metadata version matches the release policy.
- [ ] #2 Skill command examples are smoke-tested against the integrated v1 packages and distinguish portal discovery from native registry access.
- [ ] #3 Public `skills.sh` indexing/installation succeeds after v1; internal contributor skills remain excluded from public indexing.
- [ ] #4 Offline embedding/init tests pass if that feature is implemented; the skill does not falsely promise an offline closure not present in caches.
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 Public README documents the verified install command and no pre-v1 skill is indexed.
<!-- DOD:END -->
