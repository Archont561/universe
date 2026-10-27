# Session lifecycle templates

Four artifacts, in production order: opening prompt, sprint proposal, task
hand-off, and sprint report. Keep the headings and order so sessions remain
easy to compare.

A session is a sprint by default. A task-long session still uses §3 and §4 and
says why it is narrower in §2. A release stage appears only when a milestone
task actually owns one.

## 1. Session opening prompt

```text
Work in the standalone `universe/` repository. Baseline first:
`pixi install --locked && pixi run setup && pixi run test`
(expect <test summary>; environment fact: <one fact that matters>).

Read AGENTS.md, backlog/README.md, milestone m-<n>, and tasks <UNI IDs>.
This session is <sprint/task> for <milestone/version if applicable>:
stage A <spike/unknown>, stage B <build tasks>, stage C `pixi run gates`,
<stage D release only if the task owns it>.

Decisions already made: <spell them out>.
Blocked/owner decision: <none or exact decision>.
Propose the sprint and stop only if scope remains genuinely ambiguous.
```

## 2. Sprint proposal

```text
Sprint proposal — <date>

Environment: <clean/dirty>, baseline <test summary>, Git HEAD <sha>.
Milestone: m-<n> — <theme and exit condition>.
Tasks by stage:
  A — spike: UNI-<n> (<unknown and proof>)
  B — build: UNI-<n>, UNI-<n>
  C — gate: pixi run gates
  D — release: <owned release step or "not in this sprint">
Not this sprint: <task and reason>.
Blockers/decisions: <none or exact list with recommendation>.
Done means: every selected AC checked in this repo, task files closed in
format, focused conventional commits, full gates green.
```

## 3. Task hand-off

```text
UNI-<n> — <title> [stage <A|B|C|D>]

Changed: <file> (<why>), …
Evidence: <command> → <result>; suite <before → after>.
Gates: fmt <✓/…> lint <…> typecheck <…> test <…> coverage <…>
Left undone: <nothing or exact follow-up task/blocker>.
```

## 4. Sprint report

```text
Sprint report — <date>

Milestone: m-<n> — <theme>; status <open/closed and exit evidence>.
Committed: <commit or "nothing">.
Release: <not in scope | published version/run | blocked reason>.
Landed: <commit subject> (<task-id>), …
Tasks: UNI-<n> Done/In Progress — <unproven AC and blocker if any>.
Gates on final tree: <pixi run gates result and coverage>.
Environment facts: <versions, lockfiles, transport caveats>.
Open next: <task IDs in order>.

Next session should start with:

> <template §1, filled in>
```

A session that produced no commits still writes a report. The blocker belongs
in the relevant Backlog task before the next session starts.
