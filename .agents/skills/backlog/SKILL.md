---
name: backlog
description: Managing Universe project work as Markdown Backlog records. Use when creating, breaking down, tracking, or updating tasks; when the user mentions the backlog, a task ID (UNI-N), acceptance criteria (AC), milestones, decisions, or a Definition of Done (DoD).
---

Universe keeps work as plain Markdown under `backlog/`, versioned with the
code. Edit those files directly and validate them with the Python checker:

```bash
pixi run backlog-check
```

There is no separate Backlog CLI task in the standalone scaffold. The files,
their frontmatter, and `pixi run backlog-check` are the contract.

## Layout and IDs

- Tasks: `backlog/tasks/uni-<n> - <Title-with-dashes>.md`, frontmatter
  `id: UNI-<n>`, statuses `To Do`, `In Progress`, or `Done`.
- Decisions: `backlog/decisions/decision-<n> - <Title>.md`, statuses
  `accepted`, `proposed`, `superseded`, or `deprecated`.
- Docs: `backlog/docs/.../doc-<n> - <Title>.md`.
- Milestones: `backlog/milestones/m-<n> - <Title>.md`.
- Archived records go under `backlog/archive/{drafts,milestones,tasks}/` when those
  categories have content; do not create empty placeholders.
- Airlock-lane decisions/docs use the 100-series; engineering foundation docs
  use the 200-series.
- Milestones are outcome/exit-gate groupings, not sprints. Tasks link back with
  `milestone: m-<n>`; `dependencies` means a real blocking prerequisite, never
  composition. A milestone can use a roadmap plan / `## Sprint train` when
  sequencing is needed, but Universe has no sprint task type or parent/subtask
  field. Do not treat acceptance checkboxes as child tasks.
- See [`doc-203`](../../../backlog/docs/development/doc-203%20-%20Backlog-structure-and-planning.md)
  for the planning model adopted in this repository.

A filename must start with the same ID as its frontmatter, followed by
space-hyphen-space. Preserve existing IDs; choose the next unused number in
the relevant series for a new record.

## The loop

1. **Find work** — read `backlog/README.md`, the milestone file, and candidate
   task files. Search text/IDs with ordinary file tools.
2. **Read before coding** — read the task description, AC, DoD,
   dependencies, and linked decisions/docs. Match test and interface names
   to the task vocabulary.
3. **Claim and plan** — set `status: In Progress`, bump `updated_date`, and
   record the approach in implementation notes when the task has note markers.
4. **Record progress** — update notes with evidence as work lands. Do not
   check an acceptance box until the behaviour is proven in this repository.
5. **Verify** — run the focused checks during work and `pixi run gates`
   before closing product work.
6. **Close** — check every proven AC/DoD box, set `status: Done`, bump
   `updated_date`, and append a concise final note/summary.

## Creating a task

Create `backlog/tasks/uni-<n> - <Title-with-dashes>.md` with frontmatter:

```yaml
---
id: UNI-<n>
title: <imperative title>
status: To Do
assignee: []
created_date: 'YYYY-MM-DD'
updated_date: 'YYYY-MM-DD'
labels:
  - <label>
milestone: m-<n>
dependencies: []
priority: medium
type: feature
ordinal: <n>000
---
```

Then add `Description`, `Acceptance Criteria`, and `Definition of Done`
sections. Keep the existing `<!-- SECTION:... -->`, `<!-- AC:BEGIN -->`, and
`<!-- DOD:BEGIN -->` markers when editing generated task files.

## Editing rules

- Keep Markdown links inside `backlog/` relative and resolvable;
  `backlog-check` validates them.
- Dependencies use uppercase task IDs (`UNI-30`) and must point at existing
  task records. Milestone references must resolve to a milestone record;
  `backlog-check` validates both relationships.
- Do not add `.gitkeep` placeholders; create directories only with content.
- A design statement is not implementation evidence. Status stays open until
  tests/gates prove it here.
