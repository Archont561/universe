---
id: doc-1
title: Monorepo and toolchain architecture
type: specification
created_date: '2026-10-26 20:15'
updated_date: '2026-10-26 20:15'
tags:
  - architecture
  - monorepo
  - ci
  - approved
---

# Monorepo and toolchain architecture

## Status

Approved target architecture for repository layout, Pixi orchestration, offline reproducibility, owner-lane CI/release workflows, and testing infrastructure (`decision-1`, `decision-3`, `decision-4`, `decision-7`, `decision-8`, `decision-9`, `decision-10`, `decision-15`). This document contains planned-state examples and is not an inventory of current files: in particular, `ci.yml`, `autorelease.yml`, and `release.yml` are not implemented. The current toolchain uses Pixi tasks, Rust crates, Python repository checks, and the authorized Bun product-package workspace. Python remains the language for repository automation.

**V1 scope and package layout update (decision-119):** the former headless `v0.1.0` / portal `v0.2.0` sequence is superseded. The integrated v1 includes Conda/static Conda publishing, Cargo crates, Python wheels, npm, Airlock consumption, and a browsable portal generated from verified pinned releases. Use [doc-111](doc-111%20-%20V1-package-layout-and-offline-readiness.md) as the canonical target package map, artifact flow, and offline readiness assessment. Kit invokes external build/test commands; it does not implement package builders.

## Repository layout

```text
universe-kit/
├── .agents/                          # Contributor agent skills (decision-9, UNI-28)
│   └── skills/
│       ├── archont561/               # Planned external managed copies (post-v1 vendoring)
│       ├── backlog/
│       ├── session/
│       ├── tdd/
│       ├── bdd/
│       ├── clean-code-principles/
│       ├── refactor/
│       ├── audit/
│       ├── grill-me/
│       ├── rattler-build/
│       ├── conda-forge/
│       ├── svelte5-unocss-ssg/       # Local SvelteKit SSG + UnoCSS invariants (present today)
│       └── cep-channel/              # Local CEP-15 / CEP-16 channel invariants (present today)
├── .github/workflows/
│   ├── ci.yml                        # PR + main gates + packaging proof
│   ├── autorelease.yml               # Convco observe on main; cut on milestone-v* / dispatch
│   ├── release.yml                   # 4-platform rattler-build + OIDC prefix.dev + GitHub Release
│   ├── relock.yml                    # pixi-sandbox Cargo, Pixi, and Bun lock refresh
│   └── publish-sandbox.yml           # GHCR offline sandbox pack publisher
├── backlog/                          # Backlog.md project management workspace
├── crates/
│   ├── package.json                  # @universe/rust — Turbo facade over Cargo
│   ├── universe-core/                # ecosystem-neutral catalog/identity/integrity contracts
│   ├── universe-assets/              # embedded generated portal/docs assets
│   │   └── VERSIONS                  # checked against Cargo.toml
│   ├── universe-cli/                 # thin `universe` dispatcher; Conda package named `universe`
│   │   └── recipe/                  # umbrella package depends on Kit + Airlock
│   ├── universe-kit/                 # producer orchestration; co-located Conda recipe
│   │   └── recipe/                  # recipe.yaml + platform variants
│   ├── universe-airlock/             # consumer/runtime; co-located Conda recipe
│   │   └── recipe/                  # recipe.yaml + platform variants
│   ├── universe-airlock-bootstrap/   # separate bootstrap artifact
│   └── universe-adapters/            # planned Conda, Cargo, Python-wheel, npm adapters
├── packages/
│   ├── config/                       # @workspace/config — shared TypeScript/UnoCSS settings
│   ├── schema/                       # @universe/conda-schema — shared schema contracts
│   └── portal/                       # @universe/portal — SvelteKit static portal scaffold
│       └── VERSIONS                  # checked against package.json
├── schemas/
│   ├── universe.schema.json          # Exported schemars schema for universe.toml
│   └── package.schema.json           # Exported schemars schema for package.toml
├── scripts/                          # Python repo-wide checks invoked via pixi run <verb>
│   ├── backlog_check.py
│   ├── repo_check.py
│   ├── skills_check.py               # Offline skill content-hash gate (--write refreshes hashes)
│   └── lib/
│       └── files.py
├── skills/                           # Post-v1 public skills.sh package (M5, decision-9)
│   └── universe-kit/
│       └── SKILL.md
├── tests/
│   └── scripts/                      # pytest suites for scripts/*.py
├── .config/
│   └── convco.yml                    # Explicit anchored Convco policy (commit-msg + PR ranges)
├── .gitignore
├── Cargo.toml
├── Cargo.lock
├── CHANGELOG.md
├── deny.toml
├── lefthook.yml
├── pixi.toml
├── pixi.lock
├── pixi-sandbox.toml
├── restore.sh                        # Optional generated developer launcher (UNI-36, if adopted)
└── skills-lock.json                  # Local skill paths + content hashes (UNI-28)
```

## Pixi configuration (`pixi.toml`)

`pixi.toml` targets `platforms = ["linux-64"]` for the reproducible dev/CI gate environment (while `release.yml` builds native 4-platform binaries via `rattler-build`).

```toml
[workspace]
name = "universe-kit"
channels = ["conda-forge", "https://prefix.dev/archont561/archont561"]
platforms = ["linux-64"]

[dependencies]
rust = ">=1.88,<2"
llvm-tools = "*"
pkg-config = "*"
cmake = "*"
ninja = "*"
c-compiler = "*"
cargo-nextest = ">=0.9"
cargo-llvm-cov = ">=0.6"
cargo-audit = ">=0.21"
cargo-deny = ">=0.16"
bun = ">=1.2,<2"
convco = ">=0.6"
actionlint = ">=1.7"
lefthook = ">=1.10"
taplo = ">=0.9"
rattler-build = ">=0.58,<0.65"
skills-ref = "*"
pixi-sandbox = { version = ">=0.9,<0.10", channel = "https://prefix.dev/archont561/archont561" }

[feature.browser.dependencies]
playwright = ">=1.50"
playwright-chromium = ">=1.50"

[feature.browser.activation.env]
PLAYWRIGHT_SKIP_BROWSER_DOWNLOAD = "1"

[environments]
default = { solve-group = "default" }
browser = { features = ["browser"], solve-group = "default" }

[tasks]
bun-install   = "bun install --frozen-lockfile"
hooks-install = { cmd = "lefthook install", depends-on = ["bun-install"] }
build         = { cmd = "cargo build --workspace --all-features --locked" }
test          = { cmd = "cargo nextest run --workspace --all-features --locked && cargo test --workspace --locked --doc && pytest" }
test-e2e      = { cmd = "future portal package task (M7; no root TypeScript workspace)" }
coverage      = { cmd = "cargo llvm-cov ... && pytest --cov=scripts ..." }
lint          = { cmd = "cargo clippy --workspace --all-targets --all-features --locked -- -D warnings && ruff check scripts tests/scripts" }
typecheck     = { cmd = "cargo check --workspace --all-targets --all-features --locked && mypy scripts tests/scripts" }
fmt           = { cmd = "cargo fmt --all && taplo fmt && ruff format scripts tests/scripts" }
fmt-check     = { cmd = "cargo fmt --all --check && taplo fmt --check && ruff format --check scripts tests/scripts" }
repo-check    = { cmd = "pixi run repo-check", depends-on = ["bun-install"] }
skills-check  = { cmd = "python -m scripts.skills_check" }
audit         = { cmd = "cargo deny check && cargo audit --deny warnings" }
package       = { cmd = "rattler-build build --recipe crates/universe-kit/recipe/recipe.yaml --variant-config crates/universe-kit/recipe/variants.yaml --output-dir output", depends-on = ["build"] }
ci            = { cmd = "bun ./the Pixi `ci` task", depends-on = ["bun-install"] }
```

Note (`decision-15`): repository automation remains Python (`python -m scripts.<module>`) under Pixi. The root Bun/Turbo workspace is product-only and package versions are checked from `VERSIONS`.

## Bun & Turbo task graph (`package.json`, `turbo.json`, `crates/package.json`)

- **Root `package.json`**:
  - `"workspaces": ["crates", "packages/*"]`
  - `"devDependencies"`: `"turbo": "^2.5.0"`, `"@archont561/bun-test-utils": "^0.1.0"`, `"@types/bun": "latest"`, `"typescript": "^5.8.0"`.
  - In the standalone scaffold, Pixi tasks call Cargo and Python tools directly; a future package graph must be added explicitly rather than assumed.
- **`turbo.json`**:
  - `build`: `{ "dependsOn": ["^build"], "outputs": ["build/**", "dist/**"] }`
  - `test`: `{ "dependsOn": ["^build"] }`
  - `coverage`: `{ "dependsOn": ["^build"], "cache": false }`
  - `typecheck`: `{ "dependsOn": ["^build"] }`
  - `lint`: `{ "dependsOn": ["^build"] }`
  - `fmt` / `fmt-check`: `{ "cache": false }`
- **`crates/package.json` (`@universe/rust`)**:
  - Declares `"devDependencies": { "@universe/portal": "workspace:*" }` so Turbo always builds `@universe/schema` -> `@universe/portal` *before* compiling Rust crates (`crates/universe-assets`).
  - Sets `"cache": false` per task via package `turbo.json` so Cargo and `cargo-nextest` own incremental state directly:
    - `build`: `cargo build --workspace --locked`
    - `test`: `cargo nextest run --workspace --all-targets --locked && cargo test --workspace --doc --locked`
    - `coverage`: `cargo llvm-cov nextest --workspace --all-targets --locked --lcov --output-path ../target/coverage/rust.lcov`
    - `lint`: `cargo clippy --workspace --all-targets --locked -- -D warnings`
    - `typecheck`: `cargo check --workspace --all-targets --locked`
    - `fmt`: `cargo fmt --all`
    - `fmt-check`: `cargo fmt --all -- --check`

## Cargo workspace & `deny.toml` (`decision-3`, `decision-7`)

- Workspace `edition = "2024"`, `rust-version = "1.88"`.
- Pure-Rust TLS policy: `deny.toml` bans `openssl-sys`, `openssl`, and `native-tls`; all HTTP/registry crates enable `rustls` features only.
- Workspace `[workspace.dependencies]` pins test crates centrally. All nine are
  declared; a crate enrolls one with `{ workspace = true }` when its own tests
  start using it, which is the commit that puts it in `Cargo.lock` and the
  vendored transport. `proptest` is enrolled in `universe-core` (property tests
  for the neutral records); the rest enroll with `UNI-10`, `UNI-12`, `UNI-14`, and
  the adapter tasks that name them.
  - `rstest = "0.27"`
  - `rstest-reuse = "0.7"`
  - `rstest-bdd = "=0.6.0"`
  - `rstest-bdd-macros = { version = "=0.6.0", features = ["strict-compile-time-validation"] }`
  - `proptest = "1.12"`
  - `wiremock = "0.6"`
  - `assert_cmd = "2.0"`
  - `predicates = "3.1"`
  - `tempfile = "3.19"`

## Offline sandbox (`pixi-sandbox.toml`) & Git hooks (`lefthook.yml`)

- **`pixi-sandbox.toml`**:
  - `schema_version = 1`
  - `[rust] cargo_vendor = true`
  - `push_paths` (publish-sandbox trigger): root `pixi.toml`, root `Cargo.toml` only; all other changes rebuild on demand via `workflow_dispatch`.
  - `relock_paths` (relock workflow, unchanged): `pixi.toml`, `pixi.lock`, `pixi-sandbox.toml`, `Cargo.toml`, `Cargo.lock`, `crates/**/Cargo.toml`, `package.json`, `bun.lock`, `crates/package.json`, `packages/*/package.json`.
- **`lefthook.yml`**:
  - `commit-msg`: `pixi run convco check --from-stdin < {1}`
  - `pre-commit`: parallel `fmt-check` (`pixi run fmt-check`), `actionlint` (`pixi run actionlint`), and `repo-check` (`pixi run repo-check`).
  - `pre-push`: `pixi run lint && pixi run typecheck && pixi run test`.

## Owner-lane workflows (`decision-8`)

1. **`ci.yml`**:
   - Triggered on `pull_request` and `push: branches: [main]` (ignoring `backlog/**`, `docs/**`, `README.md`; watching `CHANGELOG.md`).
   - Job `gates`: restores Pixi/sandbox environment, runs `pixi run ci` (`fmt-check`, `repo-check`, `schema-check`, `lint`, `typecheck`, `coverage`, `audit`) and `pixi run --environment browser test-e2e`, uploads `target/coverage/lcov.info`.
   - Job `package`: builds the portal and independent component/facade Conda recipes with external build tooling, then smoke-tests each installed executable. The integrated v1 release requires portal output from the exact pinned catalog snapshot and native registry/static-channel artifacts; local placeholder fallback is never accepted as release output (decision-119, UNI-15/26/46).
2. **`autorelease.yml`**:
   - On `main` push: **observe** mode (`convco version --bump`) written to `$GITHUB_STEP_SUMMARY`; zero commits/tags.
   - On `milestone-v*` / `m-v*` tag push or `workflow_dispatch`: runs full quality + packaging gates, computes/validates SemVer bump with `convco`, rewrites version literals across `Cargo.toml`, `package.json`, `crates/package.json`, `packages/*/package.json`, `crates/universe-kit/recipe/recipe.yaml`, `crates/universe-airlock/recipe/recipe.yaml`, `crates/universe-cli/recipe/recipe.yaml`, `skills/universe-kit/SKILL.md` (when present), and `CHANGELOG.md`, validates with `pixi run repo-check`, commits `chore(release): vX.Y.Z`, creates annotated tag `vX.Y.Z`, and dispatches `release.yml`.
3. **`release.yml`**:
   - Triggered on `v*` tag push or `workflow_dispatch` (with `tag` and optional `existing_run_id` inputs):
     - `verify`: `pixi run repo-check`.
     - `portal-bundle`: required for integrated v1. Build `packages/portal/build/` once from the verified local catalog snapshot, record/check its digest, upload it as a workflow artifact, and provide identical bytes to package and static-site jobs through `UNIVERSE_KIT_PORTAL_DIST`. Release jobs fail closed if the build is missing or mismatched; a local placeholder is not release output.
     - `build`: 4-platform matrix (`linux-64` on `ubuntu-latest`, `win-64` on `windows-latest`, `osx-64` on `macos-15-intel`, `osx-arm64` on `macos-latest`) building `.conda` packages via `rattler-build` + standalone `.tar.gz`/`.zip` archives.
     - `publish-prefix`: runs in `environment: prefix` (`id-token: write`, `contents: read`), publishing all 4 `.conda` packages via `rattler-build upload prefix -c archont561 "$pkg" --generate-attestation`.
     - `publish-github`: attaches `.conda` files, standalone archives, and `SHA256SUMS` to the GitHub Release (`contents: write`).

## Planned release-governance and supply-chain hardening (`decision-10`)

A 2026-10-27 review of the sibling `Archont561/ODM-conda` repository produced `decision-10` and tasks `UNI-20`–`UNI-23`, `UNI-28`, and `UNI-36`, which tighten the workflows above without replacing them:

- `UNI-36`: the offline developer-transport decision and, if adopted, generated launcher/config with tests.
- `UNI-20`: a fail-closed Rust release guard (exact tag/HEAD identity, clean tree, human acknowledgement flags defaulting to `false`) and a credentialed publish job that performs no checkout and no repository-script execution, uploading corresponding source before the channel binary — making `release.yml` match the zero-checkout principle already stated in `decision-8`.
- `UNI-21`/`UNI-22`: artifact identity/payload-leak audit rules and archive/source-fetch hardening (zip-slip and tar-bomb caps, HTTPS redirect revalidation, cache re-hashing, hermetic sources).
- `UNI-23`: functional tests shipped inside the artifact and run from a reinstalled prefix, plus deterministic corresponding-source bundles.
- `UNI-28`: the standalone baseline, including content-hash skill locks, Convco policy, `AGENTS.md`, and the contributor guide.
