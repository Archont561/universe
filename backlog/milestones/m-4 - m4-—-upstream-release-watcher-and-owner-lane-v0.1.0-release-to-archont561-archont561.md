---
id: m-4
title: "M4 — Upstream release intake and trusted producer lanes"
---

## Description

Implement pinned upstream release intake, finalize the component Conda recipes and platform variants, and establish isolated, fail-closed owner publishing lanes with provenance/attestations. Kit invokes configured external native build/test commands; it does not ship `rattler-build` as a runtime dependency or compile product packages itself. This milestone provides trusted producer/release foundations but does not cut a Conda-only stable product release.

The integrated v1 release is M10 and includes the portal, Conda and static Conda channel, Cargo, Python wheels, npm, and Airlock consumption. GitHub Release artifacts must include a machine-readable manifest and checksums; release discovery uses pinned identities and digests.

Exit condition: owner/release jobs can validate, build through declared external tools, collect, verify, attest, and publish component artifacts without exposing publishing credentials to untrusted build jobs; clean install tests pass for the individual components.