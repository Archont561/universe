---
id: UNI-14
title: Implement upstream release watcher across GitHub, PyPI, and crates.io (universe-kit watch)
status: To Do
assignee: []
created_date: '2026-10-26 20:15'
updated_date: '2026-10-26 20:15'
labels:
  - release
  - testing
milestone: m-4
dependencies:
  - UNI-13
documentation:
  - backlog/docs/architecture/doc-2 - Universe Kit-CLI-and-manifest-specification.md
  - backlog/decisions/decision-3 - Hybrid-Rattler-library-+-subprocess-architecture-and-pure-Rust-rustls-dependency-policy.md
priority: medium
type: feature
ordinal: 14000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Replace `CloudCompare-conda/scripts/check_upstream_release.py` with the multi-package Rust watcher `universe-kit watch` (`doc-2`):
- Poll `github-releases`, `github-tags`, `pypi`, and `crates-io` concurrently (bounded concurrency + exponential backoff) for every `recipes/<pkg>/package.toml` with `[upstream]`.
- Filter prereleases unless `include_prereleases = true`, apply `tag_prefix` / `version_regex` extraction, and compare normalized versions via `rattler_conda_types::Version`.
- In `mode = "pr"` (`--apply` / `--open-pr`): stream new source archive(s), compute updated `sha256` digest(s), surgically rewrite `context.version`, `source.sha256`, and reset `build.number: 0` in `recipe.yaml` preserving comments/formatting, and open or update a deduplicated branch/PR (`chore(deps): bump <pkg> to <ver>`).
- In `mode = "issue"` (`--open-issue`): search open issues with label `upstream-release` to guarantee zero duplicate issues when an upstream release already has an open notification.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 `wiremock` + `rstest-bdd` (`watch.feature`) tests verify prerelease filtering, `v`-prefix stripping, custom `version_regex`, multi-URL `sha256` updates, `build.number` reset to `0`, and duplicate PR/issue suppression.
- [ ] #2 Surgical `recipe.yaml` editing preserves existing YAML comments and Jinja `${{ ... }}` expressions, and the updated recipe passes `universe-kit lint`.
- [ ] #3 `--dry-run --json` emits a complete structured report of outdated packages without writing files or calling mutating GitHub APIs.
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 `rstest-bdd`, `proptest` version normalization tests, and `wiremock` suites pass under `cargo nextest`.
<!-- DOD:END -->
