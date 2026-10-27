---
id: doc-109
title: Git-backed catalogs and on-demand command execution
type: specification
created_date: '2026-10-27T09:20:00Z'
updated_date: '2026-10-27T09:20:00Z'
---

# Git-backed catalogs and on-demand command execution

**Status: accepted product direction from the 27 October 2026 discussion; not implemented.** `universe-airlock` still only reports VERSION 0.1.0. Command/ref/record spellings here are illustrative, not a released interface. Decision-119 fixes the integrated v1 scope as Conda, Cargo, Python wheels, npm, Airlock consumption, a browsable portal, and standard static Conda publishing; the portal is not a registry.

## Supported-command execution

A proposed `universe-airlock exec -- pixi add numpy` or `universe-airlock exec -- cargo add serde` launches a supported native client with invocation-scoped mirror/source configuration and local protocol-compatible endpoints. Universe Airlock selects an exact catalog snapshot, starts the required local view, launches the child, and coordinates cache misses. Pixi/Cargo retain their resolver semantics; Universe Airlock does not choose a version by guessing from a filename.

Prefer native mirror/registry configuration over a universal forward proxy. HTTPS CONNECT hides individual request paths; TLS interception/custom root-CAs are not the intended design. Unsupported clients, Git dependencies, arbitrary build-script downloads, and additional artifact classes need explicit adapters/policy, not a claim to intercept all requests.

Keep global/user configuration unchanged. Do not accidentally write temporary localhost addresses into persistent native lockfiles. `pixi add` legitimately changes its project inputs; temporary Universe Airlock routing must preserve original source identities in the client's resulting records, or use an explicitly tested/declared relocation mode.

## Logical branch catalogs

Group published catalogs by **registry/protocol**, not language: conda, Cargo, npm, PyPI, and separately typed generic/browser assets. Conda itself contains packages for multiple languages. `bootstrap`, `packages/conda`, `packages/cargo`, `packages/npm`, `packages/pypi`, and `assets` are possible names, not frozen ref spellings.

Each catalog stores/references native metadata plus original archives or independently retrievable shards. Retain conda channel/label namespaces and priorities rather than merging unrelated sources. The authoritative consumer snapshot is the catalog manifest at an **exact commit plus index digest**; branch names are mutable discovery/publication pointers, not lock identities.

The bootstrap manifest pins all selected catalog/payload commits into one consistent snapshot. A command must not mix changing branch heads during a solve. Publish data and indexes, verify availability, and advance the stable bootstrap reference last. Retain referenced commits through reachable history or immutable refs: a remembered hash alone does not guarantee future fetchability. Hash consistency is not publisher authentication; preserve the independently approved trust policy.

## Metadata-first, selective acquisition

1. Read/pin the bootstrap's catalog commitments.
2. Fetch verified metadata needed by the native client.
3. Let that client resolve within the published finite universe.
4. Map its exact artifact request to source identity, digest, size, pinned Git location, and optional shard list.
5. Serve a verified cache hit or delegate a miss to a policy-bounded Git object loader.
6. Verify/reassemble the original artifact bytes, commit atomically to the local CAS, then serve them.

The local HTTP view serves only verified objects. In approved-Git-connected mode a separate bounded loader may fill missing **already published, digest-pinned objects**; it must not resolve, upgrade, or download arbitrary URLs from public registries. Coalesce concurrent requests for the same digest. An incomplete or changed artifact must never become a successful cache hit.

An ordinary/shallow clone may still download every payload. Sparse checkout alone is not a download filter. Use a capability-checked blobless partial fetch without a full package-tree checkout, or an explicit host-specific immutable raw/blob API backend. If selective retrieval is unavailable, fail or require an operator-approved bulk import; never silently download an entire repository. Git metadata/pack overhead and batching remain real costs.

Store each package independently (or shard that artifact independently). One giant compressed catalog archive would defeat lazy retrieval. Logical registry branches do not imply branch-size quotas; the Git 2 GiB constraint discussed earlier is per push, not per branch.

## Native views and finite catalog semantics

Conda needs source-qualified effective/hotfixed metadata, versions/builds, dependency/constraint closure, target subdirs and noarch data. Cargo needs native sparse-index records, features/conditions, checksums and original .crate bytes. Cargo source crates are normally target-independent; target conditions affect resolution/building. npm and PyPI retain their distinct metadata/peer/wheel semantics.

A command can resolve multiple configured platforms, not only the host. Conda metadata may cover a curated platform catalog rather than only one package; avoid promising single-record metadata transfer for every client. Never advertise artifacts that the pinned publication cannot supply. A filtered catalog is an explicitly bounded snapshot, not a claim to reproduce the entire current public registry.

A package outside the published universe fails locally with a useful request/sync remedy. Persistent curated requests still go through `.universe-airlock/` and the trusted connected publisher. No silent upstream fallback is allowed.

## Two consumption modes

- **Approved-Git-connected:** the fetcher can reach an explicitly allowed Git/internal source for cache misses. The wrapped workload is restricted to its intended local endpoints. Public GitHub access is not automatically compatible with denied external egress.
- **Sealed air gap:** prefetch/export/import the chosen scenario's full closure into a local object source before disconnecting. Missing objects fail. Disable unintended promisor-remote lazy fetching as well as registry traffic.

Thus **bootstrap-ready is not offline-ready**. The minimal bootstrap carries verified executables and the package closure needed to establish Pixi-managed Universe Airlock plus catalog pins, not every project environment. It must not require Universe Airlock's HTTP server to install Universe Airlock itself. PATH registration uses an existing user-bin location or explicit activation; a child script cannot mutate its parent's environment.

A future prefetch/verification operation certifies a declared profile's closure. Installation/build evidence and OS-enforced egress tests remain separate from an inventory claim. HTTP and filesystem-only views remain distinct supported contracts; the latter runs no HTTP package service.

## Related records and source references

- [Architecture](doc-102%20-%20Product-architecture-and-bundle-model.md), [bootstrap lifecycle](doc-103%20-%20Init-bootstrap-and-restore-lifecycle.md), [transport](doc-105%20-%20Git-payload-transport-and-curated-requests.md), [acceptance](doc-106%20-%20Offline-verification-and-acceptance-gates.md).
- [Native execution decision](../decisions/decision-113%20-%20Native-client-execution-through-local-package-views.md), [catalog decision](../decisions/decision-114%20-%20Pinned-Git-catalogs-and-selective-artifact-retrieval.md), [readiness decision](../decisions/decision-115%20-%20Bootstrap-readiness-is-distinct-from-sealed-offline-readiness.md).
- Pixi mirrors: https://pixi.prefix.dev/latest/reference/pixi_configuration/
- Git partial clone: https://git-scm.com/docs/partial-clone
- Cargo registry/source replacement: https://doc.rust-lang.org/cargo/reference/registry-index.html and https://doc.rust-lang.org/cargo/reference/source-replacement.html
