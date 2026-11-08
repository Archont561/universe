---
id: UNI-64
title: Lint the published documentation with Vale, behind a gate
status: Done
assignee: []
created_date: '2026-11-08'
updated_date: '2026-11-08'
labels:
  - tooling
  - documentation
  - gates
milestone: m-0
documentation:
  - backlog/decisions/decision-138 - Vale-lints-the-published-documentation,-and-the-Backlog-stays-outside-its-corpus.md
  - backlog/decisions/decision-121 - Declarative-repository-lint-with-alint-and-a-thin-Python-layer.md
priority: medium
type: chore
ordinal: 64000
---

## Description
<!-- SECTION:DESCRIPTION.BEGIN -->
Three linters already share this repository — Ruff for Python, alint for repository shape, and
`lint-gherkin` for the feature files — and none of them reads prose. The four root documents
and `docs/` were checked by whoever happened to read them, which is not a gate.

Vale is adopted for that question. `.vale.ini` names the styles and the vocabulary,
`.vale/styles/Universe/` holds the rules this repository writes for itself, and
`.vale/styles/config/vocabularies/Universe/accept.txt` holds the names it spells one way.

The corpus is the decision (`decision-138`). `scripts/prose_lint.py` lints the four root
documents and `docs/**/*.md`, and deliberately not `backlog/` (records quote commands in
whatever case those need) or `.agents/skills/` (vendored, and hashed by `skills-lock.json`).
The module owns discovery because `.vale.ini` cannot say "these files and not those", and it
fails closed when it finds no document at all — a gate that lints nothing is worse than none.
<!-- SECTION:DESCRIPTION.END -->

## Acceptance Criteria
<!-- AC.BEGIN -->
- [x] #1 `.vale.ini` exists, names the styles and the vocabulary, and records why spelling is switched off. — 2026-11-08: `StylesPath`, `Vocab = Universe`, `BasedOnStyles = Vale, Universe`, `Vale.Spelling = NO` with the reason in place.
- [x] #2 The project style holds rules for the spellings this repository never uses and the filler it refuses, and every entry is absent from the corpus today. — 2026-11-08: `Universe/Terms.yml` and `Universe/Filler.yml`; each token was counted across the corpus before being added.
- [x] #3 The vocabulary holds the names spelled one way here, and deliberately not the ones this corpus writes in lower case. — 2026-11-08: `accept.txt`; `Pixi`, `Conda`, `Cargo` and `Bun` are excluded with the reason.
- [x] #4 `pixi run lint-prose` runs Vale over the published documentation, fails closed on an empty corpus, and reports a missing binary as its own failure. — 2026-11-08: `pixi run lint-prose`; `scripts/prose_lint.py` exits 1 on an empty corpus and 2 when `vale` is missing or errored.
- [x] #5 The gate is a member of `check-fast`, and `repo-check` refuses its removal. — 2026-11-08: `check-fast` dependency, `CHEAP_TASKS` entry, and `validate_gate_order` failing on removal.
- [x] #6 Under GitHub Actions a finding is emitted as a check annotation, so a red pull request names the sentence. — 2026-11-08: `::error file=…,line=…,col=…` under `GITHUB_ACTIONS`; the first green run linted the corpus with Vale 3.26.0.
- [x] #7 `.alint.yml` refuses a missing `.vale.ini`, a missing vocabulary, and a style directory with no rule, each covered by a policy test under `alint-tests/prose/`. — 2026-11-08: three rules in `.alint.yml` and three policy tests in `alint-tests/prose/`, 77/77 and 8/8 passing.
- [x] #8 `AGENTS.md` states the split between Ruff, Vale and alint, and the rule that a finding is fixed in the sentence. — 2026-11-08: house rule 13 in `AGENTS.md`; `docs/DEVELOPMENT.md` gains the same split as a code-review note.
<!-- AC.END -->

## Definition of Done
<!-- DOD.BEGIN -->
- [x] #1 `scripts/prose_lint.py` is covered by `scripts/tests/test_prose_lint.py`, which needs no Vale binary: a stub answers with a report the test wrote. — 2026-11-08: twelve tests, a stub `vale` on `PATH`; no binary needed.
- [x] #2 `pixi run gates` passes, which requires the relocked Pixi environment to carry `vale`. — 2026-11-08: 572 Rust tests, 144 Python tests, alint 77/77 + 8/8, 13 feature files; green in CI with Vale installed.
<!-- DOD.END -->
