---
id: UNI-56
title: Add owner and consumer setup actions
status: To Do
assignee: []
created_date: '2026-10-31'
updated_date: '2026-10-31'
labels:
  - ci
  - release
  - packaging
milestone: m-4
dependencies:
  - UNI-34
  - UNI-55
documentation:
  - backlog/decisions/decision-125 - Two-setup-actions-with-a-one-line-mode-difference.md
  - backlog/decisions/decision-124 - Generated-workflows-are-split-by-trigger-family.md
priority: high
type: feature
ordinal: 56000
---

## Description
<!-- SECTION:DESCRIPTION.BEGIN -->
Give every generated job one way to obtain the `universe` command, in two modes that
share one output contract. Both actions populate one bin directory, put it first on
`PATH`, and export `UNIVERSE_BIN_DIR`; the directory must contain every component
binary, because the facade resolves its components from `PATH` and a facade-only
install fails on first use.

Two composite actions, in this repository:

- `setup-dev/` for owner mode: a Pixi environment plus one `cargo build --bins`, cached
  on the lockfile and the toolchain, exporting `UNIVERSE_MODE=dev` and the commit.
- `setup/` for consumer mode: release artifacts for a pinned commit, verified against
  `SHA256SUMS`, exporting `UNIVERSE_MODE=release` and the version.

The generated workflow then differs between modes in exactly one line, which a test
asserts. Mode mismatch must fail fast: the owner reference resolves against the
repository the workflow lives in, so a consumer that renders it asks for an action it
does not have and stops at the action-fetch step.

The action version is the product version: no version input, one release and one commit
pinning exactly one artifact set. The only overridable value is the pinned commit, which
must be a full SHA because a mutable ref would violate the repository's own
action-pinning rule.
<!-- SECTION:DESCRIPTION.END -->

## Acceptance Criteria
<!-- AC.BEGIN -->
- [ ] #1 Both actions place every component binary in one bin directory, prepend it to `PATH`, and export `UNIVERSE_BIN_DIR`; downstream steps call `universe` with no `cargo`, `pixi`, or path manipulation.
- [ ] #2 The owner action builds once inside the setup step through Pixi, and a compile failure is attributed to that step rather than to the first step that used the binary.
- [ ] #3 The consumer action downloads the pinned artifact set, verifies every checksum, and fails closed on a mismatch or an unknown pin.
- [ ] #4 A test renders owner and consumer workflows from one manifest and asserts the outputs differ in exactly the setup line.
- [ ] #5 Actions are pinned by full SHA everywhere they are used, and the repository's action-pinning lint covers the new files.
<!-- AC.END -->

## Definition of Done
<!-- DOD.BEGIN -->
- [ ] #1 This repository's own release lane runs through `setup-dev/`, proving the owner path before any consumer depends on it.
- [ ] #2 `pixi run gates`, `actionlint`, and `alint` pass.
<!-- DOD.END -->
