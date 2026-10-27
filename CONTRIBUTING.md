# Contributing to Universe

Universe is developed as a standalone Rust monorepo. The authoritative
product plan lives in `backlog/`; this guide covers the day-to-day loop.

## Environment

Install Pixi, then materialise the locked environment:

```bash
pixi install --locked
pixi run setup
```

Rust, Python, Cargo tools, formatters, workflow linters, and commit tooling
are all provided by Pixi. Do not maintain a second host toolchain path in
docs, hooks, or CI.

There is no root TypeScript configuration and no TypeScript repository
automation. Repository checks are Python modules in `scripts/`, with pytest
suites in `tests/scripts/`.

## Commands

| Command | Purpose |
| --- | --- |
| `pixi run fmt` | Format Rust, TOML, and Python |
| `pixi run fmt-check` | Verify formatting without writing |
| `pixi run lint` | Clippy with warnings denied, plus Ruff |
| `pixi run typecheck` | `cargo check` and strict mypy |
| `pixi run test` | Rust nextest + doc tests and pytest |
| `pixi run coverage` | Rust LCOV (90% line threshold) plus Python LCOV |
| `pixi run backlog-check` | Validate Backlog IDs, statuses, filenames, and links |
| `pixi run repo-check` | Validate required files, versions, clean layout, and workflow SHAs |
| `pixi run skills-check` | Validate skill frontmatter and content hashes |
| `pixi run gates` | Run the complete local gate suite |

## Backlog workflow

- Implementation work is tracked as Markdown in `backlog/tasks/`.
- Architecture choices belong in `backlog/decisions/`.
- Durable specifications belong in `backlog/docs/`.
- Preserve existing IDs; a filename starts with the ID in its frontmatter.
- Update the task in the same change as the code: check acceptance criteria
  only when proven here, bump `updated_date`, and record evidence in notes.
- Do not add `.gitkeep` placeholders. Create a directory when it has real
  content.

## Commits

Use Conventional Commits. Keep each commit focused on one coherent change and
name the Backlog task in the body when useful. Hooks run formatting,
repository, Backlog, skills, workflow, and commit-message checks; the pre-push
hook runs the full gate suite.
