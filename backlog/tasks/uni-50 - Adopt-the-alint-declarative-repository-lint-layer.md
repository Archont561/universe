---
id: UNI-50
title: Adopt the alint declarative repository-lint layer
status: In Progress
assignee: []
created_date: '2026-10-29'
updated_date: '2026-10-29'
labels:
  - tooling
  - lint
  - gates
milestone: m-0
dependencies:
  - UNI-36
priority: medium
type: chore
ordinal: 50000
---

## Description
<!-- SECTION:DESCRIPTION:BEGIN -->
Promoted from the imported draft for GitHub issue #13. Replace the declarative
half of the three hand-written Python repository checkers with `alint`, a
config-driven repository linter shipped as a static binary through a PyPI wheel.
`.alint.yml` becomes the source of truth for required and forbidden paths, naming
conventions, structured-config assertions, relative-link resolution, and
cross-file relations; `pixi run alint-check` is the entrypoint.

Rules that need real computation stay in `scripts/`; decision-121 records which
and why. Enrollment in `check`/`gates` needs `alint` pinned in `pixi.lock`, which
needs a solve this environment cannot perform, so enrollment and the retirement of
the duplicated Python branches are one follow-up change.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 `.alint.yml` exists, validates, and expresses required files, forbidden paths and basenames, naming conventions, backlog link resolution, manifest/gate assertions, and the skill-registration relation.
- [x] #2 `alint check` passes on the current tree with zero violations, and every bundled ruleset that is deliberately not satisfied is switched off with a recorded reason.
- [x] #3 Every custom rule is mutation-tested: introducing its violation makes alint report that rule id.
- [x] #4 `pixi run alint-check` is the single entrypoint, and `.alint.yml` is a required repository file.
- [ ] #5 `alint` is pinned in `pixi.lock`, `alint-check` is part of `check`/`gates`, and CI enforces it.
- [ ] #6 The Python branches that alint now owns are deleted, with their tests retired or retargeted.
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [x] #1 decision-121 records the layer, what it owns, what stays in Python, and the enrollment boundary.
- [ ] #2 `pixi run gates` passes with `alint-check` included.
- [ ] #3 The developer transport is republished so a restored checkout has `alint` available offline.
<!-- DOD:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
- 2026-10-29: Authored `.alint.yml` (70 effective rules) against alint 0.20.1.
  Extends `oss-baseline`, `rust`, `node`, `monorepo`, `monorepo/cargo-workspace`,
  `ci/github-actions`, and `agent-context`. Eight bundled rules are switched off
  with the reason inline: per-directory crate/package READMEs do not fit this
  repository's single README map, and the community/security/dependency-tool,
  toolchain-pin, and Node-engine policies are maintainer or Pixi-manifest
  decisions rather than lint findings.
- 2026-10-29: Custom rules cover the standalone layout (17 required files, one rule
  per file because an existence rule's `paths` array is a disjunction; forbidden
  paths and directories; forbidden licence/notice/placeholder basenames; no
  TypeScript repository automation), manifest and gate assertions (Cargo/Pixi
  workspace version parity via `cross_file`; the audit and `audit-sync` task
  commands; `cargo_vendor` in the sandbox config), Backlog conventions (task and
  decision filename shapes, frontmatter lead, relative-link resolution under
  `backlog/**`), and skills (`registered` relation from
  `.agents/skills/*/SKILL.md` to `skills-lock.json`, plus the lock schema version).
- 2026-10-29: Verified with `alint validate-config` and a green `alint check`
  (70/70), then mutation-tested in a scratch copy: removing a required file,
  adding `NOTICE.md`, a forbidden directory, a `.gitkeep`, a `scripts/*.ts`,
  drifting the Cargo/Pixi version, breaking the audit task command, flipping
  `cargo_vendor`, renaming a Backlog task file, stripping frontmatter, adding a
  broken relative link, registering no lock entry for a new skill, and bumping the
  lock schema each reported exactly the expected rule id (15/15). One earlier
  version of the required-files rule was vacuous because a `paths` array means
  "any of these"; the per-file template instances fixed that, and the mutation
  test is what caught it.
- 2026-10-29: The layer immediately found real defects the Python checkers cannot
  see: three links in doc-110 pointed at paths that no longer exist
  (`../../pixi-sandbox.toml`, `../../restore.sh`, `../../scripts/sandbox.py`) —
  `backlog_check.py` skips targets that resolve outside `backlog/`, and the
  `scripts/sandbox.py` bullet described a module the repository no longer has.
  Links were corrected, the stale bullet replaced, and `alint fix` appended the
  missing final newline to 13 Backlog records.
- 2026-10-29: `pixi add --pypi alint` fails here: the solve needs
  `conda.anaconda.org/conda-forge/repodata_shards.msgpack.zst`, which this
  environment cannot reach. Adding only a task (not a dependency) leaves
  `pixi.lock` valid, verified with `pixi install --locked`, so the task landed and
  the dependency did not. The binary was verified out of band: the PyPI wheel's
  SHA-256 matches `39d87dec…c45bda7`, and `pixi run alint-check` was exercised
  green with the binary placed in the local environment prefix.
- 2026-10-29: Follow-up (AC#5/#6) needs, in order: relock that adds `alint` to the
  Python dependencies, `alint-check` added to `check`, deletion of the
  now-duplicated Python branches — `repo_check.validate_repo`'s required-file,
  forbidden-path, forbidden-basename, and TypeScript guards; the Backlog link walk
  in `backlog_check.validate_backlog` and its filename-shape check — with their
  tests retired or retargeted, then a transport republish so restored checkouts
  carry `alint`.
<!-- SECTION:NOTES:END -->
