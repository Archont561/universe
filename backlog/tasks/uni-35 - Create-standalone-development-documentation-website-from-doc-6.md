---
id: UNI-35
title: Create standalone development documentation website from doc-6
status: To Do
assignee: []
created_date: '2026-10-27'
updated_date: '2026-10-27'
labels:
  - docs
  - infrastructure
milestone: m-11
dependencies:
  - UNI-28
priority: low
type: feature
ordinal: 35000
---

## Description
<!-- SECTION:DESCRIPTION:BEGIN -->
If approved later, implement the Astro + Starlight design in doc-6 as a
standalone development-repository website. It must remain outside product
packages, release archives, generated consumer portals, and the root product
gate path.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The website has its own lockfile/build and is excluded from product artifacts.
- [ ] #2 Sitemap, canonical/base-path URLs, robots, OG metadata, and LLM mirrors are generated and asserted.
- [ ] #3 Every command/configuration page labels currently available, integrated-v1 target, and post-v1 roadmap material accurately.
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 Product `pixi run gates` does not depend on website output.
- [ ] #2 The docs workflow uses full-SHA pinned actions.
<!-- DOD:END -->
