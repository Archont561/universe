---
id: UNI-81
title: Answer `--help` with the usage a verb accepts
status: To Do
assignee: []
created_date: '2026-11-13'
updated_date: '2026-11-13'
labels:
  - usability
milestone: m-8
dependencies:
  - UNI-40
documentation:
  - backlog/tasks/uni-40 - Add-thin-universe-command-facade-and-umbrella-Conda-package.md
priority: low
type: feature
ordinal: 81000
---

## Description
<!-- SECTION:DESCRIPTION.BEGIN -->
GitHub issue #125 records this as a related nit beside the component-resolution defect: the
first thing a person types against a new tool is `--help`, and every Universe verb answers it
with an error.

Verified on `bd979c1b`:

```text
$ universe-kit --help
error: unknown argument: --help
usage: universe-kit [version] [--json]          (exit 2)

$ universe-kit ci --help
error: ci.usage.invalid: unknown argument: --help
usage: universe ci <render|check> [--root <dir>] [--stdout] [--print-required-checks] [--json]
```

The usage lines exist and are accurate — each verb family already builds one from its
`VerbSpec`. What is missing is the two facts a person cannot get without them: that `--help` is
how you ask, and which flags the verb accepts beyond the line's own summary.

This is not the resolution criterion `UNI-40` #5 covers; it is the same first-run surface, and
it is tracked separately so the resolution fix does not silently claim it.
<!-- SECTION:DESCRIPTION.END -->

## Acceptance Criteria
<!-- AC.BEGIN -->
- [ ] #1 `--help` and `-h` exit 0 with the verb's usage on **stdout**, so `universe-kit --help | less` works; an unknown argument stays exit 2 with usage on stderr, and the two paths are asserted separately.
- [ ] #2 The help names the verb's flags, including the ones the one-line usage summarises — `--notes-file` is a flag a caller cannot discover from `publish-artifacts` today.
- [ ] #3 Every verb family answers, and a test iterates the families rather than listing four verbs by hand, so a new verb cannot ship without it.
- [ ] #4 The facade forwards `--help` to the component it would route to, so `universe airlock --help` describes the consumer rather than the facade.
<!-- AC.END -->

## Definition of Done
<!-- DOD.BEGIN -->
- [ ] #1 Every acceptance criterion has a test observed failing first.
- [ ] #2 `pixi run gates` passes; the help text is generated from the same flag table the parser uses, so it cannot drift from what is accepted.
- [ ] #3 `docs/DEVELOPMENT.md` shows `--help` in the first-run sequence.
<!-- DOD.END -->
