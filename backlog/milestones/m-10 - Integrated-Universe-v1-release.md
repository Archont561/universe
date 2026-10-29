---
id: m-10
title: "M10 — Integrated Universe v1 release"
---

## Description

Ship one complete, independently verifiable v1 across Conda, Rust crates, Python wheels, npm,
Airlock consumption, a browsable portal, and standard static Conda publishing. The integrated
v1 is not complete if the portal is only a list of GitHub links or if Conda binaries are not
published at standard channel/subdirectory paths. Kit, Airlock, bootstrap, and the thin
`universe` facade remain distinct components/artifacts.

## Tasks

- `UNI-16` — cut and verify the integrated release (`UNI-47` evidence review blocker).

## Exit condition

Versioned artifacts, checksums, manifests, native indexes, static Conda package paths, Airlock
consumption, and portal snapshot are published and independently verified from the same pinned
release/catalog identity. Sealed acceptance tests have no network fallback; connected release
acquisition/publication is separately evidenced.
