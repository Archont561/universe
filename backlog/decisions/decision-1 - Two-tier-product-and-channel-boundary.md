---
id: decision-1
title: Two-tier product and channel boundary (archont561/archont561 ships universe-kit; consumer universes target arbitrary channels)
date: '2026-10-26 20:15'
status: accepted
---

## Context

Maintaining separate `<TOOL>-conda` repositories (such as `CloudCompare-conda` and future `ODM-conda` repos) duplicates hundreds of lines of GitHub Actions YAML, Python binary-audit scripts, lint glue, and upstream release watchers. Replacing that sprawl requires a standalone tool (`universe-kit`) that anyone can install from a Conda channel and use to manage an arbitrary universe repository.

Two distinct channel relationships exist and must never be conflated:
1. **Owner distribution (`Archont561/universe`)**: Where the `universe-kit` CLI package itself is published so users and CI runners can install it.
2. **Consumer publication (`universe.toml` in user repositories)**: Where a user's universe publishes the packages built from its `recipes/*` directory.

Hardcoding `archont561/archont561` inside `universe-kit`'s target publisher or generated consumer configs would prevent external teams, orgs, or air-gapped mirrors from adopting `universe-kit` for their own channels. Conversely, placing `https://prefix.dev/archont561/archont561` *ahead* of `conda-forge` in consumer `pixi.toml` channel lists could shadow upstream `conda-forge` dependencies.

## Decision

1. **Owner distribution channel**: `Archont561/universe` builds `.conda` artifacts for `linux-64`, `win-64`, `osx-64`, and `osx-arm64` from `crates/universe-kit/recipe/recipe.yaml` and publishes `universe-kit` to `https://prefix.dev/archont561/archont561` via OIDC Trusted Publishing (`environment: prefix`, `--generate-attestation`), alongside GitHub Release archives and `SHA256SUMS`.
2. **Arbitrary consumer channels**: `universe-kit` treats publication destinations exclusively as explicit `[[targets]]` entries in `universe.toml`:
   - `kind = "prefix-dev"` (any `channel` URL/slug on `prefix.dev` or self-hosted instance, via OIDC or `PREFIX_API_KEY`)
   - `kind = "github-pages"` (static CEP-15/CEP-16 Conda channel + documentation portal backed by per-subdir GitHub Releases on any GitHub repository)
   - `kind = "anaconda"` (any `owner[/label]` on `anaconda.org`)
   - `kind = "s3"` (any S3-compatible bucket + region/endpoint)
   - `kind = "quetz"`, `kind = "artifactory"`, and `kind = "filesystem"` (local directory channel)
   Zero references to `archont561/archont561` are permitted as defaults in target schemas, publisher adapters, or CLI flags.
3. **Consumer solver isolation**: When `universe-kit init` scaffolds a consumer repository's `pixi.toml`, it sets `channels = ["conda-forge", "https://prefix.dev/archont561/archont561"]` (`conda-forge` first) and pins the tool explicitly via `universe-kit = { version = ">=0.1,<0.2", channel = "https://prefix.dev/archont561/archont561" }` so `archont561/archont561` supplies `universe-kit` without shadowing `conda-forge` packages.

## Consequences

- Any user or CI job can bootstrap or run a universe via `pixi global install -c https://prefix.dev/archont561/archont561 -c conda-forge universe-kit` or a consumer `pixi.toml`.
- A single consumer `universe.toml` can fan out to zero, one, or multiple channels simultaneously (e.g., `prefix.dev/my-org/tools` + GitHub Pages static channel + S3 mirror).
- Unit and property tests enforce that `universe.toml` parsing and `publish-artifacts` contain no hardcoded owner channel constants.
