# Universe Backlog

This directory is the standalone Backlog store for Universe: requirements,
architecture decisions, durable specifications, milestones, and delivery
status.

## Read first

1. [`decisions/decision-119`](decisions/decision-119%20-%20Full-multi-ecosystem-v1-and-static-catalog-distribution.md) — integrated v1 scope and distribution boundaries.
2. [`docs/architecture/doc-111`](docs/architecture/doc-111%20-%20V1-package-layout-and-offline-readiness.md) — target package map, offline dependencies/tests, and relock guidance.
3. [`docs/roadmap/doc-112`](docs/roadmap/doc-112%20-%20Ordered-milestone-DAG-and-sprint-train.md) — ordered task DAG and proposed sprint train.
4. [`decisions/decision-13`](decisions/decision-13%20-%20Layer-universe-core-library-and-ecosystem-adapters-under-a-single-universe-kit-CLI.md) — neutral core and separate ecosystem adapters.
5. [`decisions/decision-14`](decisions/decision-14%20-%20Canonical-Universe-product-and-artifact-names.md) — product and artifact names.
6. [`decisions/decision-15`](decisions/decision-15%20-%20Standalone-toolchain-and-quality-gates.md) — Pixi/Rust/Python toolchain and the gate contract.
7. [`decisions/decision-17`](decisions/decision-17%20-%20CLI-stdout-contract-and-interactive-UX-boundaries.md) — CLI output and interaction boundaries.
8. [`decisions/decision-118`](decisions/decision-118%20-%20Enable-pixi-sandbox-lock-repair-for-Cargo-Pixi-and-Bun.md) — Cargo/Pixi/Bun lock repair.
9. [`docs/development/doc-202`](docs/development/doc-202%20-%20Development-guide.md) — development workflow.
10. [`docs/development/doc-203`](docs/development/doc-203%20-%20Backlog-structure-and-planning.md) — backlog layout and milestone/task conventions.

## Layout

| Directory | Purpose |
| --- | --- |
| `decisions/` | Accepted/proposed/superseded architecture and product decisions |
| `docs/` | Durable specifications and design records |
| `milestones/` | Product/theme outcomes and observable exit conditions |
| `tasks/` | `UNI-*` implementation work, each linked to a milestone when applicable |
| `drafts/` | Unapproved task, milestone, or decision drafts |
| `archive/` | Preserved closed or superseded drafts, milestones, or tasks; create category directories when needed and never recycle IDs |

## ID rules

- Product and cross-cutting decisions/docs use the low numeric series.
- Airlock-lane decisions/docs use the 100-series.
- Engineering foundations use the 200-series docs.
- Implementation work uses `UNI-*` task IDs.
- A filename must start with the same ID recorded in its frontmatter.

## Status honesty

A task is `Done` only when its acceptance criteria are proven in this
repository. Design text, fixtures, and task descriptions are not evidence
that product behaviour exists.

Run `pixi run backlog-check` after editing Backlog files.
