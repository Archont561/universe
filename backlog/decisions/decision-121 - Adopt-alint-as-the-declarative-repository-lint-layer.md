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
    cross-referenced (`backlog_check.py`);
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

## Status boundary

The config, the pinned dependency, and the gate wiring are in place and verified
against `alint 0.20.1`: `validate-config`, a green `check` over 70 rules, and 15
mutation cases. `alint-check` runs inside `check`, so `gates` and `offline-gates`
both enforce the declarative layer.

Two facts about how this arrived are worth keeping visible. First, the dependency
was added to `pixi.toml` where the network could not perform a solve, so the lock
repair is delegated to the relock workflow (decision-118) rather than hand-edited;
until that bot commits, `pixi install --locked` legitimately fails on this branch.
Second, `alint` reaches a restored developer checkout only after the transport is
republished, which the publisher does on the next main-branch run because
`pixi.toml` and `pixi.lock` are both republish triggers (`UNI-36`). A checkout
restored from the previous transport therefore fails `alint-check` until that
republish lands — the failure is honest, not silent.

Retiring the duplicated Python branches (`UNI-50` AC#6) is deliberately not part of
this change: it must happen only once the declarative layer is green in CI, so that
enforcement is never reduced by a lock that has not been repaired yet.

Adopting this layer does not change what counts as evidence: a rule that reports
green in CI is evidence, a rule authored and never run is not.
