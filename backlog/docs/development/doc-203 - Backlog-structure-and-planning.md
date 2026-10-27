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

## Milestones, sprints, and task edges

Universe plans by product/theme milestone. Each milestone lists its contributing task IDs and observable exit gate; tasks point back using `milestone: m-N`. Milestones are outcomes, not task parents or sprints.

Universe now uses an **undated sprint train** as a scheduling overlay. The current ordered sequence is in [doc-112](../roadmap/doc-112%20-%20Ordered-milestone-DAG-and-sprint-train.md), which maps each proposed sprint to existing tasks and records the ordering rationale. No sprint dates, duration, capacity, or points are committed. Do not add a `sprint` field to every task; the sprint plan must not be mistaken for task hierarchy.

Keep task graph edges semantically distinct:

- `milestone` groups work under an outcome and exit gate.
- `dependencies` means a real prerequisite/blocker only: the dependent task cannot complete before the referenced task. Never use it to mean “this task contains those tasks” or merely to copy sprint order.
- Sprint order is a planning preference. It may group independent tracks, but it is not a hard blocker unless an actual task dependency says so.
- Markdown links document non-queryable relationships such as decisions/specifications; task checklists remain acceptance criteria, not child tasks.

Universe has no `parent_task_id` / subtask metadata or checker support. Do not introduce parent/child tasks or a sprint task type to simulate hierarchy. If decomposition becomes necessary, design and validate an explicit relationship separately. The sprint train only sequences existing flat tasks; task frontmatter remains the source of truth for blockers.

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
