---
id: decision-107
title: Commit checks and affected-snapshot Git hooks
date: '2026-10-25T23:49:00Z'
status: accepted
---

## Context

Formatting, linting, and pre-push tests must not stage unrelated edits, destroy partial stages, or test a dirty worktree instead of the code actually being committed/pushed.

## Decision

Validate commit messages **per commit** through Convco and `.versionrc`. Pre-commit formats/lints affected Rust scope; pre-push runs affected tests and cargo-deny dependency closures.

Read the index/ref updates directly, use rustfmt stdin, protect partial stages, and compile temporary staged/pushed snapshots. Select owning crates plus reverse dependents; shared configuration selects the workspace. Handle all outgoing commits/refs, roots/merges, deletions, new refs, rewinds, and shared-tip scope unions.

## Consequences

Clippy/tests/deny are crate/graph operations, not fictitious file-level checks. A partial file needing formatting must be corrected explicitly instead of blindly restaged. Full CI remains authoritative; missing remote-base objects require a fetch.

## Status boundary

Implemented automation with regression tests; this does not certify the future offline product. See [tooling](../docs/doc-107%20-%20Rust-workspace-and-development-tooling.md).
