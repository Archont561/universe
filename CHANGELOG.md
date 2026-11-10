# Changelog

All notable changes to Universe are recorded here.

## Unreleased

- The rendered `release.yml` passes `--force` to its `prefix-dev` and `releases` artifact
  uploads, so a rebuild under an occupied package name asks those destinations to replace the
  changed digest. The branch store is unchanged. `decision-132` records this as a scoped
  reversal of the no-overwrite release semantics; it does not make the build reproducible.
  prefix.dev accepts `?force=true` only from a Repository Access entry with read-write-delete.
- Run `42920755513` attempt 1 answered `403 Not authorized` on the forced prefix.dev upload.
  The owner removed the occupied package variant manually and reran the failed jobs; attempt 2
  uploaded the same artifact and `publish / verdict` is green. This does not prove that the
  current Repository Access entry can force-replace an occupied filename.
- All three publish jobs consumed one artifact (`sha256:4f050f05a7de76ac3433fcfd7e7fa1c64d95ca784306d913026efd16548a07ff`)
  whose audit fragment pins the package at `sha256:17337dbb208182060e4ccd002473ae0bf64482a1b0fe3813585124569a982ec8`,
  size 298582. The GitHub Release asset and branch repodata read back with that digest; the
  prefix.dev upload reported 298582 bytes. The public repodata still serves an older index, so
  a package API read-back remains to be recorded.

## 0.1.0

`decision-132` makes this tag a pre-release packaging checkpoint, not the integrated v1
release described by `decision-119`. It is `linux-64` only. Cargo, Python wheels, npm, the
browsable portal, the umbrella facade, and every platform beyond `linux-64` remain out of
scope.

- `universe-airlock` is published as a Conda package to prefix.dev, to a static channel on a
  Git branch, and as a GitHub Release asset. The installed consumer reports its version and
  verifies a closed payload; `universe-airlock-bootstrap` restores one into an immutable
  state root.
- The publishing workflow is rendered from `universe.toml` and drift-checked rather than
  written by hand.
- A branch channel that carries a Conda dependency edge resolves offline with Pixi.
- A repository check keeps this section, the workspace version, the recipe version, and the
  newest `v*` tag in agreement, so a later release cannot drift by memory.
- Created the standalone Universe monorepo scaffold with `universe-core`,
  `universe-kit`, `universe-airlock`, and `universe-airlock-bootstrap`.
- Added the Backlog design, decision, milestone, and task store.
- Added Pixi quality gates covering Rust and Python formatting, linting,
  type checking, tests, coverage, Backlog/repository/skills validation,
  workflow lint, dependency audit, and builds.
- Kept repository automation in Python; there is no root TypeScript
  configuration or TypeScript scripting surface.
