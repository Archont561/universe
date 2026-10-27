---
id: m-0
title: "M0 — Reproducible Pixi/Rust/Python monorepo foundation"
---

## Description

Establish the reproducible standalone `Archont561/universe` monorepo
skeleton: locked Pixi developer/CI toolchain, Rust workspace, Python
repository checks, pytest suites, Lefthook/Convco policy, contributor
`.agents/skills/` with content hashes, Backlog records, and owner-lane CI.
`UNI-28` establishes the baseline; `UNI-36` owns the offline developer
transport choice.

Exit condition: `pixi install --locked` and `pixi run gates` succeed from a
clean checkout, and the offline transport boundary for development is either
proven by `UNI-36` or explicitly accepted.
