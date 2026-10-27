---
id: m-2
title: "M2 — Core CLI: recipe lint, DAG planner, build, test, and goblin binary auditor"
---

## Description

Implement the core `universe-kit` Rust subcommands replacing per-repository `<TOOL>-conda` Python glue: `lint` (recipe/manifest/workflow static checks), `plan` (`petgraph` intra-universe dependency DAG + `rattler_repodata_gateway` channel diffing -> GitHub Actions matrix JSON), `build` & `test` (wrapping pinned `rattler-build` + native Rust Windows MSVC discovery), and `audit` (streaming `.conda` inspection + `goblin` ELF64/PE32+/Mach-O header & RPATH leak verification + JSON fragment emission), backed by `rstest`, `rstest-bdd`, and `proptest`.

Exit condition: All Rust unit, property, and Gherkin BDD suites pass under `cargo nextest` + `cargo llvm-cov`, and `universe-kit` builds, tests, and audits a real multi-recipe fixture locally.
