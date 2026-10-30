---
id: UNI-41
title: Define multi-ecosystem project and pinned release manifest contracts
status: To Do
assignee: []
created_date: '2026-10-27'
updated_date: '2026-10-31'
labels:
  - architecture
  - schemas
  - supply-chain
milestone: m-1
dependencies:
  - UNI-30
  - UNI-31
documentation:
  - backlog/docs/architecture/doc-2 - Universe-Kit-CLI-and-manifest-specification.md
  - backlog/docs/architecture/doc-111 - V1-package-layout-and-offline-readiness.md
  - backlog/decisions/decision-119 - Full-multi-ecosystem-v1-and-static-catalog-distribution.md
priority: high
type: feature
ordinal: 41000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Extend the existing `universe.toml` / per-package `package.toml` contract for Conda,
Cargo, Python wheels, and npm without replacing the established project format.

Shape is governed by `decision-127`: `[[packages]]` describes what is built (`command`,
`outputs`, `smoke`, `platforms`, a `catalog` facet, and named destinations) and
`[[stores]]` describes where bytes go, declaring `writable`, `needs_index`, `read_auth`,
and `verifier` so the renderer never special-cases an ecosystem. The single
`[[targets]]` list in `doc-2` cannot express one build publishing to two destinations,
which is why the split is part of this contract rather than a later refactor. Airlock
configuration is split into `[airlock.developer]` and `[airlock.delivery]`, because
restoring my own toolchain and consuming someone else's catalog are different
provenances that must not be mergeable into one block. Define a
versioned machine-readable GitHub Release manifest (planned filename `universe-release.json`)
that binds repository, immutable tag/release identity, catalog snapshot, ecosystem/package
identity, target/version, asset name, size, and cryptographic digest. Specify signature/
attestation references and fail-closed validation rules. All pins must resolve to exact
release identities and content digests; mutable `latest` discovery is not a release input.

The schema must make native build/test commands and expected outputs explicit while
separating them from Kit's artifact validation/index/publish responsibilities. Keep neutral
catalog and integrity fields in core; ecosystem-specific metadata belongs to adapters.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Versioned schemas cover the four v1 protocols and reject unknown/unsafe command, path, credential, URL, and digest fields as specified.
- [ ] #2 Release-manifest fixtures cover valid, missing, duplicate, tampered, mutable-ref, wrong-repository, and mismatched-catalog cases.
- [ ] #3 Manifest validation is deterministic and network-free; GitHub release acquisition is a separate pinned connected operation.
- [ ] #4 Catalog snapshots preserve exact source/release pins and content digests; portal and adapters consume the same verified snapshot identity.
- [ ] #5 Document backward compatibility/migration from existing `universe.toml` and `package.toml` without silently changing existing Conda recipe semantics.
- [ ] #6 A package with a publish destination and no smoke command is rejected at lint level, and a store declaring `read_auth: token` whose secret name is absent is rejected too, so neither fails first at publish time.
- [ ] #7 Universe authors its own `universe.toml` and it lints clean, because a facet nobody has written for a real project is a facet that will not fit the first one. It is the parser's first fixture and the only manifest in this repository.
- [ ] #8 A package naming a store of an incompatible kind, a store that does not exist, or a store declaring `writable = false` fails lint with both names in the message; a writable store no package names is a warning.
- [ ] #9 A `[[release.scope]]` that names a store no package in that scope publishes to fails lint, so a scope promising a destination nothing feeds is rejected before anything renders; what the render prints is `UNI-57`.
- [ ] #10 `lint --mode restore` refuses a manifest whose fields require network, so a disconnected host cannot be handed a configuration that silently reaches for a registry.
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 Rust/schema and Python/TypeScript generated bindings, as applicable, are synchronized and validated by the offline schema gate.
- [ ] #2 No ecosystem-specific protocol type is added to `universe-core` outside the agreed neutral model.
<!-- DOD:END -->
