---
id: m-3
title: "M3 — Artifact publication, static Conda channel, and consumer init"
---

## Description

Deliver arbitrary-target artifact publication, native Conda channel indexing, and
consumer `init`/`add` scaffolding. Publish verified package bytes and metadata at
standard static channel/subdirectory paths. CEP-15/CEP-16 solver artifacts are
part of the producer channel; the browsable portal is separately implemented in
M6 and is also required for integrated v1. Do not treat GitHub Release download
links as a replacement for static channel package paths.

Exit condition: local HTTP tests prove a real Pixi/Conda client resolves,
downloads, and installs fixture packages from the published channel tree with
network egress denied beyond the fixture. Publication target and consumer
workflow tests preserve the approved OIDC/credential boundaries.