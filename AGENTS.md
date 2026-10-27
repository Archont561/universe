# AGENTS.md

Instructions for coding agents working in the standalone Universe repository.

**Starting a work session?** Follow `.agents/skills/session/SKILL.md`. It
sequences the environment check, Backlog survey, sprint proposal, staged work,
gate run, task closure, and hand-off. A session is a sprint by default: one
milestone slice, one version target, and a checked task file at the end. A
task-long session is an explicit exception.

## What this project is

Universe is one Rust monorepo with two product lanes:

- `universe-kit` — connected owner/producer lane: declare, plan, build, audit,
  index, publish, watch, and later serve universes;
- `universe-airlock` — finite offline consumer lane: verify and consume sealed
  snapshots through native clients without network access;
- `universe-airlock-bootstrap` — the thin standalone executable generated
  restore entrypoints launch before Pixi or the full consumer exists;
- `universe-core` — ecosystem-neutral identity, manifests, fragments, plans,
  content addressing, evidence, and output contracts.

Read `backlog/README.md`, decision-13, decision-14, and decision-15 before
proposing a different architecture. Backlog is the source of truth for
product scope and status.

## Current status

This is a basic scaffold. The three binaries report version only and share a
tested plain/JSON stdout contract. Repository automation is Python under
`scripts/`, tested by pytest under `tests/scripts/`. There is no root
TypeScript configuration or TypeScript repository scripting.

## House rules

1. **Pixi is the only entrypoint.** Use `pixi run <verb>` for repository work.
   Do not invoke bare Cargo, Python, Ruff, mypy, pytest, Actionlint, or
   formatters except from Pixi task definitions or repository scripts running
   under Pixi.
2. **Backlog before scope.** Every product change belongs to a Backlog task.
   Preserve task IDs. If scope or status changes, update the task in the same
   change as the implementation.
3. **TDD at the boundaries.** Write failing tests first for parsers, planners,
   adapters, auditors, CAS/store logic, CLI output, and restore/verify flows.
   Tests target fixtures and temporary directories, never the live repository
   or a developer's real `HOME`.
4. **Core stays neutral.** `universe-core` must not gain Conda/Rattler, npm,
   Python, STAC, portal, Clap, Inquire, Ratatui, HTTP-server, or publication
   dependencies. Official adapters are feature-gated Rust crates compiled into
   the relevant binaries until an external protocol is justified.
5. **Stdout is data.** Logs, prompts, progress, and diagnostics go to stderr.
   JSON mode emits one versioned envelope; JSONL emits structured events and a
   final result. Non-TTY/CI/`--json` execution never prompts or opens a TUI.
6. **Two artifacts, one codebase.** `universe-airlock` must remain free of
   build, publishing, watcher, and network-producer capabilities at link time.
   `universe-airlock-bootstrap` stays smaller still: version, verification,
   restore, and doctor responsibilities only.
7. **Static canonical output.** A universe is portable files and hashes first;
   `serve`, portals, and dashboards are views over immutable generations, not
   alternate sources of truth.
8. **Standalone history.** Do not add references to other Universe-adjacent
   repositories, import maps, notices, or placeholder `.gitkeep` files to this
   repository. Keep this tree self-describing.
9. **Workflow discipline.** A GitHub Actions step is a full-SHA pinned
   `uses:`, one `pixi run <task>`, or a one-line host bootstrap Pixi cannot
   provide. Anything longer belongs in `pixi.toml` or `scripts/`.
10. **Complete the task file.** Check every acceptance box, set `Done`, bump
    `updated_date`, and append notes/summary using the task's existing section
    markers. Unimplemented ideas belong in a Backlog draft/task, not in code
    comments or status documents.

## Commands

```bash
pixi install --locked
pixi run setup
pixi run fmt
pixi run fmt-check
pixi run lint
pixi run typecheck
pixi run test
pixi run coverage
pixi run backlog-check
pixi run repo-check
pixi run skills-check
pixi run lint-actions
pixi run audit
pixi run build
pixi run gates
```

Focused Rust or Python work also goes through Pixi's environment, for example
`pixi run --locked cargo test -p universe-core` or
`pixi run --locked pytest tests/scripts/test_repo_check.py`.

## Skills

Contributor skills live in `.agents/skills/` and are hashed in
`skills-lock.json`. After deliberately editing a skill, run
`pixi run skills-check-write`, review the hash diff, and run
`pixi run skills-check`.
