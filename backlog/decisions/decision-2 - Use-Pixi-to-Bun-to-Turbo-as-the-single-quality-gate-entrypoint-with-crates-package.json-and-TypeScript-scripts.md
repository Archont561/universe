---
id: decision-2
title: Use Pixi -> Bun -> Turbo as the single quality-gate entrypoint with crates/package.json and TypeScript scripts
date: '2026-10-26 20:15'
status: superseded
---

> Superseded by decision-15 for the standalone scaffold: Pixi remains the
> only entrypoint, but repository automation is Python and there is no root
> TypeScript workspace. The package-graph reasoning below may be revisited
> only when real TypeScript product packages are proposed.

## Context

`universe-kit` combines a multi-crate Rust workspace (`crates/*`), generated schema packages (`packages/schema`), and a SvelteKit SSG + UnoCSS frontend (`packages/portal`), alongside repository-level verification, coverage merging, and release checks. Across `Archont561/*` repositories (`pixi-release`, `qgis-sdk`, `pixi-sandbox`, `bun-test-utils`), developer onboarding, Lefthook hooks, CI workflows, and offline `pixi-sandbox` environments rely on a uniform command contract.

Two specific failure modes arise in hybrid Rust + Bun + `pixi-sandbox` repositories if task routing is loose:
1. If Cargo builds run outside Turbo, `cargo build` can race ahead of `packages/portal`'s Vite SSG build and fail on `include_dir!("../../packages/portal/build")`.
2. If `pixi.toml` invokes `bunx scripts/foo.ts` without a leading `./`, `bunx` treats `scripts/foo.ts` as an npm registry package specifier and attempts a network fetch—breaking air-gapped `pixi-sandbox` CI runs.

## Decision

1. **Single outer entrypoint**: `pixi.toml` (`platforms = ["linux-64"]` for the reproducible dev/CI gate environment) owns all tool binaries: `rust`, `llvm-tools`, `cargo-nextest`, `cargo-llvm-cov`, `cargo-audit`, `cargo-deny`, `bun`, `convco` (commit-message lint and changelog drafting only — `decision-123`), `lefthook`, `taplo`, `rattler-build`, `skills-ref`, and `pixi-sandbox` (from `https://prefix.dev/archont561/archont561`). Humans, Lefthook, and CI invoke `pixi run <verb>` exclusively.
2. **Bun + Turbo task graph**: Root `package.json` (`"workspaces": ["crates", "packages/*"]`) delegates workspace verbs (`build`, `test`, `coverage`, `lint`, `typecheck`, `fmt-check`) to `bun x turbo run <verb>`.
3. **`crates/package.json` Cargo facade**: `crates/package.json` (`@universe/rust`) declares `"devDependencies": { "@universe/portal": "workspace:*" }` so Turbo's `"dependsOn": ["^build"]` guarantees `@universe/schema` -> `@universe/portal` -> `@universe/rust` topological execution. All tasks in `@universe/rust` set `"cache": false` in `turbo.json` so Cargo and `cargo-nextest` manage incremental compilation directly without double-caching `target/`.
4. **TypeScript `scripts/*.ts` via `bun ./scripts/<name>.ts`**: All repo-wide automation (`the Python repo check`, `ci.ts`, `coverage.ts`, `merge-coverage.ts`, `audit-deps.ts`, `lint-skills.ts`) is written in TypeScript under `scripts/`, shares repository constants (`REPO_ROOT`) via `@workspace/config` (`packages/config`), and is invoked from `pixi.toml` via `bun ./scripts/<name>.ts` so offline `pixi-sandbox` environments never query git or the npm registry.

## Consequences

- `pixi run ci` runs identically on a developer laptop, inside Lefthook hooks, and inside an air-gapped `pixi-sandbox` container.
- Cargo never compiles stale or missing SvelteKit SSG assets because Turbo enforces `@universe/portal#build` before `@universe/rust#build|test|typecheck`.
- No bash or Python glue scripts exist in the repository; `scripts/*.ts` is typechecked alongside workspace packages and tested via `bun test`.
