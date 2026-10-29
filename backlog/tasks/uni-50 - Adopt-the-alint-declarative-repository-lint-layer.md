---
id: UNI-50
title: Adopt the alint declarative repository-lint layer
status: Done
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
- [x] #5 `alint` is pinned in `pixi.lock`, `alint-check` is part of `check`/`gates`, and CI enforces it.
- [x] #6 The Python branches that alint now owns are deleted, with their tests retired or retargeted.
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [x] #1 decision-121 records the layer, what it owns, what stays in Python, and the enrollment boundary.
- [x] #2 `pixi run gates` passes with `alint-check` included.
- [x] #3 The developer transport is republished so a restored checkout has `alint` available offline.
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
- 2026-10-29: AC#5 landed as a dependency change plus gate wiring: `alint` is
  declared under `[feature.dev.pypi-dependencies]` and `alint-check` is now a
  dependency of `check`, so `gates` and `offline-gates` both run it. The lock
  repair is delegated to the relock workflow: this environment cannot solve the
  environment (no `conda.anaconda.org`), so `pixi.toml` was edited and the bot
  commits `pixi.lock`. `pixi.lock` is v7 with no PyPI entries yet, so this adds the
  first one. A restored checkout gains `alint` with the next main-branch transport
  publish, because `pixi.toml`/`pixi.lock` are republish triggers.
- 2026-10-29: Follow-up (AC#6) needs, in order: relock that adds `alint` to the
  Python dependencies, `alint-check` added to `check`, deletion of the
  now-duplicated Python branches — `repo_check.validate_repo`'s required-file,
  forbidden-path, forbidden-basename, and TypeScript guards; the Backlog link walk
  in `backlog_check.validate_backlog` and its filename-shape check — with their
  tests retired or retargeted, then a transport republish so restored checkouts
  carry `alint`.
- 2026-10-29: The relock bot repaired the lock in pull request #19. Guard failed on
  `7cf7edf` as designed (manifest edited without a solve), the relock job ran
  `pixi lock`, committed `dfa43a0 chore(lock): refresh lockfiles for #19` as
  `pixi-sandbox[bot]`, and validated the repaired head itself
  (`pixi lock --check`, `cargo fetch --locked`, `bun install --frozen-lockfile`).
  The repair added 111 lines to `pixi.lock` and pinned
  `alint-0.20.1-py3-none-manylinux_2_17_x86_64…whl` with SHA-256
  `39d87dec0138df1d96638f60fa89e66b30afba6a2c5ee29d7a87233b2c45bda7` — the same
  digest verified out of band against PyPI before the dependency was declared.
  `alint` is the lock's first PyPI entry (lock schema v7).
- 2026-10-29: The repaired head `dfa43a0` is green on every dispatched verdict:
  `lock guard` success, `gates` success (ci dispatch 40853225061, 1m0s),
  `Pack, verify, and publish linux-64` success (40853224909), and the
  `pixi-sandbox relock validation` Check Run success. Because `check` depends on
  `alint-check` and `gates` runs `check`, a green `gates` on the repaired head is
  the proof that the pinned binary installed and `alint check` passed there; a
  missing binary would have failed the task with a non-zero exit.
- 2026-10-29: DOD#3 landed as a side effect of the relay: the publisher dispatched
  by the relock job republished the transport to snapshot
  `2026-10-29T09:55:16Z` (branch head `f81aac4`, source commit `dfa43a0`). The new
  manifest lists 87 blobs where the previous one had 86, and the extra blob is
  `envs/default/pack/pypi/alint-0.20.1-py3-none-manylinux_2_17_x86_64…whl`
  (8 241 448 bytes), so a restored checkout installs `alint` without network.
  Caveat recorded rather than hidden: the transport branch now points at the pull
  request head, so it tracks `dfa43a0` until the next main-branch publish.
- 2026-10-29: AC#6 is kept as its own change so a review can see the enforcement
  move from Python to `.alint.yml` rather than infer it from a lockfile diff. It
  starts from a green `gates` on `4a17bff`, so no branch is deleted while the
  replacement is unproven.

  Two gaps were found by probing before deleting anything, and both were closed in
  the config rather than left to the deletion:

  - `.alint.yml` was not among the required files, so removing `REQUIRED_FILES`
    would have stopped requiring the config itself. Added
    `universe-required-alint-config`.
  - `gha-pin-actions-to-sha` is a bundled `warning`, while the Python check failed
    the build. Added a top-level `level: error` override. Without it the retirement
    would have silently downgraded a hard gate to a warning — the one failure mode
    that leaves CI green while enforcement disappears.

  The config is now 71 rules. Deleted: from `repo_check.py`, `REQUIRED_FILES`,
  `FORBIDDEN_PATHS`, `FORBIDDEN_BASENAMES`, the required/forbidden/placeholder
  loops, the TypeScript-automation check, and `extract_workflow_refs` /
  `is_immutable_sha` with the workflow SHA loop; from `backlog_check.py`, the
  Markdown link walk (`extract_markdown_links`, `_is_external_or_anchor`,
  `unquote`) and the standalone filename-shape test — 213 lines removed against 64
  added. Kept, with reasons: the legacy-token scan (it must read `.alint.yml`
  itself), `VERSIONS` and recipe equality (Cargo inheritance defeats alint's value
  relations), the offline-transport and audit-gate validators (graph and set
  reasoning), and Backlog identity/status/milestone/dependency resolution.

  Every retired branch was proven to be replaced before deletion, by introducing
  its violation in a scratch copy of the tree and confirming that alint fails at
  `error` and names the successor rule: missing required file →
  `universe-required-readme`; absent forbidden file → `universe-forbidden-files`;
  absent forbidden directory → `universe-forbidden-directories`; placeholder
  basename → `universe-forbidden-basenames`; TypeScript automation →
  `universe-no-typescript-automation`; unpinned action →
  `gha-pin-actions-to-sha`; malformed Backlog filename → `backlog-task-filename`;
  broken relative link → `backlog-relative-links-resolve`. The link case is a net
  gain: alint also resolves links that leave `backlog/`, including one that pointed
  at a missing file under `scripts/`, which the Python walk skipped by construction.

  Tests were retargeted, not deleted wholesale: the shape assertions were replaced
  by assertions on what Python still owns (legacy tokens, unreadable paths), the
  `main` failure path now trips the token scan instead of a missing required file,
  and the parse test regained coverage of `extract_frontmatter_list` that the
  rewrite had dropped. `ruff`, the full pytest suite, `repo-check`,
  `backlog-check`, `skills-check`, and `alint check` (71/71) all pass.
- 2026-10-29: AC#6 verified in CI on `a12dc43`: `gates` success in 56s and
  `lock guard` success in 19s (run 40854528710 / 40854528556); `relock` and
  `relock verdict` skipped because the lock did not drift. The `gates` job's step
  `Run gates` succeeded, and that step runs `check`, which depends on
  `alint-check` — so the 71-rule layer passed on the tree with the Python branches
  actually removed. That is the point of the ordering: the retirement commit is
  validated by the replacement, not by the code it deletes.
- 2026-10-29: Step-level detail came from the jobs API, not the log archive. The
  log archive is unreachable from this sandbox for network reasons, not
  credentials: a freshly minted token returns `200` from `api.github.com`, while
  the endpoint's `302` target `results-receiver.actions.githubusercontent.com`
  fails at TLS handshake (`SSL_ERROR_SYSCALL`). Job/step conclusions are fetched
  over `api.github.com`, so they are available; log text is not.
<!-- SECTION:NOTES:END -->
