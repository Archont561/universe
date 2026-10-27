---
id: m-8
title: "M8 — Universe Airlock offline consumer and bootstrap"
---

## Description

Turn the Airlock design records into a finite, verifiable consumer vertical
slice: sealed snapshot manifest, SHA-256 CAS verification, local native views,
transactional activation, and the thin `universe-airlock-bootstrap` restore
path. Producer/build/publish/watch capabilities remain absent from both
consumer artifacts.

Exit condition: a fixture snapshot restores and runs through a native client
with denied egress, corruption/missing-object cases fail closed, and the
bootstrap path is proven on a clean Linux x86_64 environment.
