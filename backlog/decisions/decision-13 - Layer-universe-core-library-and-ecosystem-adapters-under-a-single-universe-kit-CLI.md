---
id: decision-13
title: Layer universe-core library and ecosystem adapters under a single universe-kit CLI (conda, python, and future ecosystems)
date: '2026-10-27 12:00'
status: accepted
---

> **Scope amendment (decision-119):** retain the neutral-core/separate-adapter
> architecture below, but replace its earlier adapter budget and sequencing.
> V1 first-class protocols are Conda, Cargo, Python wheels, and npm. Generic
> assets, Agent Skills, and STAC are deferred from v1; they are not a binding
> five-adapter budget. Cargo/Python/npm adapters are separate implementations,
> not generic profiles, because native clients require protocol-compatible
> indexes and artifacts.

## Context

`universe-kit` today has one library crate (`crates/universe-core`) and one
binary crate (`crates/universe-kit`). Adding a second ecosystem
(Python wheels / a PEP 503 simple index, then possibly Cargo, Julia, Helm,
apt) forces a choice:

- **A. Library only** — keep growing `universe-core` and ship a separate CLI
  per ecosystem (`universe-kit`, `wheel-universe`, …).
- **B. CLI only** — build one monolithic `universe-kit` binary with
  ecosystem code inside it and no reusable library.
- **C. Layered hybrid** — a generic `universe-core` library, one adapter
  crate per ecosystem, and one `universe-kit` CLI on top, with thin
  per-ecosystem shim binaries for backwards compatibility.

The current `universe-core` is already mis-layered for option A or B. Despite
its generic name it is Conda-specific:

- Its `Cargo.toml` links the whole Rattler stack (`rattler_conda_types`,
  `rattler_index`, `rattler_package_streaming`, `rattler_repodata_gateway`,
  `rattler_upload`, …). A Python-only user would link all of it.
- `schema/target.rs` enumerates Conda services in a closed enum
  (`PrefixDev`, `Anaconda`, `Quetz`, `Artifactory`, …). Adding `PyPI` there
  would pollute the Conda `universe.toml` schema — and every generated
  ts-rs/schemars binding — with another ecosystem's registry.
- `schema/platform.rs` enumerates Conda subdirs (`linux-64`, `osx-arm64`),
  which are not wheel tags (`manylinux_2_28_x86_64`, `win_arm64`), Cargo
  targets, or apt architectures.
- `schema/fragment.rs` (`FragmentIndexJson`, `FragmentAboutJson`) mirrors
  Conda's `info/index.json` / `info/about.json`.

At the same time, much of it is already generic and must not be duplicated
per ecosystem: dotted-path `Diagnostic`/`DiagnosticList`, strict
`deny_unknown_fields` manifest parsing, the watcher model
(`UpstreamProvider` already spans `github-releases | pypi | crates-io | git`),
`goblin` ELF/PE/Mach-O auditing, SPDX validation, immutable JSON fragments on
an orphan branch (decision-5), generated-workflow rendering with
`# @generated … sha256:` drift checks, and the decision-8 rule that untrusted
builds never share a job with OIDC publishing credentials.

Separate CLIs (option A) would duplicate all of that per ecosystem — the exact
`<TOOL>-conda` sprawl decision-1 was created to eliminate — and would make a
single repository that publishes **both** a `.conda` package and a wheel from
the same upstream source need two tools, two plan files, and two publish
lanes. A CLI with no library (option B) loses embeddability (a future
central discovery crawler, other tools calling the planner/auditor as a
library), loses the pure/sync testability of the schema layer, and still
grows into a god-crate.

## Decision

Adopt **option C**: `universe-core` (library) **and** `universe-kit` (CLI).
Neither replaces the other; they are two layers of the same product.

```text
                 ┌─────────────────────────────────────────────┐
 binaries        │ universe-kit   (the product users install)  │
                 │ universe-kit (thin shim, compat + recipe)│
                 └──────────────────┬──────────────────────────┘
                                    │ closed-set enum dispatch
                 ┌──────────────────▼──────────────────────────┐
 adapters        │ universe-conda   universe-python   (future: │
 (one crate per  │   rattler-build,   PEP 503/691/658,    cargo│
  ecosystem)     │   CEP-15/16        cibuildwheel      Julia│
                 └──────────────────┬──────────────────────────┘
                                    │ normalized core models only
                 ┌──────────────────▼──────────────────────────┐
 library         │ universe-core — no ecosystem deps           │
                 │ manifest envelope, diagnostics, DAG, git,   │
                 │ watcher, goblin/spdx audit primitives,      │
                 │ fragment envelope, portal view models,      │
                 │ workflow renderer, blob-target primitives   │
                 └─────────────────────────────────────────────┘
   universe-assets (portal SSG + templates, include_dir!) — unchanged
```

### 1. Crate layout

| Crate | Type | Contents |
|---|---|---|
| `crates/universe-core` | lib | Generic only (see §2). **No `rattler_*`, no PEP/Cargo crates.** |
| `crates/universe-conda` | lib | Everything Conda: `recipe.yaml` parsing, `rattler-build` orchestration, repodata/CEP-15/CEP-16 indexer, `.conda` auditor rules, Conda targets. Depends on `universe-core` + Rattler stack. |
| `crates/universe-python` | lib | `pyproject.toml`/upstream-git sources, `cibuildwheel`/`build` orchestration, wheel auditor rules (reusing core goblin primitives on binaries *inside* wheels), PEP 503/691 + PEP 658 metadata indexer, Python targets. |
| `crates/universe-assets` | lib | Unchanged: compile-time portal/template embedding (decision-4). |
| `crates/universe-kit` | lib + bin `universe-kit` | clap grammar, ecosystem detection/dispatch, `--json` output. Feature flags `conda` / `python` (default: both) select compiled adapters. |
| `crates/universe-kit` | bin (shim) | Kept. Its `recipe/recipe.yaml` and prefix.dev distribution stay valid (decision-1); `main` forwards to `universe-kit` with `--ecosystem conda` semantics. |

No dynamic plugins, no WASM/FFI plugin ABI (decision-4 stands). The adapter
set is **closed and compile-time**: `universe-kit` matches on
`EcosystemKind`. A new ecosystem is a new adapter crate + one match arm +
one feature flag — not a plugin protocol.

### 2. What stays in `universe-core`, what moves out

**Stays (already generic):** `diagnostic.rs`; manifest envelope
(`schema`, `[universe]` identity, `[pages]`); `PackageMeta` upstream/watch/
docs sections; `petgraph` DAG over normalized package names; watcher
providers; goblin header/RPATH primitives + build-path-leak scan; SPDX
checks; fragment *envelope*; portal view models (`CatalogJson`,
`PackageDetailJson`, `InstallInstructions`); workflow rendering/drift digest;
content hashing; git helpers (orphan-branch fragment store).

**Moves to `universe-conda`:** `TargetConfig` service variants
(`PrefixDev`, `Anaconda`, `Quetz`, `Artifactory`); `Platform` → renamed
`CondaSubdir`; `SkipExisting` + channel list (`BuildSection.channels`);
`FragmentIndexJson`/`FragmentAboutJson` → `CondaFragmentPayload`; all
`rattler_*` dependencies; future UNI-8 repodata diffing, UNI-10 `.conda`
streaming rules, UNI-12 CEP indexer.

**Split on a second axis — targets:** today one enum mixes *storage*
(`github-pages`, `s3`, `filesystem`) with *registry services* (`prefix-dev`,
`anaconda`). Core keeps the reusable storage structs (`S3Target`,
`FilesystemTarget`, GitHub-Releases/Pages coordinates) as building blocks;
each adapter defines **its own** `[[<ecosystem>.targets]]` enum composing
them. `universe-python` thus adds `kind = "pypi"` (OIDC Trusted Publishing)
and `kind = "github-pages"` (static simple index) without touching the Conda
schema or its generated TS types.

**Fragments:** core `FragmentJson` becomes an envelope —
`{ schema, ecosystem, name, version, artifact {filename, sha256, size},
audit: AuditSummary, test: TestSummary, payload: <adapter-tagged> }`.
The orphan-branch fragment store and "rebuild the index from KBs of JSON,
never download binaries" property (decision-5) are inherited by every
adapter for free.

**Platforms:** core normalizes to `(os, arch, libc/variant)`; adapters map to
their own vocabulary (Conda subdir ↔ wheel tag ↔ GHA runner image). One
`plan` can therefore schedule the same source onto the right matrix per
ecosystem.

### 3. Adapter contract (in `universe-core`)

Adapters operate on core-normalized models; ecosystem detail lives behind
associated types inside the adapter crate, with closed dispatch in the CLI:

```rust
// universe-core: normalized vocabulary shared by every ecosystem
pub enum EcosystemKind { Conda, Python /* future: Cargo, Julia, Helm, Apt */ }

pub struct ArtifactRecord {
    pub filename: String,
    pub sha256: String,          // 64-hex, same rule as recipe sources
    pub size_in_bytes: u64,
    pub platform: CorePlatform,  // (os, arch, variant) — adapter re-labels it
}

pub struct FragmentEnvelope<P> { // serialized as FragmentJson
    pub schema: u32,
    pub ecosystem: EcosystemKind,
    pub name: String,
    pub version: String,
    pub artifact: ArtifactRecord,
    pub audit: AuditSummary,
    pub test: Option<TestSummary>,
    pub payload: P,              // CondaFragmentPayload | PythonFragmentPayload | …
}

// Implemented by each adapter crate (pure parsing stays sync, as in UNI-30;
// I/O verbs are async under tokio; builders spawn subprocesses, decision-3).
pub trait EcosystemAdapter {
    fn kind(&self) -> EcosystemKind;
    fn lint(&self, repo: &UniverseRepo) -> DiagnosticList;
    async fn plan(&self, repo: &UniverseRepo, opts: &PlanOpts)
        -> Result<BuildPlan, CoreError>;               // leveled GHA matrix, as today
    async fn build(&self, job: &BuildJob)
        -> Result<Vec<ArtifactRecord>, CoreError>;     // untrusted lane, no secrets
    async fn test(&self, artifact: &Path) -> Result<TestSummary, CoreError>;
    fn audit(&self, artifact: &Path)
        -> Result<serde_json::Value, CoreError>;       // fragment payload, envelope by core
    fn index(&self, fragments: &[FragmentEnvelope<serde_json::Value>],
             out: &Path) -> Result<(), CoreError>;    // repodata+shards | simple/ tree | …
    fn install_snippets(&self, target_url: &Url) -> InstallInstructions; // portal tabs
}
```

A conformance suite in `universe-core` (feature `test-kit`, rstest-bdd in the
decision-7 style) is mandatory for every adapter: strict-schema roundtrip +
unknown-field refusal, ts-rs/schemars drift gate, acyclic `plan`, fragment
roundtrip, **deterministic** index output (BTreeMap ordering), dry-run
publish to a `filesystem` target.

### 4. CLI grammar — `universe-kit`, verb-first

Users install **one** tool. Verbs are the existing doc-2 grammar; the
ecosystem is *inferred from `universe.toml`*, so single-ecosystem repos see
no extra syntax, and the shim preserves today's commands exactly:

```bash
pixi global install -c https://prefix.dev/Archont561/Archont561 -c conda-forge universe-kit

universe-kit init --ecosystem conda,python --pages
universe-kit add <name> --from-github <owner/repo>   # scaffolds recipe.yaml and/or python source
universe-kit lint [--ecosystem python] [--check-workflows] [--json]
universe-kit plan  --changed-or-missing [--json]     # fans out over all ecosystems in the repo
universe-kit build --package <name> --target-platform <label>
universe-kit test  --package-file <file.conda|file.whl>
universe-kit audit <artifact>... --emit-fragment <dir>
universe-kit watch                                   # one watcher, all ecosystems (modes: pr | issue)
universe-kit publish [--target <name>]               # privileged lane only (decision-8)
universe-kit pages build                             # index(es) + portal from fragments

universe-kit plan        ≡  universe-kit plan --ecosystem conda
```

Ecosystem-first subcommands exist only for ecosystem-specific extras
(e.g. `universe-kit conda render`, `universe-kit python simple-index`).
Generated consumer workflows invoke `universe-kit <verb> --ecosystem <kind>`
pinned to a released version, keeping decision-8's zero-checkout OIDC
publisher shape unchanged.

### 5. Naming: ecosystem is `python`, not `pypi`

PyPI is one *target* (like `anaconda` is a Conda target), not the ecosystem.
The adapter is `universe-python`, artifacts are wheels/sdists, the static
index is a PEP 503/691 "simple repository", and PyPI itself is
`kind = "pypi"` under `[[python.targets]]`. No output may imply the static
index *is* PyPI.

### 6. Manifest: `universe.toml` schema 2, schema 1 keeps working

```toml
schema = 2

[universe]
name = "my-universe"
ecosystems = ["conda", "python"]
platforms = ["linux-64", "win-64", "osx-arm64"]   # core platform labels

[conda]
channels = ["conda-forge"]
recipes_dir = "recipes"

[[conda.targets]]
kind = "github-pages"

[python]
packages_dir = "python"

[[python.targets]]
kind = "github-pages"      # static PEP 503 index; wheels in Releases (absolute href + #sha256=)

[[python.targets]]
kind = "pypi"              # optional real PyPI via OIDC, release lane only

[pages]
enabled = true
title = "My Universe"
```

Schema 1 documents desugar deterministically: top-level `[[targets]]` →
`[[conda.targets]]`, `ecosystems = ["conda"]`. Existing UNI-30 diagnostics and
drift gates remain authoritative; schema 2 adds per-ecosystem sections via
the same Rust-owned ts-rs + schemars pipeline (decision-4).

## Current v1 sequencing (decision-119)

This section replaces the older staged roadmap and adapter budget elsewhere in
the historical decision text. Keep the decision's core separation rule: neutral
models in `universe-core`, protocol behavior in separate adapters, orchestration
in Kit, and consumer execution in Airlock.

1. Stabilize neutral catalog/identity/integrity contracts and conformance fixtures.
2. Keep Conda as an initial producer workflow, but publish actual package bytes
   and metadata at standard static channel paths.
3. Implement Cargo sparse-index/crate, Python wheel Simple API, and npm registry
   adapters as first-class v1 protocols, with native-client fixtures.
4. Use a thin `universe` facade for command dispatch/umbrella install; retain
   separately installable `universe-kit`, `universe-airlock`, and the distinct
   bootstrap artifact.
5. Generate the browsable portal from a pinned catalog snapshot and verified
   GitHub Release manifests/assets. The portal is not a registry.
6. Gate the integrated release on Airlock consumption and offline/connected
   evidence for all supported components. No Conda-only `v0.1.0` release is a
   substitute for the v1 acceptance target.

Kit invokes configured external native build/test commands and validates,
collects, indexes, and publishes their outputs; it does not implement package
builders. Generic assets, Agent Skills, STAC, Julia, Helm, and apt are outside
this v1 scope unless separately accepted.

## Consequences

- **One install, one mental model.** Developers learn `init → add → lint →
  plan → build → audit → publish` once and reuse it for Conda channels,
  Python indexes, and future ecosystems; one repo can publish several
  artifact kinds from one upstream, one plan, one portal.
- **No god-crate, no duplicated CLIs.** Conda users never link Python
  machinery and vice-versa (feature-gated adapters); watcher, audit,
  fragments, workflow generation exist exactly once.
- **Library stays embeddable.** `universe-core` + an adapter can be used
  without the CLI (e.g. a future discovery crawler over self-hosted
  universes) — which option B would have foreclosed.
- **Costs:** one upfront extraction refactor (UNI-25/26); closed-set dispatch
  means third-party ecosystems need a PR to this repo rather than a plugin
  drop-in (accepted trade-off, consistent with decision-4); versioning stays
  lockstep across crates under the existing repo-check (an adapter changelog
  section per ecosystem mitigates noise).
- **Rejected:** separate per-ecosystem CLIs over a shared lib (duplicated
  workflow/publish lanes, no multi-ecosystem repo story) and a
  library-less mega-CLI (untestable core, no embedding, same bloat in one
  crate instead of split by feature).
