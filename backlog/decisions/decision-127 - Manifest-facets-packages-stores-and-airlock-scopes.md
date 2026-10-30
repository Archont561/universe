---
id: DECISION-127
title: Manifest facets for packages, stores, and airlock scopes
status: proposed
created_date: '2026-10-30'
updated_date: '2026-10-30'
---

## Context

`doc-2` specifies `universe.toml` around `[[targets]]`, where a target both describes what
is published and where it goes. That conflation breaks as soon as one build emits a wheel and
an sdist, or one package is published to two destinations.

Two further questions have no answer in the current specification:

- Where does **catalog metadata** come from? A build command cannot produce "what this is and
  why you would want it", yet the portal and every index listing need it.
- Two different things are both called airlock configuration: restoring *my* toolchain so I
  can build, and consuming *someone else's* pinned catalog.

## Decision

One snake_case `universe.toml`, with separate facets. Kebab-case keys are not used; an
importer translates from any older configuration format.

### `[[packages]]` — what to build

| Field | Purpose |
| --- | --- |
| `name`, `kind` (`conda`/`pypi`/`npm`/`cargo`/`r`) | identity and protocol |
| `command`, `env`, `platforms`, `depends_on` | the declared external build, and its ordering |
| `outputs` | globs, resolved against a declared output directory; zero matches or a match outside it is a failure |
| `smoke` | **mandatory** for any package with a publish destination |
| `catalog` | title, summary, license, links, readme — authored here, so discovery works offline |
| `publish` | names of `[[stores]]` entries |
| `submit` | upstream review queues this package declares itself a candidate for |

Structural validation is per kind, because existence is too weak a contract: a `.conda` must
be a zip containing `info/` and a `pkg-*.tar.zst`, a wheel must contain
`<name>-<ver>.dist-info/METADATA`, an npm tarball must contain `package/package.json`. The
build also asserts that it did not modify tracked files.

### `[[stores]]` — where it goes

Each store declares capabilities rather than relying on the renderer special-casing
ecosystems:

| Field | Values |
| --- | --- |
| `kind` | `static`, `releases`, `branch`, `prefix-dev`, `anaconda`, `quetz`, `artifactory`, `s3`, `filesystem`, `npmjs`, `github-packages`, `ghcr`, `command` |
| `writable` | whether the lane publishes here |
| `needs_index` | whether an index must be generated for this destination |
| `read_auth` | `none` or `token` — decides whether the *consumer* needs credentials |
| `verifier` | `file` (read the artifact back) or `registry` (read the registry back) |
| `secrets` | names only; the renderer emits the reference and never a value |

`kind = "command"` is the escape hatch for a registry with no shipped client. It runs a
declared command with declared environment and declared secrets, and it requires a verify
step. It is not an alternative to the supported paths and is deliberately scheduled after
them.

### `[release]` — see `decision-126`

Scopes, tag pattern, changelog path, promotion policy.

### `[airlock.developer]` and `[airlock.delivery]` — two provenances, not one

| Scope | Restores | Trust |
| --- | --- | --- |
| `developer` | my toolchain (pixi, cargo, node), from my own branches | my own pins |
| `delivery` | a catalog someone else published, pinned by commit | theirs, verified by digest |

Merging them would let a restore mix a local compiler with a foreign package set.

### `[workflow]` — the bounded surface

`checks` names a fixed list of commands permitted inside the validation job. `verify_artifact`
and `verify_published` switch the two verification ranges. There is no field that emits an
arbitrary step, and none that changes the file layout.

### Mode separation

The same file is read by both modes, and the consumer mode must not reach the network:

| Mode | Reads | Refuses |
| --- | --- | --- |
| `publish` (connected) | everything | — |
| `restore` (isolated) | `[project]`, `[airlock]`, package identity | any field that requires network; `lint --mode restore` fails such a file |

## Which ecosystems get a static surface

The rule is whether a documented static index format exists that a client can be pointed at:

| Ecosystem | Static index | Decision |
| --- | --- | --- |
| conda | `repodata.json` | static |
| PyPI | PEP 503 `simple/` | static |
| cargo | sparse index | static |
| R | `PACKAGES` | static, source tarballs only; no binary builds |
| npm | no official static format | **hosted**: `npmjs` or `github-packages`; a pointer from the portal, never a byte server |
| containers | OCI is an API, not a layout | **hosted** only |

This partially conflicts with `decision-119` §4, which names an npm-compatible registry
surface. Either the conflict is resolved in favour of a self-hosted surface with a documented
client-support matrix, or `decision-119` is formally amended to permit a hosted npm
destination. **It is not resolved by quietly preferring one**, which is why this record is
`proposed`.

## Consequences

- The renderer emits job shapes from `writable`, `needs_index`, `read_auth`, and `verifier`,
  instead of from ecosystem names.
- `universe lint` validates a store declaration completely offline: URL shape, index kind
  against ecosystem, secret names present, a command present for `kind = "command"`.
- `smoke` is mandatory and has no empty-list escape, so a green verification always means
  something was actually proven.
- Catalog metadata lives in the manifest, which makes discovery work on a disconnected host
  and removes the need for an authored portal page per package.
