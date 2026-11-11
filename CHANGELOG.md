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
  prefix.dev upload reported 298582 bytes. The public repodata served an older index, so no
  fresh prefix.dev read-back was recorded for that run; the three-package re-watch below
  records one for all three packages.

- The `v0.1.0` pre-release scope is widened to declare the producer CLI and the umbrella `universe` facade
  alongside Airlock. `decision-132` records the reversal; the `v0.1.0` tag was cut afterwards, as the entry below records.
- Run `43045274207` re-watched the widened checkpoint after the owner cleared the occupied
  prefix.dev variants again. All five jobs are green, and the build audited three packages:
  `universe-airlock` at `sha256:2e98b81d2155f5033bef5001dae2c6a3dbf5037d97e32e182f4af7e050bfab26`
  (298583 bytes), `universe-kit` at
  `sha256:763999409faee8b9972e7935d8e4f8d49599c4004e7eea8f80b5541304392c0b` (3489421 bytes), and the
  umbrella `universe` facade at
  `sha256:19e00dffc113f7d6d26e139b8d7cca3c72a995c20846f709b8aa4dca790f7217` (195020 bytes). The GitHub
  Release assets and a fresh cache-busted prefix.dev repodata (publication `87457`) read back
  those three digests. The plain repodata URL is still CDN-stale.
- The same run exposed a branch-store defect: three `publish --store branch` steps each
  force-pushed an orphan snapshot, so `refs/heads/channel` ended at
  `1d30ef21701497dd196eeaa1599d7ec3ee24edc6` holding only the `universe` facade. A green
  verdict is not proof that all three packages reached all three destinations. `ci render`
  now emits one command naming every package for a branch or static store; artifact stores
  keep one upload step per package. `decision-132` records the correction.
- Run `43061441347` re-watched the corrected renderer from `main` at `fa785a32` after the
  owner granted read-write-delete on the prefix.dev Repository Access entry. All five jobs are
  green on attempt 1 with no manual removal. Its build audited `universe-airlock` at
  `sha256:134f5e7dfd2d249f08195a2678e7ff6200f54b53fe04bc95c3229ff1ca8e651d` (298595 bytes),
  `universe-kit` at
  `sha256:63b958b773573e158ff30bdd682a0669311d58058a790c2783ae1c90902e9c12` (3491258 bytes), and the
  umbrella `universe` facade at
  `sha256:470f52e41e4dfaae20ce22d893cf1e098b2784c123ae623cdacab07c84667e69` (195021 bytes). The
  three forced prefix.dev uploads replaced occupied names under changed digests with no `403`,
  so this run proves force-replacement with the granted entry. The three GitHub Release assets,
  the `channel` branch at `7afdff10e32c28d368caaf07acc18a118d81ea74` (one snapshot, twelve
  files) and a fresh prefix.dev repodata read (publication `87896`) all resolve those three
  digests. The plain repodata URL and publication `87577` were briefly CDN-stale. The offline
  branch solve and a clean-container restore transcript remain open; the `v0.1.0` tag was cut afterwards, as the entry below records.
- The annotated `v0.1.0` tag is cut from `1de18891bd69f8d16b4d261cfdaa53525555eabd`; its subject is
  the first line of `docs/releases/0.1.0.md`, and `pixi run gates` passes at that commit. The
  tag-triggered `release.yml` run `43073492712` is green, including `publish / verdict`; the
  tag-triggered `universe` run `43073492717` is green.
  https://github.com/Archont561/universe/actions/runs/43073492712. The tag run's build audited
  `universe-airlock` at
  `sha256:7153aedcc8188a044524f6bc84639188e7d62491caf74d9630a4fcbd5859a3ab` (298600 bytes),
  `universe-kit` at
  `sha256:314d6941c8a1776f04954a117ea6d91cca3d97971ca6203e68d1c5a11d6233b2` (3491139 bytes), and the
  umbrella `universe` facade at
  `sha256:c4e1fc2792135a803fb3ec9a772d2fd9e2a529d323ccb792d30f7ef1636f9cec` (195023 bytes). The
  three GitHub Release assets, the `channel` branch at `eb532021ccc386c875c720e6b67138b4b30c1152`
  and a fresh cache-busted prefix.dev repodata read (publication `88268`) all resolve those three
  digests; the plain repodata URL was briefly CDN-stale. The build is still not reproducible, so
  these tag-run bytes supersede the digests recorded above for runs `43045274207` and
  `43061441347`. The offline branch solve with network denied is not proven: `pixi install
  --offline` against a clone of `channel` fails on `libgcc >=16`, a run export from `stdlib('c')`
  that the branch channel does not carry. A clean-container restore transcript is also still
  unrecorded: no container runtime is available here, and this owner-lane manifest declares no
  restore entrypoint. `UNI-59` stays open on those two criteria and DoD #3; the `v0.1.0` tag does
  not complete four-platform packaging, a sealed payload, or the integrated v1 gate.

## 0.1.0

`decision-132` makes this tag a pre-release packaging checkpoint, not the integrated v1
release described by `decision-119`. It is `linux-64` only. Cargo, Python wheels, npm, the
browsable portal, and every platform beyond `linux-64` remain out of scope.

- Three `linux-64` Conda packages are published to prefix.dev, to a static channel on a Git
  branch, and as GitHub Release assets: `universe-airlock`, `universe-kit`, and the umbrella `universe` facade.
  The installed Airlock consumer reports its version and verifies a closed payload;
  `universe-airlock-bootstrap` restores one into an immutable state root.
- The publishing workflow is rendered from `universe.toml` and drift-checked rather than
  written by hand.
- A branch channel that carries a Conda dependency edge resolves offline with Pixi in the repository's
  tests. The published `v0.1.0` branch channel does not yet resolve offline; see the Unreleased
  entry above.
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
