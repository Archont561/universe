---
id: DECISION-134
title: Run the advisory audit on a daily schedule, not on every commit
status: accepted
created_date: '2026-11-08'
updated_date: '2026-11-08'
---

## Context

`pixi run gates` included the `audit` task (decision-120), and CI runs `pixi run gates`
on every pull request and every push to `main`. The audit's input is the RustSec advisory
database and the crates.io index — neither of which is in this repository, and both of
which change without anybody committing anything.

That mismatch has two costs. A pull request goes red for an advisory in a crate its diff
never touched, and the diff cannot fix it: the remedy is either a dependency bump nobody
asked for or a recorded ignore, both of which belong to a deliberate change. And the
coverage is accidental: an advisory disclosed on Tuesday is noticed on Thursday, whenever
someone next pushes, rather than on a clock.

The gate is also the only one in `gates` whose pass depends on data that may be absent — a
restored, egress-denied checkout has to `audit-sync` first (decision-120). So the per-diff
contract already carried a footnote, which the schedule removes: `gates` becomes a
statement about a diff, and the audit becomes a statement about yesterday's world.

## Decision

- The advisory audit leaves `gates`. `pixi run gates` no longer runs it, and
  `repo-check` refuses a regression that puts it back.
- A hand-written `.github/workflows/security.yml` runs `pixi run --frozen audit` daily at
  05:17 UTC, and on `workflow_dispatch`. It is hand-written rather than rendered:
  `universe.toml` describes the lanes a *consumer's* project gets, and a scheduled advisory
  sweep is owner-lane housekeeping.
- `repo-check` requires that some workflow both declares a `schedule:` and runs the audit
  task. With the audit out of `gates`, a deleted workflow or a removed cron would otherwise
  leave the repository green forever while never auditing.
- The task itself is unchanged: still `python -m scripts.audit check`, still offline-first,
  still bounded, still seeded by `pixi run audit-sync`. `audit-sync` stays the one network
  path, and the daily job relies on the bounded bootstrap the task already performs when
  the cache is cold.

## Consequences

- A pull request is judged on its diff. A new advisory cannot red a diff that did not cause
  it, and the day it lands is the day it is reported.
- A dependency *added* in a pull request is not audited until the next daily run, so a
  vulnerable new dependency can be merged and sit for up to a day. That is the accepted cost
  of the trade, and it is bounded: the window is one run, and the audit is not the only
  review of a new dependency.
- A failed scheduled run is reported to the repository's Actions tab and by email to the
  actor who last changed the cron. There is no issue-creating step: a failure that opens an
  issue needs a closing rule, or the tracker accumulates them.
- Local contributors who want the audit before pushing run `pixi run audit` (sub-second with
  the cache present) or the full `pixi run ci`, which is the same list minus this task.
- This does not change what is audited, nor the recorded ignores in `deny.toml` and
  `.cargo/audit.toml`. It changes only *when*.

## Alternatives

- **Keep the audit in `gates` and accept unrelated reds.** Rejected: a red that a diff
  cannot fix teaches contributors to re-run until it is someone else's turn, which is how a
  gate stops being read.
- **Weekly instead of daily, as `Archont561/artifactspec` does.** Rejected for this
  repository: the cron costs one short job a day, and a seven-day window on a supply-chain
  advisory is a long time to sit on a known-bad dependency. Nothing here is slow enough to
  justify the wider window.
- **Keep both: audit in `gates` *and* on a schedule.** Rejected: the per-diff run buys
  nothing the schedule does not, and the schedule is the one that can report an advisory
  that arrived between commits. `artifactspec` does run both, and the difference is that its
  `audit` task also covers its own Python development environment, which does change with a
  diff.
- **Gate the audit on paths in `gates`, so only dependency-changing diffs run it.** Rejected:
  it reintroduces the network dependency into a per-diff job, and it still cannot see an
  advisory published after the diff.
