---
id: UNI-60
title: Test tooling follows the automation, and feature files are linted
status: Done
assignee: []
created_date: '2026-11-04'
updated_date: '2026-11-04'
labels:
  - tooling
  - bdd
  - gates
milestone: m-0
documentation:
  - backlog/decisions/decision-133 - Python-tests-live-under-scripts-tests,-beside-the-automation-they-cover.md
  - backlog/decisions/decision-9 - Dual-tier-Agent-Skills-strategy-and-post-v0.1.0-skills.sh-publication-gate.md
priority: medium
type: chore
ordinal: 60000
---

## Description
<!-- SECTION:DESCRIPTION.BEGIN -->
Three tooling gaps closed in one change, because each one is a gate that could pass
while checking less than its name claims.

**The pytest suites moved under `scripts/tests/`.** Every formatter, linter, and
type-checker had to be told `scripts` and `TESTS` separately, so a new task could
cover half the Python and still be green. One root, one argument. `decision-133`
records the reversal; `decision-15`'s sentence is amended in place rather than left
contradicting the tree.

**Two BDD skills are vendored from `thebushidocollective/han`.** `bdd-scenarios` and
`bdd-patterns` were installed with `npx skills add … --skill <name> --agent universal`
and are locked with their upstream provenance (`sourceType: github`, the repository,
and the path the file has there), so refreshing them is a command and not a rewrite.
`skills_check.py` learned the difference between a skill written here and one vendored:
a local skill is locked by its installed path, a vendored skill by its origin.

**`gherlint` lints every `features/*.feature` file.** The linter is a PyPI dependency
of the dev environment; `scripts/gherkin_lint.py` discovers the files and supplies the
exit code the linter's own CLI does not, and fails closed when there is nothing to
lint. It runs as `lint-gherkin` inside `check`, so `gates`, `offline-gates`, and CI all
carry it.
<!-- SECTION:DESCRIPTION.END -->

## Acceptance Criteria
<!-- AC.BEGIN -->
- [x] #1 Python tests live in `scripts/tests/`, and every tool that reads them names one path: `ruff format scripts`, `ruff check scripts`, `mypy scripts`, `pytest`, and coverage's `omit`. — 2026-11-04: `TESTS/` is gone, `pyproject.toml` points `testpaths` and coverage `omit` at the new path, and the four Pixi tasks carry a single argument.
- [x] #2 A test that needs the repository root still finds it after the move, and the suites that assert on files by path still pass. — 2026-11-04: the three suites that compute the root from `__file__` now use `parents[2]`; 116 Python tests pass.
- [x] #3 A vendored skill and a local skill can both be locked, with the checker refusing an external entry that names no owner/repository and `--write` preserving vendored provenance. — 2026-11-04: `skills_check.py` distinguishes the two, and three tests cover the acceptance, the refusal, and the provenance-preserving refresh.
- [x] #4 Every `features/*.feature` file in the repository is linted by `lint-gherkin`, which is part of `check` (and therefore of `gates` and CI). — 2026-11-04: the task runs `python -m scripts.gherkin_lint`; a test asserts the repository's own feature files are discovered and clean.
- [x] #5 The gate fails closed: a repository with no feature file, or a feature file with a finding, exits non-zero with a message naming the file and the check. — 2026-11-04: `gherkin-lint: no *.feature file …` when discovery is empty, and `file:line:column: message (name)` per finding otherwise.
<!-- AC.END -->

## Definition of Done
<!-- DOD.BEGIN -->
- [x] #1 The new script is covered by tests beside it (`scripts/tests/test_gherkin_lint.py`) and added to the gate set rather than run by hand.
- [x] #2 `pixi run gates` passes. — 2026-11-04: 17 gates.
<!-- DOD.END -->
