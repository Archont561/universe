---
id: UNI-44
title: Publish npm packages to a hosted store and mirror npm artifacts
status: To Do
assignee: []
created_date: '2026-10-27'
updated_date: '2026-10-31'
labels:
  - npm
  - registry
  - testing
milestone: m-6
dependencies:
  - UNI-31
  - UNI-41
documentation:
  - backlog/decisions/decision-127 - Manifest-facets-packages-stores-and-airlock-scopes.md
  - backlog/docs/architecture/doc-111 - V1-package-layout-and-offline-readiness.md
  - backlog/docs/doc-109 - Git-backed-catalogs-and-on-demand-command-execution.md
priority: high
type: feature
ordinal: 44000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Implement npm as a **publish destination plus a mirror**, not as a generated registry.
`decision-127` settled this: there is no official static registry format, `npm audit`,
provenance, and dist-tags degrade silently against a hand-rolled packument surface, and
`npm` and `bun` disagree about incomplete documents. Generating one would be a
compatibility promise this project cannot keep.

Publishing is delegated to a hosted store, and the adapter implements the two store
kinds the manifest may declare, with no default:

- `npmjs` — one publish token, anonymous reads, provenance attestation supported.
- `github-packages` — the token is required for **reads as well**, so the store declares
  `read_auth: token` and consumers pay a real authentication cost. The portal must say so
  rather than presenting the two as interchangeable.

Mirroring stays in scope and is the part this project actually owns. A consumer's
`package-lock.json` records a `resolved` URL and an `integrity` digest per package; the
lock adapter (UNI-51) turns those into pinned artifacts, verifies the digest, and stores
the bytes on a mirror branch. Only the publish half is handed to a hosted registry.

Verification is per destination kind: the `registry` verifier confirms the published copy
with `npm view` against the declared registry, rather than comparing against a file this
tool wrote itself.
<!-- SECTION:DESCRIPTION:END --><!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Publishing to `npmjs` and to `github-packages` both work from one pinned artifact set, and the two differ only in declared store fields (registry URL, auth scope, provenance support), not in code paths visible to the CLI.
- [ ] #2 A store declaring `read_auth: token` causes the consumer-facing documentation and any generated `.npmrc` to require authentication for reads; a manifest that omits the secret name fails lint rather than at first install.
- [ ] #3 The published copy is verified with `npm view` against the declared registry, including version and dist-tag, and a mismatch fails the release with the destination named.
- [ ] #4 A real npm-compatible client installs the published package with egress restricted to the declared registry, and the test proves it cannot fall through to the public registry.
- [ ] #5 Mirroring is covered: a `package-lock.json` fixture with `resolved` and `integrity` values produces digest-verified mirror entries, and a tampered fixture is rejected.
- [ ] #6 No task, code path, or document in this repository generates or serves npm packuments; a check fails if one reappears.
<!-- AC:END --><!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 Both store kinds are exercised against a pinned release before either is enabled for real publication, and token presence alone never counts as evidence.
- [ ] #2 Offline client tests pass with egress denied; the absence of a hosted-registry dependency in the sealed path is documented.
<!-- DOD:END --><!-- DOD:END -->
