# Universe

<p align="center">
  <a href="https://github.com/Archont561/universe/actions/workflows/ci.yml"><img src="https://github.com/Archont561/universe/actions/workflows/ci.yml/badge.svg" alt="CI"></a>
  <a href="https://github.com/Archont561/universe/blob/main/LICENSE"><img src="https://img.shields.io/badge/license-MIT-blue" alt="MIT license"></a>
  <img src="https://img.shields.io/badge/status-pre--release%200.x-6e56cf" alt="Pre-release 0.x">
  <img src="https://img.shields.io/badge/packages-3%20pre--release-6e56cf" alt="Three pre-release packages published">
</p>

**One `universe.toml`. Every ecosystem. Every channel they expect — and a release that restores with the network off.**

Universe is a Rust toolkit for shipping your own software, data, models and skills. You write down
what you build and where it goes, once, in one manifest. Universe runs the builds you already have,
publishes each package to the channels its ecosystem actually uses, and then cuts **one release
whose bytes are pinned** — which the same tool can restore on a machine with no network at all.

> Status: pre-release `0.x`. The distribution design is settled and recorded in
> [`backlog/decisions`](backlog/decisions); implementation is tracked in
> [`backlog/tasks`](backlog/tasks). **Four `linux-64` Conda packages are published** —
> `universe-airlock`, `universe-kit`, the umbrella `universe` facade, and the separately
> installable `universe-airlock-bootstrap` — to prefix.dev, the Git branch channel, and as
> GitHub Release assets. An earlier dispatch (run
> `43061441347`) proved force-replacement with read-write-delete access. The annotated tag run
> `43073492712` is green, and the rebuilt channel's destinations read back the same digests. The
> branch channel solves offline from a clone with the network denied. This repository also
> commits a rendered restore entrypoint, `.universe-airlock/restore.sh`, pinned at a `bootstrap`
> ref that carries the bootstrapper binary, and `universe airlock pack` writes the closed payload
> that binary restores. Run against the real remote, the entrypoint clones the pin, hands over,
> and the four published packages are materialised as one activated environment. What a restore
> does *not* yet do is replay a lock into an installed prefix — that is `UNI-52` — and a
> clean-container transcript is still unrecorded, so `UNI-59` stays open.
> One platform is not the integrated v1 release, and the v1
> surfaces remain absent. Every `0.Y.0` is a breaking boundary until `1.0.0`.

## What it does

```text
universe.toml  ──►  your build commands  ──►  one GitHub Release (bytes + checksums)
                                       └──►  fan-out: conda · pypi · npm · crates · static channel
                                       └──►  catalog  ──►  airlock restore, offline, digest-verified
```

Builds happen **once**. A release is not done until every declared destination holds the same
bytes — a partial release is not a release.

| You declare | Universe runs | It lands on |
| :-- | :-- | :-- |
| conda packages, per platform | your `rattler-build` command | a `static` channel, `prefix.dev`, Anaconda, Releases |
| wheels | your build command | PyPI, a PEP 503 `static` index, Releases |
| npm packages | your bundle command | npmjs, GitHub Packages, Releases |
| crates | `cargo package` | crates.io, npm (wrapper), Releases |

## Read next

| Page | What it answers |
| :-- | :-- |
| [**The manifest**](docs/MANIFEST.md) | the full `universe.toml` — six packages, nine destinations — and how a package finds its stores |
| [**Generated workflows**](docs/WORKFLOWS.md) | what that manifest renders, the dispatch schema, and the dry run that keeps its verification |
| [**Airlock**](docs/AIRLOCK.md) | the pixi, bun and cargo adapters and what "offline" means here |
| [**Development guide**](docs/DEVELOPMENT.md) | environments, tasks, hooks, and where automation lives |

## Command surface

| Command | What it does |
| :-- | :-- |
| `universe validate` | manifest, recipes, digests, licences — offline, no network |
| `universe build` | run your declared commands in dependency order, assert output digests |
| `universe index` | render the Conda channel — CEP-15/16 metadata, README, artifacts — into a directory |
| `universe publish` | fan out to every declared destination, verifying each |
| `universe release plan\|prepare\|cut\|promote` | attempts, evidence, promotion, journal |
| `universe ci render\|check` | generate the workflow set; fail on drift |
| `universe airlock restore\|verify` | replay a pinned lockfile with the network off |

### Try it today

```sh
git clone https://github.com/Archont561/universe && cd universe
bash scripts/restore.sh                 # locked Pixi environments, offline-capable
pixi run check                          # fmt, lint, types, tests, backlog, repository invariants
pixi run universe -- --version          # the facade this repo just built, dogfooded
```

Everything above the "Try it today" heading is the accepted design, not shipped behaviour. The
verbs land in the order the [roadmap](backlog/docs/roadmap/doc-4%20-%20Milestone-roadmap-and-skills-sh-rollout.md)
gives; the [DAG](backlog/docs/roadmap/doc-112%20-%20Ordered-milestone-DAG-and-sprint-train.md)
says which task blocks which.

## Editions

Universe is one product in two editions, which differ in trust posture rather than in brand or data
format (`decision-122`). One tag binds every artifact.

| Edition | Artifacts | Role |
| :-- | :-- | :-- |
| **Universe** | `universe-kit`, `universe` (umbrella) | connected owner and producer: validate, build, verify, publish |
| **Universe Airlock** | `universe-airlock`, `universe-airlock-bootstrap` | finite offline consumer: restore and verify a pinned snapshot |

`universe-airlock-bootstrap` is a phase of the Airlock edition rather than an edition of its own: a
separate artifact because bootstrap readiness and sealed offline readiness are different promises
(`decision-115`), not a separate product.

It is **separately installable** and ships on the same release train: same version, same three
destinations, no independent version line (`UNI-55`). Its declaration is deliberately empty of run
requirements — it has **no runtime dependency** on `universe-airlock`, on the producer, or on any
registry tooling, because it is what runs before there is an environment to restore. The umbrella
`universe` facade does not depend on it either, so installing the facade never pulls a binary that
must run before the facade exists.

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
scripts/                            repository automation (Python) and restore
scripts/tests/                      pytest suites for that automation
docs/                               manifest, workflows, airlock, development
backlog/                            decisions, designs, roadmap, tasks
.agents/skills/                     contributor agent skills
```

## Documentation

- [Design records](backlog/decisions) — every decision, with the alternative that lost and why
- [Roadmap](backlog/docs/roadmap/doc-4%20-%20Milestone-roadmap-and-skills-sh-rollout.md) and the [task DAG](backlog/docs/roadmap/doc-112%20-%20Ordered-milestone-DAG-and-sprint-train.md)
- [Architecture](backlog/docs/architecture) — layout, catalogs, adapters
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

MIT. Nothing is vendored: the repository [`LICENSE`](LICENSE) is the only license text.

<!-- airlock:begin -->
## Restoring this project

A disconnected host restores this project with git and nothing else:

```sh
sh .universe-airlock/restore.sh
```

On Windows, `pwsh .universe-airlock/restore.ps1`.

The entrypoint clones the pinned branch, checks out the commit, verifies that the checkout
really is that commit, and hands over to the bootstrapper in that ref. Set
`UNIVERSE_AIRLOCK_HOME` to keep the checkout somewhere else; the default is
`$XDG_CACHE_HOME/universe-airlock` (or `~/.cache/universe-airlock`).

The entrypoint does not change the working directory: the closed payload the bootstrapper
restores belongs in this project's `.universe-airlock/payload`, and the immutable environment
it creates is `.universe-airlock/state`.

| pin | value |
| --- | --- |
| remote | `https://github.com/Archont561/universe.git` |
| branch | `bootstrap` |
| commit | `991324f5dee0cb02d18ea66e35e1e12206a35b14` |

The commit is the whole pin. It is a commit rather than a branch head so the entrypoint keeps
fetching the bytes it was rendered against, and it is checked rather than trusted.
<!-- airlock:end -->
