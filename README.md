# Universe

**A Universe in Your Pocket.**

Rust toolkit for building self-hosted universes of software, data, models, and skills. Publish anywhere—and take them offline.

Universe is a standalone Rust monorepo. This checkout is a **basic,
gate-clean scaffold**: it establishes the shared core, three
version-reporting binaries, an embedded-assets crate, the Bun package workspace
for shared configuration/schema/static portal scaffolding, Python repository
checks, agent skills, and the Backlog design/roadmap store. It does not yet build
channels, publish them, or restore sealed snapshots; that work is tracked in `backlog/`.

This repository is MIT-licensed. Imported package and asset components retain their
upstream MIT OR Apache-2.0 license texts under `third_party/licenses/`.

## Editions

Universe is one product in two editions, which differ in trust posture rather than in brand
or data format (`decision-122`).

| Edition | Artefacts | Role |
| --- | --- | --- |
| **Universe** | `universe-kit`, `universe` (umbrella) | Connected owner and producer: validate, build, verify, publish |
| **Universe Airlock** | `universe-airlock` | Finite offline consumer: restore and verify a pinned snapshot |

`universe-airlock-bootstrap` is a phase of the Airlock edition rather than an edition of its
own: it is a separate artifact because bootstrap readiness and sealed offline readiness are
different promises (`decision-115`), not because it is a separate product.

| Behaviour today | Note |
| --- | --- |
| `version`, `--version`, `-V`, optional `--json` on every binary | Verified by tests |
| `universe-core` | Ecosystem-neutral shared library: product identity, version-command parsing, stdout envelope |
| `universe-assets` | Placeholder-aware `include_dir` bundle; M6 build integration remains pending |

## Repository map

```text
crates/universe-core/               ecosystem-neutral shared library
crates/universe-kit/                producer CLI
crates/universe-airlock/            sealed offline consumer CLI
crates/universe-airlock-bootstrap/  thin native bootstrapper
crates/universe-kit/recipe/          Conda build recipe for the producer CLI
crates/universe-airlock/recipe/      Conda build recipe for the offline consumer
crates/universe-assets/             embedded portal assets with a source placeholder
packages/config/                    shared TypeScript config and UnoCSS setup
packages/schema/                    shared Conda-facing schema contracts
packages/portal/                    SvelteKit static portal scaffold
packages/*/VERSIONS                 package version contract (checked against manifests)
scripts/                            Python repository checks
tests/scripts/                      pytest suites for repository checks
backlog/                            decisions, docs, milestones, tasks
.agents/skills/                     contributor agent skills
```

Start with:

1. [`backlog/README.md`](backlog/README.md);
2. [`backlog/decisions/decision-13`](backlog/decisions/decision-13%20-%20Layer-universe-core-library-and-ecosystem-adapters-under-a-single-universe-kit-CLI.md) — neutral core and compiled adapters;
3. [`backlog/decisions/decision-14`](backlog/decisions/decision-14%20-%20Canonical-Universe-product-and-artifact-names.md) — product and artifact names;
4. [`backlog/decisions/decision-15`](backlog/decisions/decision-15%20-%20Standalone-toolchain-and-quality-gates.md) — Pixi, Rust, Python, and the gate contract;
5. [`backlog/docs/development/doc-202`](backlog/docs/development/doc-202%20-%20Development-guide.md) — day-to-day development guide.

## Develop

Pixi is the only environment and task entrypoint.

```bash
pixi install --locked
pixi run setup
pixi run gates
```

`pixi run gates` is the complete local gate suite:

- Rust/TOML formatting (`cargo fmt`, Taplo) and Python formatting (Ruff);
- Rust Clippy with warnings denied and Ruff lint;
- Rust `cargo check` and strict mypy for Python scripts/tests;
- Rust nextest and doc tests, plus pytest for repository automation;
- Rust LCOV coverage with a 90% line threshold, plus Python LCOV coverage;
- Backlog, repository, agent-skill, workflow, dependency-audit, package-workspace, and build checks.

The root Bun/Turbo workspace is reserved for product packages; repository
automation remains Python under `scripts/`. `pixi run web-check` runs the
locked package checks, while the Rust/Python gates remain separately owned by
Pixi. The package and asset scaffolds do not imply the M6 portal is complete.
