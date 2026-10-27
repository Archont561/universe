---
id: m-2
title: "M2 — Kit planning, artifact orchestration, test delegation, and audit"
---

## Description

Implement core `universe-kit` subcommands that replace per-repository glue: lint manifests/recipes/workflows; plan native dependency DAGs; run explicitly configured external build/test commands; audit/verify collected artifacts; and emit deterministic receipts/fragments. Rattler, Cargo, Python wheel, and npm tools remain external native builders/packers. Kit does not bundle or implement them as runtime build systems. Preserve artifact identity, digest checks, isolation, and credential separation.

Exit condition: Rust unit/property/BDD suites pass under the repository gates; command-recording fixtures prove deterministic invocation/collection without network; representative external fixture commands emit and audit artifacts without Kit choosing or implementing ecosystem build steps.