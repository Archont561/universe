# Universe

<p align="center">
  <a href="https://github.com/Archont561/universe/actions/workflows/ci.yml"><img src="https://github.com/Archont561/universe/actions/workflows/ci.yml/badge.svg" alt="CI"></a>
  <a href="https://github.com/Archont561/universe/blob/main/LICENSE"><img src="https://img.shields.io/badge/license-MIT-blue" alt="MIT license"></a>
  <img src="https://img.shields.io/badge/status-pre--release%200.x-6e56cf" alt="Pre-release 0.x">
  <img src="https://img.shields.io/badge/packages-0%20published-lightgrey" alt="No package published yet">
</p>

**One `universe.toml`. Every ecosystem. Every channel they expect — and a release that restores with the network off.**

Universe is a Rust toolkit for shipping your own software, data, models and skills. You write
down what you build and where it goes, once, in one manifest. Universe runs the builds you
already have, publishes each package to the channels its ecosystem actually uses — conda to
prefix.dev, Anaconda and a static channel; wheels to PyPI and a static index; npm to npmjs and
GitHub Packages; crates to crates.io and npm — then cuts **one release whose bytes are pinned**,
which the same tool can restore on a machine with no network at all.

> Status: pre-release `0.x`. The distribution design is settled and recorded in
> [`backlog/decisions`](backlog/decisions); implementation is tracked in
> [`backlog/tasks`](backlog/tasks). **Nothing is published yet** — the scaffold answers `version`
> and nothing more. Every `0.Y.0` is a breaking boundary until `1.0.0`.

## The idea in one line

```text
universe.toml  ──►  your build commands  ──►  one GitHub Release (bytes + checksums)
                                       └──►  fan-out: conda · pypi · npm · crates · static channel
                                       └──►  catalog  ──►  airlock restore, offline, digest-verified
```

Builds happen **once**. Every channel receives the same bytes, and a release is not "done" until
every declared destination has them — a partial release is not a release.

| You declare | Universe runs | It lands on | Proven by |
| :-- | :-- | :-- | :-- |
| conda packages, per platform | your `rattler-build` command | `static` channel, `prefix-dev`, `anaconda`, GitHub Releases | re-read index + digest |
| wheels | your build command | PyPI, a PEP 503 `static` index, Releases | registry read-back |
| npm packages | your bundle command | `npmjs`, `github-packages`, Releases | `npm view` |
| crates | `cargo package` | crates.io, npm (wrapper), Releases | registry read-back |
| anything | — | GitHub Releases, always | checksums + attestations |

## One manifest

Kinds are per package, so `universe-kit` can be a conda package and a wheel at the same version,
and the npm package can be an alias of another — without a second config file anywhere.

```toml
# universe.toml — what you ship, and where it goes.

[project]
name = "universe"
version = "0.1.0"                    # one version for every artifact in the train (decision-122)
summary = "Self-hosted universes of software, data, models, and skills"

[release]
tag_pattern = "v{version}"           # one tag for the whole train
promote = "manual"                   # a prerelease attempt is a draft; promotion publishes it
changelog = "CHANGELOG.md"

# ─────────────────────────────────────────────────────────── packages: what to build ──
[[packages]]
name = "universe-kit"
kind = "conda"                       # conda | pypi | npm | cargo | r
command = "rattler-build build --recipe crates/universe-kit/recipe/recipe.yaml"
platforms = ["linux-64", "osx-arm64", "win-64"]
outputs = ["output/universe-kit/**/universe-kit-*.conda"]
smoke = "universe-kit version"       # mandatory: a package without a smoke test cannot publish
publish = ["conda-static", "conda-prefix", "conda-anaconda"]
submit = ["conda-forge"]             # a review queue, not a destination — see below

[[packages]]
name = "universe-airlock"
kind = "conda"
command = "rattler-build build --recipe crates/universe-airlock/recipe/recipe.yaml"
platforms = ["linux-64", "osx-arm64", "win-64"]
outputs = ["output/universe-airlock/**/universe-airlock-*.conda"]
smoke = "universe-airlock version"
publish = ["conda-static", "conda-prefix", "conda-anaconda"]

[[packages]]
name = "universe-kit"                # same name, different protocol
kind = "pypi"
command = "uv build packages/kit-py"
outputs = ["packages/kit-py/dist/*.whl"]
smoke = "uv pip install --find-links dist universe-kit && universe-kit version"
publish = ["pypi", "pypi-static"]

[[packages]]
name = "@archont561/universe"
kind = "npm"
command = "bun run --cwd packages/cli build"
outputs = ["packages/cli/dist/*.tgz"]
smoke = "npm install --prefix smoke ./dist/*.tgz && node smoke/bin.js version"
publish = ["npm", "github-packages"]

[[packages]]
name = "universe"                    # unscoped alias, same bytes, same version
kind = "npm"
command = "bun run --cwd packages/cli-alias build"
outputs = ["packages/cli-alias/dist/*.tgz"]
smoke = "npm install --prefix smoke ./dist/*.tgz && node smoke/bin.js version"
publish = ["npm", "github-packages"]

[[packages]]
name = "universe-core"
kind = "cargo"
command = "cargo package --workspace --locked"
outputs = ["target/package/*.crate"]
smoke = "cargo install --path crates/universe-core --locked"
publish = ["crates-io", "npm"]       # the npm copy is the wrapper built from the same crate
catalog = { title = "Universe core", license = "MIT" }

# ──────────────────────────────────────────────────────────── stores: where it goes ──
# `releases` is added to every package automatically. Opt one out with `release = false`.

[[stores]]
name = "releases"                    # the default destination for everything below
kind = "releases"
writable = true

[[stores]]
name = "conda-static"                # a real conda channel: <channel>/<subdir>/repodata.json
kind = "static"
writable = true
needs_index = true                   # we generate and upload the index, so it is reviewable
read_auth = "none"                   # consumers need no credentials to restore
verifier = "file"

[[stores]]
name = "conda-prefix"
kind = "prefix-dev"
writable = true
needs_index = true
verifier = "file"
secrets = ["PREFIX_DEV_TOKEN"]

[[stores]]
name = "conda-anaconda"
kind = "anaconda"
writable = true
needs_index = true
verifier = "registry"
secrets = ["ANACONDA_TOKEN"]

[[stores]]
name = "pypi"
kind = "pypi"
writable = true
verifier = "registry"
auth = "oidc"                        # trusted publishing: no secret held at all
                                     # the first publish of a new name needs auth = "token"

[[stores]]
name = "pypi-static"                 # PEP 503 simple/ index, no server process
kind = "static"
writable = true
needs_index = true
verifier = "file"

[[stores]]
name = "npm"
kind = "npmjs"
writable = true
verifier = "registry"                # `npm view` reads the published copy back
secrets = ["NPM_TOKEN"]

[[stores]]
name = "github-packages"
kind = "github-packages"
writable = true
read_auth = "token"                  # unlike npmjs, even consumers need a token
verifier = "registry"

[[stores]]
name = "crates-io"
kind = "crates-io"
writable = true
verifier = "registry"
auth = "oidc"                        # same shape via rust-lang/crates-io-auth-action
```

### What that manifest becomes

```text
6 packages → 3 platforms → 1 release → 9 destinations

$ universe ci render
  universe.yml          plan · validate · build(6×3) · audit · verify-artifact · verdict
  universe-publish.yml  publish/releases · publish/conda-static · publish/conda-prefix
                        publish/conda-anaconda · publish/pypi · publish/pypi-static
                        publish/npm · publish/github-packages · publish/crates-io
                        verify-index · verdict
  universe-airlock.yml  mirror/pixi · mirror/bun · mirror/cargo · render-airlock · verdict
```

Two things are worth noticing, because they are the design:

- **Publishing is split by destination, not by ecosystem.** Each job holds exactly one write
  scope and one secret. Within a destination, packages publish serially — two writers racing on
  one `repodata.json` produce an index that silently omits a package.
- **Jobs run in order of how reversible their destination is.** Releases, static channels and
  mirrors first, npm and PyPI next, crates.io last — because a crates.io version can only be
  yanked, never deleted, and a release that fails late has already written the earlier ones.
- **A file is generated only if you declared the capability.** No stores, no publish file; no
  lock adapters, no airlock file. A consumer repository gets one file.

### conda-forge is not a store

You cannot publish to conda-forge — it is a review queue with a recipe pull request. So it lives
under `submit`, and Universe opens the `staged-recipes` change for you, refuses duplicates
upstream, and reuses the branch on a retry. Everything else in the manifest above is a
destination Universe writes to directly.

## Airlock: the offline half

The same pins that produced the release restore it. Airlock never solves, never resolves, and
never reaches a channel: it replays a lockfile against mirrored artifacts and verifies every
digest.

```toml
[airlock.developer]                  # my toolchain, from my own pins
adapters = ["pixi", "cargo"]
mirror = "universe/developer"

[airlock.delivery]                   # someone else's catalog, pinned by commit
catalog = "github:Archont561/universes@8f3c1aa"
adapters = ["pixi", "bun", "cargo"]
```

| Adapter | Reads | Mirrors | Restores |
| :-- | :-- | :-- | :-- |
| **pixi** | `pixi.lock` — conda archives *and* PyPI wheels | every recorded URL with its sha256 | a prefix, with build-time paths relocated |
| **bun** | `bun.lock`, `package-lock.json` | `resolved` + `integrity` per package | `node_modules` from the mirror only |
| **cargo** | `Cargo.lock` | crate downloads with checksums | a vendor directory and `.cargo/config.toml` |

The two scopes are separate on purpose: restoring *your own toolchain* and consuming *someone
else's package set* are different provenances, and a single block would let one restore mix a
local compiler with a foreign catalog.

## Workflows are generated, and every write has a dry run

The manifest declares capabilities; the renderer writes the YAML. No hand-editing, no
`--sync-workflows` flag buried on another command.

```toml
[workflow]
checks = { format = "pixi run fmt-check", lint = "pixi run lint", typecheck = "pixi run typecheck" }
verify_artifact = true
verify_published = true
```

`workflow_dispatch` and `workflow_call` expose a **closed input schema**, filled in at render time:

| Input | Type | Default | Declared by |
| :-- | :-- | :-- | :-- |
| `dry_run` | boolean | `false` | always |
| `scope` | choice | first scope | `[[release.scope]]` |
| `package` | choice | `all` | `[[packages]].name` |
| `platform` | choice | `all` | package platforms |
| `store` | choice | `all` | writable `[[stores]]` |

A dry run skips the step that writes and **keeps the verification**, so it proves the plan
resolved, the credentials are addressed, and the bytes match the digest they would have been
checked against. Two dry runs of one input produce identical output — which is what makes
"dry-run parity" a testable claim rather than a promise.

```sh
universe ci render --dry-run    # print the files, write nothing
universe ci render              # write them
universe ci check               # fail on any drift, name the file that drifted
```

## Command surface

| Command | What it does |
| :-- | :-- |
| `universe validate` | manifest, recipes, digests, licences — offline, no network |
| `universe build` | run your declared commands in dependency order, assert output digests |
| `universe publish` | fan out to every declared destination, verifying each |
| `universe release plan\|prepare\|cut\|promote` | attempts, evidence, promotion, journal |
| `universe ci render\|check` | generate the workflow set; verify it has not drifted |
| `universe airlock restore\|verify` | replay a pinned lockfile with the network off |

### Try it today

```sh
git clone https://github.com/Archont561/universe && cd universe
bash scripts/restore.sh                 # locked Pixi environments, offline-capable
pixi run check                          # every gate: fmt, lint, types, tests, backlog, repo
pixi run universe -- version            # the binary this repo just built, dogfooded
```

Everything above the "Try it today" heading is the accepted design, not shipped behaviour. The
verbs land in the order the [roadmap](backlog/docs/roadmap/doc-4%20-%20Milestone-roadmap-and-skills-sh-rollout.md)
gives; the [DAG](backlog/docs/roadmap/doc-112%20-%20Ordered-milestone-DAG-and-sprint-train.md)
says which task blocks which.

## Editions

Universe is one product in two editions, which differ in trust posture rather than in brand or
data format (`decision-122`). One tag binds every artifact.

| Edition | Artifacts | Role |
| :-- | :-- | :-- |
| **Universe** | `universe-kit`, `universe` (umbrella) | connected owner and producer: validate, build, verify, publish |
| **Universe Airlock** | `universe-airlock` | finite offline consumer: restore and verify a pinned snapshot |

`universe-airlock-bootstrap` is a phase of the Airlock edition rather than an edition of its own:
a separate artifact because bootstrap readiness and sealed offline readiness are different
promises (`decision-115`), not a separate product.

## Repository map

```text
crates/universe-core/               ecosystem-neutral shared library
crates/universe-kit/                producer CLI
crates/universe-airlock/            sealed offline consumer CLI
crates/universe-airlock-bootstrap/  thin native bootstrapper
crates/universe-assets/             embedded templates and portal assets
crates/universe-workflow/           deterministic manifest → workflow renderer
crates/*/recipe/                    Conda build recipes
packages/                           Bun workspace: config, schema, portal
scripts/                            Python repository checks
tests/scripts/                      pytest suites for those checks
backlog/                            decisions, designs, roadmap, tasks
.agents/skills/                     contributor agent skills
```

## Documentation

- [Design records](backlog/decisions) — every decision, with the alternative that lost and why
- [Roadmap](backlog/docs/roadmap/doc-4%20-%20Milestone-roadmap-and-skills-sh-rollout.md) and the [task DAG](backlog/docs/roadmap/doc-112%20-%20Ordered-milestone-DAG-and-sprint-train.md)
- [Architecture](backlog/docs/architecture) — manifest, layout, catalogs, adapters
- Contributors start with [`AGENTS.md`](AGENTS.md) and [`backlog/README.md`](backlog/README.md)

## Develop

```sh
bash scripts/restore.sh     # locked Pixi environments and Git hooks
pixi run check              # format, lint, types, tests, backlog, repository invariants
pixi run gates              # the full CI gate, including coverage and the audit
```

House rules: gates run only through Pixi tasks; tests never mutate the live repository; fixtures
are named by the behaviour they pin.

## License

MIT. Imported package and asset components retain their upstream MIT OR Apache-2.0 license texts
under `third_party/licenses/`.
