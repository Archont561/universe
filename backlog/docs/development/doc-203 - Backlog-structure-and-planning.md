---
id: doc-203
title: Backlog structure and planning conventions
type: guide
created_date: '2026-10-27'
updated_date: '2026-10-27'
---

# Backlog structure and planning conventions

This guide records the Backlog layout and planning conventions adopted for Universe. Keep project-specific decisions and product scope in Universe's own records; do not copy another repository's identities, implementation status, or release promises.

## Record hierarchy

Use each record for one level of planning:

1. **Decisions** record durable choices and their status (`proposed`, `accepted`, `superseded`, or `deprecated`). Acceptance is a design decision, not proof of implementation.
2. **Docs** capture architecture, design, development, or roadmap detail that is too large for a decision or task.
3. **Milestones** state a coherent outcome, explicitly identify the tasks contributing to it, and define an observable exit condition.
4. **Tasks** are implementable units of work. Set `milestone: m-N` when a task contributes to a milestone; use `dependencies` for prerequisites and acceptance criteria / Definition of Done for proof.
5. **Drafts** hold unapproved work; **archive** preserves superseded drafts and closed task/milestone records without losing their IDs or history.

The roadmap at [`doc-4`](../roadmap/doc-4%20-%20Milestone-roadmap-and-skills-sh-rollout.md) is the cross-milestone summary. A milestone record remains the authoritative scope and exit gate for that milestone; a task's milestone field is the direct backlink.

## Milestones, sprint trains, and task edges

Universe plans by product/theme milestone. The milestones may list contributing task IDs, and tasks point back using `milestone: m-N`. A milestone is not a sprint and should not be replaced by a task with `type: sprint`.

The QGIS SDK backlog demonstrates a useful optional extension: when a milestone needs time-boxed sequencing, record a **sprint train** in a planning document or a `## Sprint train` section in the milestone. Keep the mapping from each sprint to the tasks and the dependency rationale there; avoid adding a `sprint` field to every task. Universe has not assigned sprint dates or adopted sprint cycles, so this is a pattern available for future planning, not a current commitment.

Keep task graph edges semantically distinct:

- `milestone` groups work under an outcome and exit gate.
- `dependencies` means a real prerequisite/blocker only: the dependent task cannot complete before the referenced task. Never use it to mean “this task contains those tasks.”
- Markdown links document non-queryable relationships such as decisions/specifications; task checklists remain acceptance criteria, not child tasks.

Universe currently has no `parent_task_id` / subtask metadata or checker support. If decomposition into independently tracked child tasks becomes valuable, add and validate an explicit parent-child edge before adopting it. Do not simulate hierarchy with a sprint-type task or misuse dependencies. For larger milestones, a plan document can sequence existing work without claiming that those tasks are formally subtasks.

## Work types and status honesty

Keep `type` about the nature of the work, not its schedule: `feature` for a capability/contract; `chore` for tooling, refactoring, release gates, or evidence-only qualification; `spike` for a bounded question that closes with a recorded recommendation; `bug` only for a reproducible failure. Use `docs` for standalone documentation work and `task` only when a more precise type does not fit. Do not create a spike for an already-understood limitation.

A task is `Done` only when its acceptance criteria are backed by repository evidence. Keep future design and unvalidated recipe/release work open. Close or archive cancelled/obsolete records without recycling IDs. Keep records and links local so Backlog checks remain usable in offline development.

## Directory layout

```text
backlog/
├── decisions/
├── docs/
│   ├── architecture/
│   ├── design/
│   ├── development/
│   └── roadmap/
├── milestones/
├── tasks/
├── drafts/
├── archive/                  # Create drafts/, milestones/, or tasks/ only when records are archived
├── README.md
└── config.yml
```

Do not commit empty-directory placeholders; create archive category directories only when they contain records. Run `pixi run backlog-check` after editing records. The checker verifies IDs, statuses, filenames, and local Markdown links; it also checks that task milestone and dependency references resolve.
