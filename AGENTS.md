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
`scripts/`, tested by pytest under `TESTS/`. The repository uses Python for repository automation and a Bun/Turbo workspace for the
`packages/*` portal, schema, and shared configuration packages. Root TypeScript tooling must
not replace the Python repository scripts.

## House rules

1. **Pixi is the only entrypoint.** Use `pixi run <verb>` for repository work.
   Do not invoke bare Cargo, Bun/Turbo, Python, Ruff, mypy, pytest, Actionlint,
   or formatters except from Pixi task definitions or repository scripts running
   under Pixi.
2. **Backlog before scope.** Every product change belongs to a Backlog task.
   Preserve task IDs. If scope or status changes, update the task in the same
   change as the implementation.
3. **Test first, and read the two skills before you do.** Before writing any
   feature, parser, planner, adapter, auditor, store, CLI verb, or restore/verify
   flow, read `.agents/skills/tdd/SKILL.md` and `.agents/skills/bdd/SKILL.md`.
   Together with `.agents/skills/clean-code-principles/SKILL.md` they are the
   standard here, not a suggestion, and they are read *before* the first line of
   implementation, not after it.

   **TDD.** Red before green, one vertical slice at a time: one test, then only
   enough code to pass it, then the next. Never write a batch of tests for
   behaviour you have not built yet — that tests the shape you imagined. Tests
   observe public seams and their expected values come from an independent
   source: a published test vector, a worked example, the specification. An
   assertion that recomputes the answer the way the implementation does is not a
   test. `pixi run test` must show the new test failing before it passes, and a
   slice that was never red is a slice whose test may simply agree with the bug.

   **BDD.** For behaviour a user or a consuming repository can observe, establish
   the scenario in domain language first: who acts, what happens, what is
   observable afterwards. Then let it drive the implementation through
   `rstest-bdd`, which is how the Rust crates in this repository express
   scenarios. The BDD skill's preflight still applies and is not a formality:
   not every change earns a `.feature` file, and one written after the code is
   acceptance automation, so record it as that rather than calling it BDD.
   Scenarios use concrete data (a real package name, a real digest), not `foo`
   and `user1`, and they state what, never how.

   **Clean code.** Read `.agents/skills/clean-code-principles/SKILL.md` before
   implementing, and apply it *while* writing the slice rather than cleaning up
   afterwards. The parts that decide whether this codebase stays reviewable:

   - one reason to change per module — the renderer decides what a job is, the
     template decides how it looks, and neither knows the other's rules;
   - names say what a thing is, not how it is built (`store_write_scope`, not
     `get_perm_2`);
   - fail fast at the boundary: an invalid manifest is refused by the validator,
     not discovered halfway through a publish;
   - no speculative generality — a field, trait, or flag that nothing consumes
     yet is scope the next reader has to carry;
   - duplication is removed at the second occurrence, not the third, but only
     when the two things change together; a shared helper for two coincidences
     is worse than the duplication it replaced.

   The loop continues for the rest of the work, including the parts added after
   the first green: every later behaviour change starts from a test or a
   scenario, not from an edit. Tests target fixtures and temporary directories,
   never the live repository or a developer's real `HOME`.
4. **Core stays neutral.** `universe-core` must not gain Conda/Rattler, npm,
   Python, STAC, portal, Clap, Inquire, Ratatui, HTTP-server, or publication
   dependencies. Official adapters are feature-gated Rust crates compiled into
   the relevant binaries until an external protocol is justified.
5. **Package boundaries.** Bun packages live under `packages/` (plus the Cargo Turbo facade);
   use their own `VERSIONS` files and package manifests. Keep shared domain contracts in
   `universe-core` ecosystem-neutral, and keep Conda/Rattler dependencies in adapters or
   product crates.
6. **Stdout is data.** Logs, prompts, progress, and diagnostics go to stderr.
   JSON mode emits one versioned envelope; JSONL emits structured events and a
   final result. Non-TTY/CI/`--json` execution never prompts or opens a TUI.
7. **Two artifacts, one codebase.** `universe-airlock` must remain free of
   build, publishing, watcher, and network-producer capabilities at link time.
   `universe-airlock-bootstrap` stays smaller still: version, verification,
   restore, and doctor responsibilities only.
8. **Static canonical output.** A universe is portable files and hashes first;
   `serve`, portals, and dashboards are views over immutable generations, not
   alternate sources of truth.
9. **Standalone history.** Do not add references to other Universe-adjacent
   repositories, import maps, upstream notices, or placeholder `.gitkeep` files.
   Nothing is vendored: the repository `LICENSE` is the only license text, and a copied third-party license file is a lint error.
10. **Workflow discipline.** A GitHub Actions step is a full-SHA pinned
   `uses:`, one `pixi run <task>`, or a one-line host bootstrap Pixi cannot
   provide. Anything longer belongs in `pixi.toml` or `scripts/`.
11. **Complete the task file.** Check every acceptance box, set `Done`, bump
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
pixi run web-check
pixi run web-build
pixi run gates
```

Focused Rust or Python work also goes through Pixi's environment, for example
`pixi run --locked cargo test -p universe-core` or
`pixi run --locked pytest TESTS/test_repo_check.py`.

## Skills

Contributor skills live in `.agents/skills/` and are hashed in
`skills-lock.json`. After deliberately editing a skill, run
`pixi run skills-check-write`, review the hash diff, and run
`pixi run skills-check`.
