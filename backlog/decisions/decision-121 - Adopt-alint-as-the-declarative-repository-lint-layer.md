---
id: decision-121
title: Adopt alint as the declarative repository-lint layer
date: '2026-10-29'
status: accepted
---

## Context

Three hand-written Python checkers (`scripts/repo_check.py`,
`scripts/skills_check.py`, `scripts/backlog_check.py`) carry the repository's
structural contract. A large part of that contract is declarative: required and
forbidden paths, filename conventions, config assertions, and cross-file
relations such as "every skill directory is registered in `skills-lock.json`".
Expressing those in Python means each rule is a loop plus a test, and every new
convention costs code.

GitHub issues #13–#18 proposed `alint` — a Rust, config-driven, language-agnostic
repository linter — as that declarative layer. This decision adopts the layer.

## Decision

- `.alint.yml` at the repository root is the declarative source of truth for
  repository structure, naming, structured-config assertions, and cross-file
  relations. It extends the bundled `oss-baseline`, `rust`, `node`, `monorepo`,
  `monorepo/cargo-workspace`, `ci/github-actions`, and `agent-context` rulesets,
  switching off the rules this repository deliberately does not satisfy, with the
  reason recorded next to each `level: off`.
- `pixi run alint-check` is the only entrypoint (house rule 1). alint is a static
  binary with no runtime dependency, so the rule layer needs no Python in the hot
  path and no plugin loading.
- Rules that need real computation stay in `scripts/`:
  - content hashing over a whole skill directory (`skills_check.py`) — the digest
    covers sorted paths plus bytes, not a single file;
  - Backlog frontmatter identity: filename-to-`id` equality, duplicate ids, and
    milestone/dependency resolution, which need every record parsed and
    cross-referenced (`backlog_check.py`). Relative-link resolution and the
    filename shape are declarative rules; only the *equality between a filename
    and its own frontmatter `id`* still needs Python, because it compares two
    fields of the same file;
  - status enums (`To Do`/`In Progress`/`Done`, decision statuses): alint's regex
    engine is RE2, which has no look-ahead, so "a `status:` line that is not one of
    the allowed values" is not expressible;
  - `VERSIONS`-to-manifest equality, because Cargo crates inherit
    `version.workspace = true` and alint's value relations skip non-literal values
    rather than failing them;
  - the offline-transport contract (`.pixi-sandbox.toml` bundle inheritance,
    publisher arguments, `push_paths` as a set, and the transitively-offline task
    graph) and the audit-task wiring, which are graph and set reasoning.
  - the standalone-history token scan, which cannot live in a config file it also
    scans.
- A rule is only kept if it is proven to bite: every custom rule was mutation-tested
  by introducing its violation in a scratch copy and confirming alint reports that
  rule id.
- Where the bundled rulesets ship a rule we depend on below `error`, the config
  overrides `level` explicitly rather than inheriting the default. That is why
  `gha-pin-actions-to-sha` carries a top-level `level: error`: the bundled default
  is `warning`, and retiring the Python SHA check against that default would have
  silently downgraded a hard gate.

## Status boundary

The config, the pinned dependency, and the gate wiring are in place and verified
against `alint 0.20.1`: `validate-config`, a green `check` over 70 rules, and 15
mutation cases. `alint-check` runs inside `check`, so `gates` and `offline-gates`
both enforce the declarative layer.

The dependency was declared in `pixi.toml` from an environment that could not
perform a solve, and the lock was repaired by the relock workflow (decision-118)
rather than hand-edited: the bot committed the lock, validated the repaired head,
and dispatched the configured CI and publisher. The publisher run republished the
developer transport, so the new environment snapshot carries the `alint` wheel and
a restored checkout runs `alint-check` offline. That is the intended division of
labour for a manifest edit made where the network is not — the lockfile is never
written by hand, and the repaired commit is validated before it is trusted.

## Retirement of the duplicated branches

`UNI-50` AC#6 retires the Python code that the declarative layer now owns. It is
sequenced after the green CI run so that enforcement is never reduced by a lock
that has not been repaired yet: `gates` is green with `alint-check` inside it
before a single branch is deleted.

What was removed:

- from `repo_check.py`: `REQUIRED_FILES`, `FORBIDDEN_PATHS`, `FORBIDDEN_BASENAMES`,
  the required/forbidden/placeholder loops, the TypeScript-automation check, and
  `extract_workflow_refs`/`is_immutable_sha` plus the workflow SHA loop;
- from `backlog_check.py`: the Markdown link walk (`extract_markdown_links`,
  `_is_external_or_anchor`, `unquote`) and the standalone filename-shape test.

What was kept, with the reason:

- the legacy-token scan — it must read every file, including `.alint.yml` itself,
  so it cannot be a rule inside that file;
- `VERSIONS` and recipe checks, because Cargo crates inherit
  `version.workspace = true` and alint's value relations skip non-literal values;
- `validate_offline_cargo_transport` and `validate_audit_gate`, which are graph and
  set reasoning;
- Backlog identity (filename-to-`id` equality, duplicate ids), status enums, and
  milestone/dependency resolution.

Each removed branch was checked against alint before deletion by introducing its
violation in a scratch copy: missing required file, absent forbidden file, absent
forbidden directory, placeholder basename, TypeScript automation, unpinned action,
a malformed Backlog filename, and a broken relative link each fail at `error` and
name the rule that replaces the Python code. The link case is a net gain: alint
also resolves links that leave `backlog/`, which the Python walk deliberately
skipped.

The deleted branches are gone from the tests too, so no test can pass by asserting
on code that no longer runs; the tests that remain target what Python still owns.

Adopting this layer does not change what counts as evidence: a rule that reports
green in CI is evidence, a rule authored and never run is not.
