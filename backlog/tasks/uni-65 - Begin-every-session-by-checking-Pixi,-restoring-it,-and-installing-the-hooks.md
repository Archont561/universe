---
id: UNI-65
title: Begin every session by checking Pixi, restoring it, and installing the hooks
status: Done
assignee: []
created_date: '2026-11-08'
updated_date: '2026-11-08'
labels:
  - tooling
  - infrastructure
  - documentation
milestone: m-0
documentation:
  - backlog/decisions/decision-139 - A-session-begins-by-bringing-the-environment-up-and-wiring-the-hooks.md
priority: medium
type: chore
ordinal: 65000
---

## Description
<!-- SECTION:DESCRIPTION.BEGIN -->
`AGENTS.md` told an agent to follow the session skill, and the session skill told it to bring
the environment up. Everything an agent reads first said nothing about the one instruction that
has to happen before it knows it needs it: check that Pixi resolves, restore it when it does
not, and install the Lefthook hooks before the first commit.

`AGENTS.md` house rule 14 now states the three steps and the commands, `.alint.yml` holds one
rule per command so the rule cannot be edited away, and `scripts/repo_check.py` holds the half
the documentation cannot — that `pixi run setup` still reaches `hooks-install`, and that
`hooks-install` still runs `lefthook install`.
<!-- SECTION:DESCRIPTION.END -->

## Acceptance Criteria
<!-- AC.BEGIN -->
- [x] #1 `AGENTS.md` carries a house rule that begins a session with `command -v pixi`, falls back to `bash scripts/restore.sh`, and then runs `pixi run setup`. — 2026-11-08: house rule 14, with the commands in a fenced block.
- [x] #2 The rule says why each step is the one it is: the check, the checked-in launcher rather than an install, and the hooks before the first commit. — 2026-11-08: one bullet per step, including the `.git/hooks` verification loop.
- [x] #3 `.alint.yml` refuses an `AGENTS.md` that has lost any of the three commands, each with a policy test under `alint-tests/session/`. — 2026-11-08: `agents-md-checks-for-pixi`, `agents-md-names-the-restore-script`, `agents-md-installs-the-hooks-at-session-start`; 11/11 policy tests pass.
- [x] #4 `repo-check` refuses a `pixi.toml` whose `setup` no longer reaches `hooks-install`, or whose `hooks-install` no longer runs `lefthook install`. — 2026-11-08: `validate_session_bootstrap` plus a test covering both renames.
<!-- AC.END -->

## Definition of Done
<!-- DOD.BEGIN -->
- [x] #1 `pixi run alint-check`, `pixi run alint-test`, `pixi run repo-check`, and `pixi run backlog-check` pass. — 2026-11-08: 80/80 rules, 11/11 policy tests, both checkers clean.
- [x] #2 `backlog/decisions/decision-139` records the choice, including the alternatives it rejected. — 2026-11-08: the skill, a wrapper script, and `post-checkout`.
<!-- DOD.END -->
