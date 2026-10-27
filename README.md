# Universe

**A Universe in Your Pocket.**

Rust toolkit for building self-hosted universes of software, data, models, and skills. Publish anywhere—and take them offline.

Universe is a standalone Rust monorepo. This checkout is a **basic,
gate-clean scaffold**: it establishes the shared core, three
version-reporting binaries, Python repository checks, agent skills, and the
Backlog design/roadmap store. It does not yet build packages, publish
channels, or restore sealed snapshots; that work is tracked in `backlog/`.

## Products

| Artifact | Role | Current scaffold behaviour |
| --- | --- | --- |
| `universe-kit` | Connected owner/producer CLI | `version`, `--version`, `-V`, optional `--json` |
| `universe-airlock` | Finite offline snapshot consumer | Same version contract |
| `universe-airlock-bootstrap` | Thin standalone restore/bootstrap executable | Same version contract |
| `universe-core` | Ecosystem-neutral shared Rust library | Product identity, version-command parsing, stdout envelope |

## Repository map

```text
crates/universe-core/               ecosystem-neutral shared library
crates/universe-kit/                producer CLI
crates/universe-airlock/            sealed offline consumer CLI
crates/universe-airlock-bootstrap/  thin native bootstrapper
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
- Backlog, repository, agent-skill, workflow, dependency-audit, and build checks.

There is no root TypeScript configuration or TypeScript repository scripting
in this standalone scaffold. Future TypeScript product packages must be added
as explicit packages with their own task wiring and Backlog decision.
