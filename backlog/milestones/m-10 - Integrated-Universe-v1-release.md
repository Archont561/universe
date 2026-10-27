---
id: m-10
title: "M10 — Integrated Universe v1 release"
---

## Description

Ship one complete, independently verifiable v1 across Conda, Rust crates, Python wheels, npm, Airlock consumption, a browsable portal, and static Conda publishing. The integrated v1 is not complete if the portal is only a set of GitHub links or if Conda binaries are not published in standard channel/subdirectory paths. The `universe` facade/umbrella package is convenient; Kit, Airlock, and bootstrap remain separate components/artifacts.

Release inputs are verified, pinned GitHub Release manifests/assets and catalog snapshots. Package builders are external tools invoked by declared commands; Kit validates and collects outputs, emits native indexes, publishes the static channel, and generates the portal. Release jobs separate untrusted builds from credentialed publication.

The gate must distinguish deterministic offline fixture/consumer tests from connected release discovery and publication. It must not claim the entire suite is offline until every package/browser/cache/advisory dependency is available locally and the complete tests pass with egress denied.

Exit condition: UNI-47 independently reviews passing evidence for the full matrix; versioned artifacts, checksums, manifests, indexes, static Conda package paths, Airlock consumption, and portal snapshot are published together under the v1 release (UNI-16), with clean install/resolve smoke tests and no network fallback in sealed acceptance tests.