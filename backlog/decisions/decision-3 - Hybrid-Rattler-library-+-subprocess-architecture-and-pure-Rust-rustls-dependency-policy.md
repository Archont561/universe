---
id: decision-3
title: Hybrid Rattler library + subprocess architecture and pure-Rust rustls dependency policy
date: '2026-10-26 20:15'
status: accepted
---

## Context

`universe-kit` needs deep Conda ecosystem capabilities: parsing `MatchSpec`/`VersionSpec`/`RepoData`, querying remote channels, streaming `.conda` (ZIP + Zstd tar) archives, computing digests, indexing channels, uploading to `prefix.dev`/`anaconda.org`/`s3://`, and building + testing `recipe.yaml` v1 packages (including multi-output `staging` recipes, Jinja templates, C/C++ compiler activation, and dynamic RPATH relocation).

`prefix-dev/rattler-build` splits into two layers:
- Foundational `conda/rattler` crates (`rattler_conda_types`, `rattler_repodata_gateway`, `rattler_package_streaming`, `rattler_networking`, `rattler_index`, `rattler_upload`, `rattler_digest`), which publish stable APIs on `crates.io`.
- Internal `rattler-build` crates (`rattler_build_core`, `rattler_build_recipe`, `rattler_build_jinja`), which evolve tightly with each `rattler-build` CLI release and pull heavy transitive trees.

Furthermore, linking OpenSSL (`openssl-sys` / `native-tls`) complicates 4-platform Conda builds (`linux-64`, `win-64`, `osx-64`, `osx-arm64`) with C header/library discovery skew.

## Decision

1. **Link `conda/rattler` crates in-process** for all metadata, repodata diffing, streaming `.conda` archive extraction, digest verification, channel indexing, and artifact upload (`rattler_conda_types`, `rattler_repodata_gateway`, `rattler_package_streaming`, `rattler_networking`, `rattler_index`, `rattler_upload`, `rattler_digest`), paired with pure-Rust `goblin` (ELF64, PE32+, Mach-O binary inspection), `spdx` (license expression validation), `petgraph` (DAG planning), `reqwest` (GitHub REST v3 / GraphQL APIs without heavy generated OpenAPI SDK memory spikes), and `cc` (`cc::windows_registry` MSVC discovery on Windows).
2. **Invoke pinned `rattler-build` as a managed subprocess** for `recipe.yaml` rendering (`--render-only`), building (`build`), and isolated testing (`test --package-file`):
   - `crates/universe-kit/recipe/recipe.yaml` declares `rattler-build >=0.58,<0.65` in `requirements.run` so installing `universe-kit` guarantees a compatible `rattler-build` binary sits in the same Conda prefix (`$CONDA_PREFIX/bin` or `%CONDA_PREFIX%\Library\bin`).
   - `universe-kit` resolves `rattler-build` adjacent to `std::env::current_exe()` first (falling back to `--rattler-build-bin` / `UNIVERSE_KIT_RATTLER_BUILD` / `PATH`) and verifies its SemVer floor before execution.
   - `RattlerBuildPort` abstracts subprocess execution behind a trait in `universe-core` so unit/BDD tests inject deterministic fakes while integration tests exercise real `rattler-build`.
3. **Enforce pure-Rust TLS (`rustls`) everywhere**: Configure `rattler_*` and `reqwest` with `default-features = false` + `rustls` (`aws-lc-rs`), and explicitly ban `openssl-sys`, `openssl`, and `native-tls` in `deny.toml`.

## Consequences

- `universe-kit` achieves 100% compatibility with upstream `rattler-build` `recipe.yaml` v1 features without fighting internal crate lockstep churn.
- Binary auditing (`goblin`), Windows MSVC environment discovery (`cc::windows_registry`), DAG planning, channel indexing, and publishing execute natively in Rust with zero Python, `ldd`, `otool`, `dumpbin`, or `vswhere` subprocesses.
- Cross-platform builds across `linux-64`, `win-64`, `osx-64`, and `osx-arm64` require only `rust` and a standard `c` compiler (`ring`/`zstd-sys`).
