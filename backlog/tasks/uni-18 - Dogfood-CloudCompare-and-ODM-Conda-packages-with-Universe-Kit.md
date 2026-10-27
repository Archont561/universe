---
id: UNI-18
title: Dogfood CloudCompare and ODM Conda packages through Universe v1
status: To Do
assignee: []
created_date: '2026-10-26 20:15'
updated_date: '2026-10-27'
labels:
  - channel
  - portal
  - testing
milestone: m-11
dependencies:
  - UNI-16
documentation:
  - backlog/docs/architecture/doc-2 - Universe-Kit-CLI-and-manifest-specification.md
  - backlog/docs/architecture/doc-3 - Static-channel-and-SvelteKit-UnoCSS-portal-specification.md
  - backlog/docs/architecture/doc-111 - V1-package-layout-and-offline-readiness.md
  - backlog/docs/roadmap/doc-4 - Milestone-roadmap-and-skills-sh-rollout.md
priority: medium
type: task
ordinal: 18000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
After the integrated v1 release, dogfood the producer and consumer workflow against the
real-world CloudCompare/ODM geospatial workload. Scaffold a multi-package Conda universe,
migrate recipes/workflows where appropriate, and exercise planning, configured external
build/test commands, audit, trusted publication, static channel installation, and the live
portal. This task validates a representative Conda workflow; it does not replace UNI-47's
four-ecosystem v1 evidence gate.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Consumer repository uses the supported manifest/recipe format and generated workflows without custom Python/Bash CI glue.
- [ ] #2 Both packages build through declared external tools, pass audit, and publish to the project's static channel with package bytes available at standard channel/subdir paths.
- [ ] #3 `pixi install` resolves/installs directly from the static Conda channel with egress restricted to the configured host; portal pages display the same pinned catalog data.
- [ ] #4 Findings and ergonomics gaps become follow-up tasks; results are clearly limited to Conda dogfooding and do not imply Cargo/Python/npm behavior beyond UNI-47 evidence.
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 Dogfood report, catalog/release pins, and reproducible install evidence are recorded.
<!-- DOD:END -->
