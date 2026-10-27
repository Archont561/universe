---
id: m-8
title: "M8 — Universe Airlock offline consumer and bootstrap"
---

## Description

Turn the Airlock design records into a finite, verifiable consumer vertical
slice: sealed snapshot manifest, SHA-256 CAS verification, local native views,
transactional activation, and the thin `universe-airlock-bootstrap` restore
path. Producer/build/publish/watch capabilities remain absent from both
consumer artifacts. The Airlock package recipe scaffold and its completion task are tracked by UNI-38.

For v1, local protocol views must cover the supported Conda, Cargo, Python-wheel,
and npm native clients without transparent all-HTTP interception. The producer's
pinned catalog and artifact identity are preserved. Bootstrap readiness and a
fully sealed offline project closure are separate evidence claims.

Exit condition: deterministic fixtures restore and run through each supported
native client with denied egress; corruption, missing-object, and unsupported
request cases fail closed; the bootstrap path is proven on a clean Linux x86_64
environment.
