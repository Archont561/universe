---
id: DECISION-125
title: Two setup actions with a one-line mode difference
status: accepted
created_date: '2026-10-30'
updated_date: '2026-10-30'
---

## Context

Every generated job needs the `universe` command on `PATH` before it can do anything. That
step must work in two situations that share nothing:

- **Owner mode**, inside this repository, where the working tree is the product. There is no
  released artifact to download, and there may be none at all before the first tag.
- **Consumer mode**, in someone else's repository, where nothing may be installed and no
  compiler may be assumed.

The alternative — inlining the installation as `run:` steps — was rejected earlier for a
specific reason: it multiplies the trust-sensitive surface a template emits. One pinned
`uses:` line that the generator owns and upgrades is reviewable; four steps of shell in
every consumer repository are not.

A constraint discovered while reviewing the facade: `universe` resolves `universe-kit` and
`universe-airlock` from `PATH`, so a setup step that installed only the facade would produce
a command that exits 127 on first use. Whatever the mode, the bin directory must contain
every component.

## Decision

**Two composite actions in this repository, one output contract.**

| | `setup-dev/` (owner) | `setup/` (consumer) |
| --- | --- | --- |
| Prerequisite | a workspace checkout with `Cargo.toml` and `pixi.toml` | none |
| Installs | Pixi environment, then `cargo build --profile dev --bins` | release artifacts for the pinned commit, verified against `SHA256SUMS` |
| Network | crates.io and the developer transport | GitHub release assets only |
| Fails when | not in a workspace, or the lockfile does not match the manifest | the pin does not match an artifact set, or a checksum fails |
| Exports | `UNIVERSE_BIN_DIR`, `UNIVERSE_MODE=dev`, `UNIVERSE_COMMIT` | `UNIVERSE_BIN_DIR`, `UNIVERSE_MODE=release`, `UNIVERSE_VERSION` |

Both populate one bin directory and put it first on `PATH`, exporting `UNIVERSE_BIN_DIR`:

- **CI:** `$RUNNER_TEMP/universe/bin`. Survives steps, dies with the runner, never enters
  the repository — which matters because a fork pull request's checkout is untrusted.
- **Local:** `target/universe/bin`, populated by a Pixi install task and prepended by an
  activation script, so the same command name exists in every shell and hook.

Four supporting rules:

1. **The owner action builds once, inside the setup step.** Per-step `cargo run` was
   considered and rejected: it repeats a freshness check per step and attributes a compile
   failure to whichever step touched the binary first. The build goes through Pixi, so the
   toolchain stays pinned by the environment rather than by the runner image.
2. **The rendered workflow differs in exactly one line.** `uses: $/setup-dev` versus
   `uses: Archont561/universe/setup@<sha> # vX.Y.Z`. Everything after it is identical, and a
   test asserts that the two renders differ in that line and nothing else.
3. **Mode mismatch fails fast.** `$/setup-dev` resolves against the repository the workflow
   lives in, so a consumer repository that renders the owner reference asks for an action it
   does not have and dies at the action-fetch step rather than mid-release.
4. **The action version is the product version.** No `version` input: one release, one tag,
   one commit, exactly those artifacts. The only user-overridable value is the pinned commit,
   which the manifest may set and which must be a full commit SHA — a tag there would violate
   the repository's own action-pinning rule.

## Provenance asymmetry

The two modes are driven by different controllers, and a release record that does not say
which one is unverifiable afterwards:

| Mode | Controller identity | Recorded as |
| --- | --- | --- |
| dev | commit of the working tree | `controller: { mode: "dev", commit }` |
| release | version and asset digest from `SHA256SUMS` | `controller: { mode: "release", version, asset_sha256 }` |

A dev-mode plan refuses a dirty tree, unless explicitly allowed for a dry run. A release
driven by uncommitted code is not reproducible, and the difference between the two modes
must be in the artifact rather than in someone's memory of the run.

## Consequences

- Universe dogfoods its consumer path: the product jobs call the owner action and then plain
  `universe …`, so a defect in either action is found by this repository's own release.
- The generated workflow body contains no `cargo` and no `pixi` outside the bounded check
  list, which is what makes "the same command in both modes" mechanical rather than
  aspirational.
- Publishing artifacts is a prerequisite for `setup/`, which makes the release lane a
  dependency of consumer adoption — not of the owner lane, because the owner lane compiles.
- The earlier "the owner action does not need to build" position is superseded by rule 1.
  What changed is where the build lives, not whether it happens.

## Deliberately not decided here

The distribution of the standalone installer, channel publication, and the artifact set each
release contains. Those are `decision-126` and the release-lane specification.
