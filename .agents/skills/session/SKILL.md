---
name: session
description: Run a Universe work session from environment baseline to gated hand-off. Use at the start of any work session in this repository, when asked what to work on, and again at the end for a sprint report or next-session prompt. A session is a sprint by default—one milestone slice with explicit stages—unless the user explicitly chooses one task or an external proof blocks the sprint.
---

# Session lifecycle for Universe

A session has five phases: **environment**, **Backlog survey**, **sprint
proposal**, **work**, and **close/report**. Templates live in
[`standup-template.md`](standup-template.md).

Two rules span the loop:

- never propose work the current environment cannot execute or verify;
- never close an acceptance criterion on an unproven claim.

## 0. A session is a sprint

Default shape:

> sprint → one milestone slice → staged tasks → full gates → task closure.

A release is included only when the selected milestone/task explicitly owns a
release. Do not invent a tag or release for ordinary implementation work.

Task-long sessions are legitimate when the user asks for one task or when the
next proof needs a native runner, remote workflow, or maintainer action.

## 1. Bring the environment up

Pixi is the only entrypoint.

```bash
git status --short --branch
if ! command -v pixi >/dev/null 2>&1; then
  # The checked-in restore launcher bootstraps the published linux-64 bundle.
  # Skip profile edits; expose the restored Pixi only in this shell.
  PIXI_SANDBOX_USER_TOOLS=skip bash scripts/restore.sh
  export PATH="$PWD/.pixi/tools/linux-64:$PATH"
fi
pixi --version
pixi install --locked
pixi run setup          # hooks first: they are the fast feedback loop
pixi run test
```

### Install the hooks before the first commit

`pixi run setup` runs `hooks-install`, which wires Lefthook into `.git/hooks`.
Do this at the start of the session, not when a gate first fails: the hooks are
the whole difference between a mistake that costs a second and the same mistake
costing a push, a CI run, and a force-push. What each one catches:

| Hook | Catches | Cost when it is missing |
| --- | --- | --- |
| `commit-msg` | a type or scope `.versionrc` does not allow — `style:` is rejected, the allowed set is nine types | the pull-request commit check fails after the push, and the fix is a rewrite of published history |
| `pre-commit` | unformatted Rust, a stale skill hash, an invalid Backlog record, a broken workflow ref | the same failure arrives from CI minutes later, mixed in with everything else |
| `pre-push` | anything `offline-gates` proves without the network | the push succeeds and the failure belongs to the next person who runs CI |
| `post-checkout` | nothing: it re-runs `pixi run setup`, so a checkout self-heals an environment whose hooks went missing | a stale clone keeps passing checks that the repository no longer accepts |

Verify rather than assume, because a restore can leave `.git/hooks` untouched:

```bash
for hook in commit-msg pre-commit pre-push; do
  test -x ".git/hooks/$hook" || echo "missing hook: $hook"
done
```

If one is missing, run `pixi run hooks-install`; it writes a shim for all four
hooks in one pass. Lefthook replaces an existing hook of the same name, so check
what is already in `.git/hooks` before installing over one that does something
else, and restore it afterwards if it did.

If Pixi is missing, try the repository's checked-in `scripts/restore.sh` before
reporting an environment blocker. The first sandbox publication must already
exist at `sandbox/developer-linux-64`; this initial bootstrap currently targets
Linux x86_64 only. If the branch is not available locally or from `origin`,
report that precisely rather than silently falling back to an unpinned install.
Record the baseline test result. If restoration or network access is unavailable,
say exactly which proof cannot be produced in this session.

Use `pixi run <task>` for format, lint, typecheck, tests, coverage, audits,
and checks. Focused Cargo or pytest work also goes through Pixi, for example
`pixi run --locked cargo test -p universe-core`.

## 2. Survey the Backlog

Read, in order:

1. `AGENTS.md` — mandatory repository rules;
2. `backlog/README.md` — Backlog layout and status rules;
3. the relevant milestone and task files;
4. only the decision/doc records those tasks reference.

Decision-13 owns the adapter layering, decision-14 owns artifact names, and
decision-15 owns the Pixi/Rust/Python toolchain. A task is `Done` only when
its acceptance criteria are proven in this repository.

## 3. Propose the sprint, then stop if scope is a real choice

Use template §2. Name:

- milestone and exit condition;
- stages (spike/build/gate, plus release only if owned by the milestone);
- tasks in dependency order;
- blockers and owner decisions;
- exact gate commands that will prove completion;
- what is explicitly not in scope.

If the user's request already selects the task/scope unambiguously, do not add
a ceremonial confirmation round-trip; state the plan and work.

## 4. Work the stages

- Start with the unknown that could invalidate the sprint.
- Before the first line of implementation, read the three skills that govern it:
  `tdd` (red before green, one vertical slice at a time),
  `bdd` (scenario in domain language before the code, `.feature` files through
  `rstest-bdd` where a user or a consuming repository can observe the behaviour),
  and `clean-code-principles` (one reason to change per module, intention-revealing
  names, fail fast at the boundary, no speculative generality).
  These are not a review checklist to run afterwards; they are how the slice is
  written. A slice that was never red, or whose module took on a second reason to
  change, gets fixed now rather than in a later session.
- A behaviour added after the first green still starts from a test or a scenario.
- Keep commits focused and conventional.
- Update the Backlog task in the same change as the implementation.
- Run the cheapest relevant gate during development, then `pixi run gates`
  before closing product work.

If a gate fails, read the failing tool's output and fix the smallest cause.
Do not lower coverage, silence a lint, or rewrite a skill hash without
recording why.

## 5. Close and report

Before reporting:

1. check every acceptance box that is truly proven in this repo;
2. set completed tasks to `Done` and bump `updated_date`;
3. append task notes/summary using existing section markers;
4. run `pixi run gates` and `git status --short`;
5. name anything released, blocked, or intentionally left undone.

End with template §4 and a next-session opening prompt using template §1.
Unimplemented ideas go into a Backlog task/draft, never into AGENTS, README,
or a specification as if they were decisions.
