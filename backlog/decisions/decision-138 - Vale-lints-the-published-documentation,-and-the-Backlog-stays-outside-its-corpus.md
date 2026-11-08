---
id: DECISION-138
title: Vale lints the published documentation, and the Backlog stays outside its corpus
status: accepted
created_date: '2026-11-08'
updated_date: '2026-11-08'
---

## Context

Three linters already share this repository and each answers one question: Ruff owns Python
style, alint owns repository shape (`decision-121`), and `lint-gherkin` owns the feature files
(`UNI-60`). Nothing owned the prose, so the four root documents and `docs/` were reviewed by
whoever happened to read them.

Vale is the tool for that question, but "adopt Vale" does not say *which documents are its
corpus*, and the corpus is the decision. This repository holds about 160 Markdown files, and
they are three different kinds of writing:

- the published documentation — `README.md`, `CONTRIBUTING.md`, `AGENTS.md`, `CHANGELOG.md`,
  and `docs/` — which is prose written for a reader;
- the Backlog, which is 151 records of checklists, dated notes, and commands quoted in
  whatever case the command needs;
- `.agents/skills/`, which is vendored from upstream and hashed in `skills-lock.json`.

## Decision

Vale lints the published documentation. `scripts/prose_lint.py` decides that corpus — the four
root documents and `docs/**/*.md` — because `.vale.ini` has no way to say "these files and not
those", and a gate that discovers its own inputs is the only kind that covers a document added
next month.

Two trees are excluded, for reasons that are about the writing rather than about effort:

- **`backlog/`.** A record is a checklist. It quotes `cargo test -p universe-core`, names
  `pixi.toml`, and dates its own acceptance criteria. A terminology rule written for README
  prose finds the wrong case in all three, and rewriting 151 historical records to satisfy it
  is churn against the record's job, which is to be accurate about what happened. The
  repository's own term for this is that a record is not a document.
- **`.agents/skills/`.** Vendored upstream text with a content hash. A finding there is a
  finding against someone else's file, and a fix breaks the hash that proves it is unmodified.

The consequences, stated so they are not rediscovered later:

- `.vale.ini` carries `StylesPath = .vale/styles`, `Vocab = Universe`, and
  `BasedOnStyles = Vale, Universe`. The project style holds this repository's rules; the
  vocabulary holds the names it spells one way.
- `Vale.Spelling` is off. The corpus is dense with product and format names — `rattler-build`,
  `minisign`, `repodata`, `CEP-15` — and a dictionary that had to carry all of them would turn
  the vocabulary into a word list instead of a statement about house style.
- `pixi run lint-prose` is a member of `check-fast`, so a prose finding reports before
  anything compiles. Under GitHub Actions the script also emits `::error` annotations, because
  a failing gate that says only "see the log" is a gate nobody reads.
- `.alint.yml` refuses a missing `.vale.ini`, a missing vocabulary, and a style directory with
  no rule file. Those three are the ways the prose gate could silently lint nothing.
- `AGENTS.md` house rule 13 states the split between the three linters, including the rule
  that a finding is fixed in the sentence and a rule is switched off only with a record.

## Alternatives

- **Lint every Markdown file.** Rejected: it reads the Backlog as prose and the vendored
  skills as ours, and the first finding on either would be a false positive that has to be
  suppressed by name — 151 times.
- **Lint only the Backlog and not the public documents.** Rejected as backwards: the documents
  a reader meets first are the ones worth a gate.
- **`errata-ai/vale-action` in CI instead of a Pixi-managed binary.** Rejected: house rule 1
  makes Pixi the entrypoint, and a gate that only exists on the runner cannot be run before a
  push. `vale` is a Pixi dependency like `alint` and `gherlint`.
- **A repository-wide `Vocab` that accepts every technical word.** Rejected: it turns the
  vocabulary into a spelling dictionary, and the entries that matter — the names with one
  correct spelling — get lost in it.

## Reopening

The Backlog joins the corpus when a record format exists that distinguishes prose from
quoted input — for example a fenced block or frontmatter field the linter can skip. Until
then, a prose rule that fires on a record is a rule aimed at the wrong tree.
