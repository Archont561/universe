---
id: m-9
title: "M9 — Native Cargo, Python-wheel, and npm adapters"
---

## Description

Implement the three additional native registry protocols required for v1: Cargo sparse-index/crate archives, a Python wheel Simple API/index and wheel artifacts, and npm-compatible package metadata/tarballs. Keep each adapter behind the neutral core contracts, with protocol fixtures and separate dependency/runtime tests. Conda remains covered by the existing static-channel work; Airlock consumption across all four protocols is tracked separately in M8.

Generic assets, Agent Skills, and STAC are explicitly not part of this v1 adapter milestone. The former five-adapter budget in decision-13 is superseded by decision-119.

Exit condition: each protocol resolves and fetches pinned fixtures with correct metadata/integrity semantics, its tests pass without external network access, and no adapter implementation has leaked into `universe-core`.