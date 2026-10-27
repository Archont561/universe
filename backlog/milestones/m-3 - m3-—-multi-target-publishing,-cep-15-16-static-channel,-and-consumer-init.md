---
id: m-3
title: "M3 — Multi-target publishing, CEP-15/16 static channel, and consumer init"
---

## Description

Deliver arbitrary-channel artifact publishing (`prefix.dev` OIDC + Sigstore attestation, `anaconda.org`, `s3://`, `quetz://`, `artifactory://`, `file://`), rolling `packages-<subdir>` GitHub Releases asset publishing, the fragment-driven headless CEP-15 `repodata.json(.zst)` + CEP-16 sharded repodata + `run_exports.json(.zst)` channel indexer (`universe-kit pages build`, solver data only per decision-11; browser HTML hydration moves to M6 task `UNI-25`), and `universe-kit init` / `add` consumer scaffolding with drift-checked zero-checkout workflows.

Exit condition: A generated consumer repository passes `actionlint`, `universe-kit lint`, and end-to-end local `pixi lock` / `pixi install` resolution against a generated headless CEP-15 static channel and mock `prefix.dev` gateway.
