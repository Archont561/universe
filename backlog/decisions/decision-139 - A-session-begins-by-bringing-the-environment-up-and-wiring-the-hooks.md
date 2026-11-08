---
id: DECISION-139
title: A session begins by bringing the environment up and wiring the hooks
status: accepted
created_date: '2026-11-08'
updated_date: '2026-11-08'
---

## Context

`.agents/skills/session/SKILL.md` already told an agent to check for Pixi, restore it when it
was absent, and run `pixi run setup` for the Lefthook hooks. It told that to a reader who had
chosen to open the skill. `AGENTS.md` — the file every agent reads first, and the only one
this repository can hold a gate over — said "follow the skill" and nothing about the
environment.

The gap was not theoretical. Twice in one session the Pixi launcher disappeared from the
sandbox between stretches of work, and twice the recovery was `bash scripts/restore.sh`. An
agent that does not know that, and that starts by surveying the Backlog, plans work and then
discovers it cannot run any of it. The same shape applies to the hooks: a session that makes
its first commit before `lefthook install` has run finds out from `commit-msg` at the worst
moment, with the message already written and the branch about to be pushed.

## Decision

`AGENTS.md` house rule 14 makes the start of a session explicit, in three steps and in this
order:

1. `command -v pixi` — check that the entrypoint exists. Not `pixi --version`: the same
   question asked more slowly, and it fails as though the environment were broken rather than
   absent.
2. `PIXI_SANDBOX_USER_TOOLS=skip bash scripts/restore.sh` when it does not — the checked-in
   launcher, not a `cargo install`, a system package manager, or a bare `pip install`. The
   gates were measured against the version the bundle carries.
3. `pixi run setup` — the hooks, before the first commit rather than after the first rejected
   one.

The consequences, stated so they are not rediscovered later:

- The rule names the commands, and `.alint.yml` holds three `file_content_matches` rules over
  it, one per command (`agents-md-checks-for-pixi`,
  `agents-md-names-the-restore-script`, `agents-md-installs-the-hooks-at-session-start`). A
  rewrite that keeps the commands keeps the rule green; an edit that drops one does not.
- `scripts/repo_check.py::validate_session_bootstrap` holds the other half, which the
  documentation cannot: `setup` must depend on `hooks-install`, and `hooks-install` must run
  `lefthook install`. A `setup` that still exists but reaches nothing is a command that exits
  zero and installs no hooks.
- The skill keeps its own copy of the sequence and stays the fuller procedure — it continues
  with `pixi install --locked`, the baseline test run, and the session's phases. This record
  moves the first three steps into the file an agent cannot skip reading.
- The verification step stays in the rule rather than being assumed: a restore can leave
  `.git/hooks` untouched, so `test -x .git/hooks/pre-commit` is what says the hooks are
  really there.

## Alternatives considered

- **Leave it in the skill.** The skill is where the detail belongs and where it already was.
  The reason it is not enough is that a rule an agent has to choose to read is advice, and
  this is the one instruction that has to happen before the agent knows it needs it.
- **A `pixi` wrapper script in `scripts/`.** It would hide the restore behind one command, but
  it also hides which step ran and why, and it adds a shell entrypoint where house rule 1
  keeps exactly one.
- **Let `post-checkout` install the hooks and skip the step.** Lefthook's `post-checkout` hook
  exists only after it has been installed, so it cannot bootstrap itself.
