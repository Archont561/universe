---
id: decision-17
title: CLI stdout contract and interactive UX boundaries
date: '2026-10-27'
status: proposed
---

## Context

Universe Airlock decision-111 left Inquire/Ratatui-style UX libraries proposed. The
Universe also needs official ecosystem adapters and automation-friendly
CLI behaviour across two artifacts with different trust profiles.

## Proposed decision

- Stdout is data only. Logs, progress, prompts, and human diagnostics go to
  stderr.
- `--json` emits one versioned result envelope. `--jsonl` emits structured
  progress/diagnostic events followed by one authoritative result event.
- Artifact bytes never travel over stdout; outputs reference paths plus
  SHA-256 digests.
- Exit codes are `0` success, `1` user/policy failure, `2` config/usage error,
  and `3` infrastructure failure.
- Official adapters are feature-gated Rust crates compiled into the relevant
  binary. Their in-process requests/results remain serializable so an external
  adapter protocol can be extracted after several adapters stabilize.
- Clap owns command grammar when commands land. Inquire may be used for TTY
  questions; Ratatui is reserved for an explicit dashboard/UI mode.
- `--json`, `--jsonl`, `--yes`, `--non-interactive`, CI, non-TTY execution, and
  `TERM=dumb` suppress all prompts and full-screen UI.
- `universe-core` and `universe-airlock-bootstrap` contain no Clap, Inquire, or
  Ratatui dependency.

## Status in the scaffold

The version commands already exercise the plain/JSON split, stderr-only usage
errors, and exit code `2` for unknown arguments without taking a CLI framework
dependency. The interactive library choice remains proposed until the first
real command surface is designed.
