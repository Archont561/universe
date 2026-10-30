# Development guide

This guide covers the current Rust/Python repository and the product-only Bun workspace. Pixi is the outer environment/task entrypoint; Python remains the language for repository automation.

## Quick start

```bash
pixi install --locked
pixi run setup
pixi run gates
```

The optional VS Code Dev Container uses the pinned Pixi image and runs the same setup on first creation. If the lock-repair bot has refreshed the lockfiles on your pull request, `pixi install --locked` materializes the complete environment, including Bun. The pre-push hook runs `CARGO_NET_OFFLINE=true pixi run --offline --as-is offline-gates`, which uses the sandbox's vendored Cargo sources. It deliberately excludes the Bun workspace install and advisory-database refresh; CI's `pixi run gates` remains the complete online suite.

## Daily workflow

1. Start from a clean branch and run `pixi run gates` before changing code.
2. Make a focused change and add tests at the affected boundary.
3. Run the smallest relevant named Pixi task, then rerun `pixi run gates` before opening a pull request.
4. Use a conventional commit message; the Git hook and CI validate the configured commit policy.

Useful tasks:

```bash
pixi run fmt
pixi run fmt-check
pixi run lint
pixi run typecheck
pixi run test
pixi run web-check
pixi run web-build
pixi run audit
pixi run gates
```

For focused Rust tests, use the Pixi environment, for example:

```bash
pixi run --locked cargo test -p universe-core
pixi run --locked cargo test -p universe-assets
```

To build the co-located Conda recipes, install the separate packaging environment and invoke the named tasks:

```bash
pixi install --locked --environment packaging
pixi run --environment packaging package-kit
pixi run --environment packaging package-airlock
```

Each recipe builds only its corresponding workspace binary and smoke-tests its version command. Airlock has no package runtime dependency on Conda or a network service.

## Workspace boundaries

- `crates/universe-core` owns ecosystem-neutral identity, manifest, plan, evidence, and output contracts. Do not add Conda/Rattler or Cargo registry implementation details to it.
- Put real Conda channel/repodata behavior in a Conda adapter and Cargo registry/index behavior in a Cargo adapter. The proposed `universe-adapter-conda` and `universe-adapter-cargo` crates are a good eventual split, but create them when there is a concrete API and behavior to test—not as empty dependency holders.
- `crates/universe-assets` embeds the generated static portal when available and retains its committed placeholder as a fresh-checkout fallback.
- `packages/config`, `packages/schema`, and `packages/portal` are product packages in the Bun/Turbo workspace. They are not repository-maintenance scripts. Every package's `VERSIONS` file must match its manifest; `repo-check` enforces that contract.
- `universe-airlock` is a finite offline consumer. Developer transport and Cargo vendoring do not constitute sealed Airlock snapshots or Airlock product behavior.

## Adding or changing dependencies

- Edit workspace requirements in the relevant Rust manifest and keep `universe-core` neutral.
- The PR relock bot validates and repairs `Cargo.lock`, `pixi.lock`, and `bun.lock` on the connected runner. Review its commit and require the repaired-head CI check before merging. Do not copy another workspace's lockfile wholesale.
- Cargo vendoring is enabled for the developer sandbox bundle. It carries the locked Cargo source closure, not npm payloads; Bun package installation may still require access to the npm registry.
- Keep advisory exceptions narrow, documented with a removal condition, and mirrored in the reviewed dependency policy where necessary.

## Automation and scripts

The upstream package examples informed the Cargo Turbo facade and package task graph. Their TypeScript CI/audit/coverage/repository scripts are not copied wholesale: this repository already has tested Python equivalents and named Pixi tasks. Add automation in `scripts/` as Python, register it as a Pixi task, and test it under `TESTS/`. Shell is reserved for the restore launcher and generated shell-specific templates; small CI helpers should use the Pixi/Python task entrypoint.

The sandbox publisher only transports a developer environment. It is separate from the sealed-snapshot format, consumer restore lifecycle, and any future Airlock release pipeline.

## Pull requests and CI

CI installs the locked default Pixi environment and runs `pixi run gates`. The relock workflow may commit corrected lockfiles to a same-repository pull-request branch when a manifest changes. Fork pull requests cannot receive the write token needed for that repair; refresh the lockfiles through a trusted same-repository PR instead.

Before merging, review the task/backlog updates, the Cargo/Pixi/Bun lock diff, the Cargo vendor policy, and the CI verdict on the repaired commit. The portal remains scaffold work until its M6 behavior and integration acceptance criteria pass.
