---
id: DECISION-133
title: Python tests live under scripts/tests, beside the automation they cover
status: accepted
created_date: '2026-11-04'
updated_date: '2026-11-04'
---

## Context

`decision-15` put repository automation under `scripts/` and its pytest suites under a
top-level `TESTS/`. The split kept test files out of the importable automation tree, and it
was recorded as a decision rather than a preference, so moving them is a reversal and is
written as one.

Two things pushed the reversal. Every tool that reads Python code here (`ruff format`,
`ruff check`, `mypy`, `pytest`, coverage) had to be told the same two paths, and every new
task that formatted or type-checked the automation gained a second argument that could be
forgotten — a gate that silently stops covering half the Python is indistinguishable from a
green gate. And the two directories drifted in how they found the repository root: a test
under `TESTS/` reached the root with `parents[1]`, which is the same expression a test under
`scripts/tests/` reaches it with after one extra level, so the coupling was already implicit
in the layout and hidden by the depth.

## Decision

Python tests for repository automation live in `scripts/tests/`. They are part of the
`scripts` package (`__init__.py` present), so `mypy scripts` and `ruff check scripts` cover
them with no second argument, and coverage keeps them out of the measured source via
`omit = ["scripts/tests/*"]` rather than by sitting outside the tree.

The consequences that follow, stated so they are not rediscovered later:

- Every tool that names the tests names `scripts/tests` — the Pixi tasks no longer carry a
  separate argument for them, and `pyproject.toml` points `testpaths` there.
- A test that needs the repository root uses `Path(__file__).resolve().parents[2]`; the
  layout, not a marker file, is what makes that true.
- The top-level `TESTS/` is retired. `decision-15`'s sentence is amended in place to point
  here rather than left contradicting the tree.
- Test files stay out of coverage measurement, which is the property the old layout was
  protecting: `scripts/tests/__init__.py` is the only file in that directory the `scripts`
  coverage source would otherwise include, and `omit` excludes it.

## Alternatives

- **Leave `TESTS/` and keep passing two paths.** Rejected: the failure mode is a gate that
  quietly skips new code, and the mitigation would be a second test asserting the gate's own
  arguments — more machinery than the move.
- **A `tests/` directory at the repository root, beside `scripts/`.** Rejected: it says
  nothing about what is being tested, and this repository has Rust tests in
  `crates/*/tests/`, so a bare `tests/` would advertise a fourth test home.
