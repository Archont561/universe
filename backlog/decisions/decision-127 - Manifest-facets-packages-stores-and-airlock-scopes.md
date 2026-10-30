---
id: DECISION-127
title: Manifest facets for packages, stores, and airlock scopes
status: accepted
created_date: '2026-10-30'
updated_date: '2026-10-31'
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
| `kind` | `static`, `releases`, `branch`, `prefix-dev`, `anaconda`, `quetz`, `artifactory`, `s3`, `filesystem`, `npmjs`, `github-packages`, `pypi`, `crates-io`, `ghcr`, `command` |
| `writable` | whether the lane publishes here |
| `needs_index` | whether an index must be generated for this destination |
| `read_auth` | `none` or `token` — decides whether the *consumer* needs credentials |
| `auth` | `token` (default) or `oidc` — decides whether the *publisher* holds a secret |
| `verifier` | `file` (read the artifact back) or `registry` (read the registry back) |
| `secrets` | names only; the renderer emits the reference and never a value |

`pypi` and `crates-io` are kinds in their own right rather than `command` invocations. Both are
hosted registries with a documented upload client, no index for us to generate, anonymous reads,
and a `registry`-verified read-back — the same shape as `npmjs`, which is already a kind. Leaving
them to the escape hatch would have made the two oldest package registries the two most awkward
destinations in the manifest, and a README could not honestly show one file publishing to all
four ecosystems.

`auth` names how the publishing job authenticates, separately from `read_auth`, because the two
questions are independent: GitHub Packages needs a token to *read*, while PyPI and crates.io can
be *written* with no long-lived secret at all.

`auth = "oidc"` is supported by the two registries that implement trusted publishing — PyPI and
crates.io — and it removes the secret rather than rotating it: the job declares
`id-token: write`, exchanges the workflow identity for a short-lived token, and stores nothing.
Three consequences follow, and the third is the one that surprises people:

1. It is opt-in per store, because a registry without trusted publishing would otherwise fail
   at publish time with an authorisation error nobody can interpret.
2. The rendered job changes shape: an exchange step appears between setup and publish, and the
   secret reference disappears from the job's environment entirely.
3. **Neither registry lets you configure a trusted publisher before the package exists.** The
   first publish of a new package must use `auth = "token"`; the publisher configuration and
   `auth = "oidc"` follow it. The manifest keeps `token` as the default for that reason, and a
   store declaring `oidc` whose package has never been published is a lint warning rather than a
   silent first-release failure.

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
| npm | no official static format | **hosted**: `npmjs` or `github-packages`; see §npm below |
| containers | OCI is an API, not a layout | **hosted** only |

### npm: hosted, and here is what that does and does not change

`decision-119` §4 asks for "npm packages through an npm-compatible registry surface". That
phrase is a statement about **client compatibility** — npm clients must be able to resolve the
packages — and not a requirement that Universe generate registry metadata itself. This record
reads it that way, and amends §4 with a clarifying note so the narrower reading is on the
record rather than in a reader's head.

Why a generated npm surface was rejected:

- **There is no official static-registry format.** Clients fetch a full packument per package
  and expect tarball URLs, so any generated surface is a de-facto protocol that npm does not
  specify and does not promise to keep working.
- **Silent degradation.** `npm audit`, provenance, and dist-tag semantics either fail or
  quietly return less. A user who points a client at the surface discovers this after wiring
  it, which is the worst moment.
- **Client divergence.** npm and bun differ in tolerance for an incomplete packument, so
  "works for me" would be tool-specific.

**What does not change: npm mirroring stays in scope.** Reading `package-lock.json`, fetching
each entry's `resolved` URL, verifying its `integrity` hash, and storing the tarballs on a
mirror branch is core to the offline-replay promise in `doc-113`, and it is independent of
where packages are published. Only *publishing* is delegated to a registry.

Two store kinds, and **no default between them** — the declaration decides:

| Kind | Use | Auth | Consequence |
| --- | --- | --- | --- |
| `npmjs` | public packages | publish token | anonymous reads; provenance attestations available |
| `github-packages` | private or team packages | token, **including for reads** | `read_auth: token`, so consumers need credentials to restore |

The portal lists npm packages as pointers to these registries; it never serves npm bytes.
`verify-index` uses the `registry` verifier (`npm view` read-back) rather than comparing a
digest of a file Universe did not host.

## Consequences

- The renderer emits job shapes from `writable`, `needs_index`, `read_auth`, and `verifier`,
  instead of from ecosystem names.
- `universe lint` validates a store declaration completely offline: URL shape, index kind
  against ecosystem, secret names present, a command present for `kind = "command"`.
- `smoke` is mandatory and has no empty-list escape, so a green verification always means
  something was actually proven.
- Catalog metadata lives in the manifest, which makes discovery work on a disconnected host
  and removes the need for an authored portal page per package.
