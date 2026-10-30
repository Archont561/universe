# Universe

<p align="center">
  <a href="https://github.com/Archont561/universe/actions/workflows/ci.yml"><img src="https://github.com/Archont561/universe/actions/workflows/ci.yml/badge.svg" alt="CI"></a>
  <a href="https://github.com/Archont561/universe/blob/main/LICENSE"><img src="https://img.shields.io/badge/license-MIT-blue" alt="MIT license"></a>
  <img src="https://img.shields.io/badge/status-pre--release%200.x-6e56cf" alt="Pre-release 0.x">
  <img src="https://img.shields.io/badge/packages-0%20published-lightgrey" alt="No package published yet">
</p>

**One `universe.toml`. Every ecosystem. Every channel they expect — and a release that restores with the network off.**

Universe is a Rust toolkit for shipping your own software, data, models and skills. You write down
what you build and where it goes, once, in one manifest. Universe runs the builds you already have,
publishes each package to the channels its ecosystem actually uses, and then cuts **one release
whose bytes are pinned** — which the same tool can restore on a machine with no network at all.

> Status: pre-release `0.x`. The distribution design is settled and recorded in
> [`backlog/decisions`](backlog/decisions); implementation is tracked in
> [`backlog/tasks`](backlog/tasks). **Nothing is published yet** — the scaffold answers `version`
> and nothing more. Every `0.Y.0` is a breaking boundary until `1.0.0`.

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
| `universe publish` | fan out to every declared destination, verifying each |
| `universe release plan\|prepare\|cut\|promote` | attempts, evidence, promotion, journal |
| `universe ci render\|check` | generate the workflow set; fail on drift |
| `universe airlock restore\|verify` | replay a pinned lockfile with the network off |

### Try it today

```sh
git clone https://github.com/Archont561/universe && cd universe
bash scripts/restore.sh                 # locked Pixi environments, offline-capable
pixi run check                          # fmt, lint, types, tests, backlog, repository invariants
pixi run universe -- version            # the binary this repo just built, dogfooded
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
| **Universe Airlock** | `universe-airlock` | finite offline consumer: restore and verify a pinned snapshot |

`universe-airlock-bootstrap` is a phase of the Airlock edition rather than an edition of its own: a
separate artifact because bootstrap readiness and sealed offline readiness are different promises
(`decision-115`), not a separate product.

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
TESTS/                              pytest suites for that automation
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
