# universe

A static Conda channel, published by Universe. Every file here is generated: the
metadata from the audit fragments, the README from the same fragments. Edit the
project's `universe.toml`, not this branch.

## Packages

| subdir | package | version | build | size | sha256 |
| --- | --- | --- | --- | --- | --- |
| `linux-64` | `universe` | `0.1.0` | `h41f06ac_0` | 195337 | `553744a354b73465ae9e31ed3e0ec49d720fc44bcddcc8ccfd4d0484e4fe190e` |
| `linux-64` | `universe-airlock` | `0.1.0` | `h41f06ac_0` | 568371 | `007a5ed5c16f5028e04fed63a381b2a15ce5e79a26a3e9b0f40d20ac04bef146` |
| `linux-64` | `universe-airlock-bootstrap` | `0.1.0` | `h41f06ac_0` | 535107 | `2ffdd991f81959369b3060e85d7c04630e18531cec9648cb3ec17ec3a4737b41` |
| `linux-64` | `universe-kit` | `0.1.0` | `h41f06ac_0` | 3561765 | `5788df4852acc0a730b96ed27317a7564095afecd2e0b338c4c5ca0a06db537c` |

## Using it

This branch is a channel tree, not a URL. Serve it over HTTP(S) — a static host,
an object store, or `python -m http.server` from a checkout — and point a Pixi
project at wherever it ends up:

```toml
[project]
channels = ["https://<host>/<path>"]
```

A checkout of this branch is enough for a local solve: point the URL at the
directory you checked it out into.

The digests above are the ones the audit measured. A consumer that wants to verify a
download hashes the file and compares it with the row for that package.

Every artifact also has its audit receipt in this tree:
`meta/<subdir>/<package>-<version>-<build>.conda.json`, a kilobyte of metadata —
name, version, dependencies, licence, digests — for a client that wants to know
what a package contains without downloading it.
