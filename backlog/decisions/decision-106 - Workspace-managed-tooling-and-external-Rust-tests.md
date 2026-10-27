---
id: decision-106
title: Workspace-managed tooling and external Rust tests
date: '2026-10-25T23:49:00Z'
status: superseded
---

## Context

The repository needs reproducible developer tooling without becoming another multi-language product workspace or mixing test implementation into Rust source modules.

## Decision

Retain exactly the Pixi features **rust, utils, python, typescript**. Keep requested tools workspace-managed, root JavaScript tooling/npm locks managed by **npm**, and automation under `scripts/` implemented in **Python**.

Expose automation as `pixi run scripts <script name>` and use named `pixi run <task>` calls in CI/Lefthook. Keep each Rust crate's tests under `tests/`, with src unit-test harnesses disabled and layout checks. The application remains Rust-based.

## Consequences

Python/Node/Backlog/skills are tooling, not alternative application implementations. Conda Rust already supplies rustfmt/Clippy. A checksum-pinned workspace-local cargo-dist asset is acceptable; a hidden global Cargo installation is not.

## Status boundary

Configured in the current repository. See the [tooling guide](../docs/doc-107%20-%20Rust-workspace-and-development-tooling.md). Broader Rust testing/UI libraries remain separate proposals.
