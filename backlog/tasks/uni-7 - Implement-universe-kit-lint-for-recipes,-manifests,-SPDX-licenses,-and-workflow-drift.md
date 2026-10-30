---
id: UNI-7
title: Implement universe-kit lint for recipes, manifests, SPDX licenses, and workflow drift
status: To Do
assignee: []
created_date: '2026-10-26 20:15'
updated_date: '2026-10-31'
labels:
  - audit
  - architecture
  - testing
milestone: m-2
dependencies:
  - UNI-30
documentation:
  - backlog/docs/architecture/doc-2 - Universe Kit-CLI-and-manifest-specification.md
  - backlog/decisions/decision-3 - Hybrid-Rattler-library-+-subprocess-architecture-and-pure-Rust-rustls-dependency-policy.md
priority: high
type: feature
ordinal: 7000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Port and generalize all static recipe & repository lint rules (`CloudCompare-conda/scripts/lint.py`) into `crates/universe-core::lint` and expose `universe-kit lint` (`doc-2`):
- Parse `recipe.yaml` v1 files with `marked-yaml` so every diagnostic reports exact file path, line, and column.
- Enforce SHA-256 presence/format on all URL sources and immutable commit `rev` hashes on all `git` sources (refusing unpinned branches/tags unless explicitly overridden).
- Validate `about.license` with `spdx::Expression::parse`, enforce `about.license_file` presence when `audit.enforce_license_file = true`, validate `rattler_conda_types::Version` on `context.version` / `package.version`, and check `build.number >= 0`.
- Validate `universe.toml` and `recipes/*/package.toml` schemas, plus optional `--check-workflows` `# @generated` SHA-256 header drift verification.
- Support both human-readable diagnostics (`miette`/annotated spans) and `--json` output.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 `universe-kit lint` catches missing `sha256`, mutable git `branch`/`tag` without `rev`, invalid SPDX license strings, missing `license_file`, negative `build.number`, and unknown `universe.toml` / `package.toml` keys with line/key locations.
- [ ] #2 Multi-output `staging` recipes (`outputs:` arrays in `recipe.yaml` v1) are parsed and validated accurately.
- [ ] #3 `--json` emits deterministic structured diagnostics suitable for CI annotations.
- [ ] #4 Version parity is enforced across `Cargo.toml`, `universe.toml`, every recipe's `context.version`, and the `CHANGELOG.md` section for the version; a release with no section for its version is refused (`decision-123`).
- [ ] #5 The workflow drift check covers every rendered file rather than one, and names which file diverged (`decision-124`).
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 Parametrized `rstest` suites, `proptest` SPDX/digest fuzzing, and `rstest-bdd` Gherkin scenarios (`crates/universe-core/tests/features/lint.feature`) pass under `cargo nextest`.
<!-- DOD:END -->
