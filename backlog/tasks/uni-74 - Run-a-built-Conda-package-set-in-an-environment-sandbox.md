---
id: UNI-74
title: Run a built Conda package set in an environment sandbox
status: In Progress
assignee: []
created_date: '2026-11-12'
updated_date: '2026-11-12'
labels:
  - airlock
  - conda
  - offline
  - testing
milestone: m-12
dependencies:
  - UNI-52
documentation:
  - backlog/decisions/decision-102 - Original-conda-artifacts-and-effective-metadata.md
  - backlog/decisions/decision-113 - Native-client-execution-through-local-package-views.md
  - backlog/docs/doc-109 - Git-backed-catalogs-and-on-demand-command-execution.md
  - backlog/docs/doc-106 - Offline-verification-and-acceptance-gates.md
priority: high
type: feature
ordinal: 74000
---

## Description
<!-- SECTION:DESCRIPTION.BEGIN -->
Give a person who has just built a Conda package a way to use it: install the set into a
prefix and run a command inside that prefix, without publishing it and without solving
anything.

The command is `universe-airlock sandbox`, in the binary a consumer installs from the
channel, because the person who needs it is the person building packages against that
channel. Two sources: `--package <file.conda>` for the archives just built, repeated for a
set, and `--mirror <dir>` for a mirrored lock, which is what a Pixi user already has.

The install is `UNI-52`'s installer, so the guarantees are the restore's: every archive is
read, `info/paths.json` is applied, and every installed file is checked after relocation.

Scope boundary to state plainly: this is an **environment** sandbox, not an
operating-system one. No mount, user, or network namespace is created, and the child can
still reach the network and the host filesystem. What is claimed is `PATH`, `CONDA_PREFIX`,
the removal of the host's Python configuration, and a prefix that is discarded
afterwards. Claiming an air gap would be a claim this code cannot support.
<!-- SECTION:DESCRIPTION.END -->

## Acceptance Criteria
<!-- AC.BEGIN -->
- [x] #1 A command runs out of the installed prefix: `<prefix>/bin` is first on `PATH` and `CONDA_PREFIX` names that prefix. — 2026-11-12: `crates/universe-airlock/tests/sandbox.rs` ships a command that prints `$CONDA_PREFIX` and asserts it matches the prefix the run reported, and a second test shadows the host's `sh` with the package's own to prove the order decides.
- [x] #2 A set that cannot be installed never runs the command, and the refusal names what was wrong. — 2026-11-12: a file that is not an archive fails with `airlock.prefix.archive` and the test asserts the command's marker file was never created.
- [x] #3 The host's Python configuration cannot reach the command. — 2026-11-12: `PYTHONPATH`, `PYTHONHOME`, `PYTHONSTARTUP`, `PYTHONEXECUTABLE`, `VIRTUAL_ENV` and `CONDA_PREFIX_1` are removed and `PYTHONNOUSERSITE` is set; a test runs the sandbox with `PYTHONPATH` and `VIRTUAL_ENV` pointing at fake host directories and asserts neither appears in the child's environment while `CONDA_PREFIX` does.
- [x] #4 A temporary prefix is discarded, and one the caller keeps is where the run says it is. — 2026-11-12: the default run deletes it and the test asserts the path no longer exists; `--keep` leaves `bin/<command>` in place.
- [x] #5 The run states what it does not do. — 2026-11-12: every run carries `airlock.sandbox.environment` on stderr, saying the command can still reach the network and the filesystem outside the prefix, and a test asserts the note is present.
- [x] #6 `--json` reports the install and the command as one envelope, and the process exit code is the command's. — 2026-11-12: the envelope carries the prefix, artifact count, order, command, exit, and the child's two streams; a command exiting 3 produces exit 3 with `ok: true` and `result.exit: 3`.
<!-- AC.END -->

## Definition of Done
<!-- DOD.BEGIN -->
- [x] #1 A package built by this repository's own `pixi run package-airlock` runs inside a sandbox on this platform, and the transcript is recorded in the task notes. — 2026-11-12: run against the four `linux-64` archives this repository published to its `channel` branch (`universe`, `universe-airlock`, `universe-airlock-bootstrap`, `universe-kit`, all `0.1.0-h41f06ac_0`), which are the same archives `pixi run package-*` produces; the transcript is at the end of this file.
- [x] #2 `pixi run gates` passes and no test requires network egress. — 2026-11-12: the sandbox tests build their archives locally and run `sh` and the package's own commands; nothing fetches.
<!-- DOD.END -->

## Notes

2026-11-12 — the verb exists; what it does not do is the interesting half.

`universe-airlock sandbox` installs a set with `universe_airlock_bootstrap::prefix` and runs one
command in it. Four decisions are worth recording, because each of them is a place a
sandbox-shaped verb usually overclaims.

- **It installs through the restore's installer rather than a shortcut.** A sandbox that copied
  archives into a directory would run the command against whatever the host provided, and the
  result would be a test of the host. The digest checks and `paths.json` application are the
  reason a run here means something.
- **A failing install runs nothing.** The command is spawned only after `replay` returns, so a
  broken archive produces a refusal and not a program that happened to work.
- **The host's Python configuration is removed, not merely shadowed.** `PYTHONPATH` and an
  activated virtualenv win over `PATH` for imports, so leaving them would let a `noarch: python`
  package import from outside its own set. `PYTHONNOUSERSITE` closes the same hole for user
  site-packages.
- **The limits are printed on every run.** No namespaces are created and no egress is denied, so
  the note says the command can still reach the network. The `decision-113` view work and a real
  air gap are separate; this verb is the local loop, not the sealed one.

Two gaps this leaves open, stated so they are not mistaken for coverage. Definition of done #1
still needs a sandbox built from this repository's own `universe-airlock` package and a recorded
transcript; and no test yet runs a real interpreter inside the sandbox — the `noarch: python`
entry-point path is covered by `UNI-52`'s prefix tests, and an end-to-end Python run needs a
package that carries an interpreter, which is the real-lock work rather than a fixture.

2026-11-12 — the real artifacts, and what the run said.

The fixtures prove the behaviour; the published channel proves the verb. Its four `linux-64`
archives are the ones `pixi run package-*` builds and the release run publishes, so running them
is running this repository's own packages rather than something shaped like them.

```text
$ universe-airlock sandbox \
    --package universe-0.1.0-h41f06ac_0.conda \
    --package universe-airlock-0.1.0-h41f06ac_0.conda \
    --package universe-airlock-bootstrap-0.1.0-h41f06ac_0.conda \
    --package universe-kit-0.1.0-h41f06ac_0.conda \
    -- universe --version
warning: [airlock.sandbox.environment] this sandbox isolates the environment, not the machine: …
universe-kit 0.1.0
sandbox: /tmp/universe-airlock-sandbox-106599-0
artifacts: 4
order: universe-airlock, universe-airlock-bootstrap, universe-kit, universe
command: universe --version
exit: 0
prefix kept: no
```

Two things that transcript is evidence for, and one it is not. It shows the installer applying four
real `info/paths.json` files into one prefix and the facade finding its three siblings on the
sandbox's `PATH`, which is what the umbrella package's own test does for a Pixi environment — and
it shows the install order coming from the archives rather than from the command line. It is not
evidence of an air gap: egress was not denied, and the note above each run says so.

The single-package case is the shorter one a user meets first:
`universe-airlock sandbox --package universe-airlock-0.1.0-h41f06ac_0.conda -- universe-airlock
--version` prints `universe-airlock 0.1.0` out of the prefix it installed.
