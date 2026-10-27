---
id: UNI-37
title: Evaluate ecosystem distribution adapters
status: To Do
assignee: []
created_date: '2026-10-27'
updated_date: '2026-10-27'
labels:
  - architecture
  - core
  - release
milestone: m-9
dependencies:
  - UNI-30
  - UNI-31
priority: low
type: chore
ordinal: 37000
---

## Description
<!-- SECTION:DESCRIPTION:BEGIN -->
Evaluate whether Universe should support publishing ecosystem-specific
artifacts to GitHub Actions Marketplace, MCP registries/marketplaces, the
Visual Studio Code Marketplace, and the QGIS plugin repository. Treat these as
separate optional adapters over an ecosystem-neutral core, not as one shared
plugin format or as replacements for each ecosystem's discovery and install
flow. Consider prioritising GitHub Actions and MCP as initial candidates, then
compare their readiness with VS Code and QGIS before committing to an
implementation sequence.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 A comparison records each ecosystem's artifact format, required metadata, build/test workflow, publishing mechanism, compatibility policy, and offline/bundling implications.
- [ ] #2 The comparison distinguishes Universe build/validation/publishing support from marketplace discovery, installation, and hosting responsibilities.
- [ ] #3 A recommendation identifies a pilot ecosystem (or recommends deferral), scope boundaries, and the adapter contract implications; implementation is not implied by this evaluation task.
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 The recommendation is reviewed and any accepted architectural direction is captured in a Backlog decision before implementation work is created.
