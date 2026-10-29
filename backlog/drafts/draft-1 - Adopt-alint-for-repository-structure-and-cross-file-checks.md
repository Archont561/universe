---
id: draft-1
title: Adopt alint for repository structure and cross-file checks
status: draft
created_date: '2026-10-29'
updated_date: '2026-10-29'
source: https://github.com/Archont561/universe/issues/13
labels:
  - tooling
  - lint
  - gates
related_tasks:
  - UNI-7
  - UNI-39
---

## Origin

GitHub issue #13, imported into Backlog on 2026-10-29. No Backlog record existed
for it. Unapproved: this file records the proposal and its prerequisites; it is not
scheduled work.

## Proposal

Replace the declarative half of the three hand-written Python repository checks
(`scripts/repo_check.py`, `scripts/skills_check.py`, `scripts/backlog_check.py`)
with `alint`, a config-driven repository linter shipped as a prebuilt binary through
a PyPI wheel. Structure checks become `.alint.yml` rules; irreducible product checks
stay as `command` rules. Roll out non-blocking with `alint baseline` first, then flip
to blocking.

## Reality check against this repository (verified 2026-10-29)

- The three checks total 27 348 bytes, matching the issue's "~27 KB" premise.
- `alint` is absent from `pixi.toml`/`pixi.lock`; nothing in the tree references it.
- Gate ownership is decision-15: `pixi run gates` is the contract and Pixi is the only
  entrypoint. Adding a Python-packaged Rust binary through `pixi add --pypi alint`
  changes the environment solve and therefore needs a decision-15 amendment, not a
  silent task-level adoption.
- The offline/transport story is UNI-36's: the developer transport bundles only the
  Pixi environment and vendored crates. A wheel dependency enters `pixi.lock` and the
  614 MiB payload, so the transport must be republished before the gate can be
  claimed offline-ready.
- House rule 1 forbids invoking linters outside Pixi tasks, so `alint` would run as a
  `pixi run <task>` wrapper, never as a bare binary.

## Prerequisites

- An amendment to decision-15 covering the extra linter and its gate slot.
- `UNI-36` transport closure (payload change and republish trigger review).
- A decision on which Python checks the declarative layer retires; `UNI-39` already
  owns Backlog reference integrity and must not lose coverage.

## Recommended disposition

Keep as a draft until the UNI-36/m-0 slice closes, then decide as post-v1 tooling
(m-11) or fold into `UNI-7` if the declarative rule model is adopted first.
