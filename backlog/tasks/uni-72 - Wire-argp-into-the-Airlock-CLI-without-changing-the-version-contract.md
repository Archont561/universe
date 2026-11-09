---
id: UNI-72
title: Wire argp into the Airlock CLI without changing its version contract
status: Done
assignee: []
created_date: '2026-11-09'
updated_date: '2026-11-09'
labels:
  - airlock
  - cli
  - infrastructure
milestone: m-0
documentation:
  - backlog/decisions/decision-122 - One-product-two-editions.md
  - backlog/decisions/decision-126 - Release-control-lives-in-Universe-with-attempts-promotion-and-a-resumable-journal.md
priority: medium
type: chore
ordinal: 72000
---

## Description
<!-- SECTION:DESCRIPTION.BEGIN -->
The `argp` dependency arrived with its own round trip: the relock bot repaired `Cargo.lock`
and `publish-sandbox` repacked the vendored Cargo closure, so the offline developer
environment can compile a consumer that uses it. Until this slice landed, nothing consumed
that dependency — `universe-airlock` still routed every argument to
`universe_core::execute_version`, whose hand-written parser knows only `version`,
`--version`, `-V`, and `--json`.

Wiring the parser in is a behaviour change, not a refactor that can be reasoned about from
the types alone. `argp` has its own usage text, its own error vocabulary, and its own early
exit for `--help`. The version report, the usage diagnostic, and the stream and exit-code
contract stay with `universe-core`, because the other two editions still use that renderer.
The slice therefore has two jobs: give the consumer a real grammar with observable help, and
prove that the report did not move by even one byte.
<!-- SECTION:DESCRIPTION.END -->

## Acceptance Criteria
<!-- AC.BEGIN -->
- [x] #1 `universe-airlock`'s `run` and `run_main` parse through `argp`'s `FromArgs::from_args`, so the declared dependency is consumed and no crate reports an unused dependency.
- [x] #2 `--help` succeeds on stdout, names the version command, and leaves stderr empty. A BDD scenario and a unit test were written and seen red before the wiring was added.
- [x] #3 The version contract is byte-identical to `universe-core`'s reference renderer for no arguments, `version`, `--version`, `-V`, `--json`, and `version --json`; the parity tests take their expected values from `execute_version`, not from the implementation under test.
- [x] #4 An unknown verb still exits 2 with empty stdout and a refusal that names it. The three existing assertions on that diagnostic were left unchanged rather than reworded to fit the new parser.
- [x] #5 `universe-core` stays neutral (`crate-check`), and the usage diagnostic and the print-and-exit contract live in one place shared with the other editions.
- [x] #6 `pixi run gates` passes, with 82 alint rules, the full Rust and Python suites, and the workflow-render drift check green.
<!-- AC.END -->

## Definition of Done
<!-- DOD.BEGIN -->
- [x] #1 The red run is recorded in the notes with the exact failures.
- [x] #2 The pull request is merged by rebase with local gates green first.
- [x] #3 `docs/DEVELOPMENT.md` records that the consumer parses with `argp` while the core owns output, and both roadmap documents list this task.
<!-- DOD.END -->

## Notes
<!-- NOTES.BEGIN -->
**2026-11-09 — red before green.** The tests were written against the unwired consumer and
run with `pixi run --frozen cargo nextest run -p universe-airlock --all-features --locked
--no-fail-fast`:

```
        FAIL [   0.010s] ( 3/13) universe-airlock::airlock_bdd the_consumer_lists_what_it_can_do
      left: 2
     right: 0
        FAIL [   0.005s] ( 5/13) universe-airlock::cli a_repeated_option_is_refused_by_name
      left: 0
     right: 2
        FAIL [   0.005s] ( 7/13) universe-airlock::cli help_is_a_success_that_names_the_version_command
      left: 2
     right: 0
     Summary [   0.048s] 13 tests run: 10 passed, 3 failed, 0 skipped
```

The help scenario failed with `error: unknown argument: --help` and the usage line on
stderr, which is the behaviour the slice replaces.

**Correction, recorded as a correction.** The third failure was a mis-specified test, not a
missing behaviour. `a_repeated_option_is_refused_by_name` asserted that `--json --json`
should exit 2, but neither parser ever refused that: the scaffold parser set `json = true`
twice, and `argp` models a switch as a `Flag` that accepts repetition. The test was replaced
with `a_repeated_switch_still_reports_one_envelope`, which pins the pre-existing contract —
repeating the switch yields the same single envelope — instead of asserting a refusal that
never existed. No production code was written to make the mistaken expectation pass.

**What the wiring does.** `Airlock` declares a `-V`/`--version` switch, a `--json` switch,
and a `version` subcommand. On success it forwards the parsed request to
`universe_core::execute_version` in the core's own vocabulary, so the envelope and the
one-line report are rendered by exactly the function the bootstrap and the producer use.
`EarlyExit::Help` becomes a successful stdout report; `EarlyExit::Err` is translated into
`CliError::UnknownArgument` and rendered by the shared diagnostic. `universe-core` gained
`version_usage_output` and `emit` so the diagnostic text and the stdout/stderr/exit-code
rule each have one implementation; its own `execute_version_as` and `run_main_as` now call
them. `crate-check` still finds `universe-core` ecosystem, UI, and network free.

**Process-level guard.** `the_binary_prints_help_on_stdout_with_no_diagnostic` drives the
built binary, because `run` is a pure seam and a host sees the process. Together with the
two parity tests and the unchanged refusal assertions, the version contract is held from
three directions.
<!-- NOTES.END -->
