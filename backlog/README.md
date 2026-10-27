# Universe Backlog

This directory is the standalone Backlog store for Universe: requirements,
architecture decisions, durable specifications, milestones, and delivery
status.

## Read first

1. [`decisions/decision-13`](decisions/decision-13%20-%20Layer-universe-core-library-and-ecosystem-adapters-under-a-single-universe-kit-CLI.md) — neutral core and compiled adapters.
2. [`decisions/decision-14`](decisions/decision-14%20-%20Canonical-Universe-product-and-artifact-names.md) — product and artifact names.
3. [`decisions/decision-15`](decisions/decision-15%20-%20Standalone-toolchain-and-quality-gates.md) — Pixi/Rust/Python toolchain and the gate contract.
4. [`decisions/decision-17`](decisions/decision-17%20-%20CLI-stdout-contract-and-interactive-UX-boundaries.md) — CLI output and interaction boundaries.
5. [`docs/development/doc-202`](docs/development/doc-202%20-%20Development-guide.md) — development workflow.

## Layout

| Directory | Purpose |
| --- | --- |
| `decisions/` | Accepted/proposed/superseded architecture and product decisions |
| `docs/` | Durable specifications and design records |
| `milestones/` | Version/theme exit conditions |
| `tasks/` | `UNI-*` implementation work and status |
| `drafts/` | Unaccepted task drafts when present |

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
