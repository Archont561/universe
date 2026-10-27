---
id: UNI-22
title: Harden archive extraction and source fetching against tar bombs and unsafe redirects
status: To Do
assignee: []
created_date: '2026-10-27 09:20'
updated_date: '2026-10-27 09:20'
labels:
  - audit
  - ci
milestone: m-2
dependencies:
  - UNI-10
documentation:
  - backlog/decisions/decision-10 - Adopt-ODM-conda-release-governance-and-supply-chain-hardening-patterns.md
  - backlog/decisions/decision-3 - Hybrid-Rattler-library-+-subprocess-architecture-and-pure-Rust-rustls-dependency-policy.md
  - backlog/docs/architecture/doc-2 - Universe Kit-CLI-and-manifest-specification.md
priority: high
type: feature
ordinal: 22000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
ODM-conda's `package_archive.py` and `common.py` encode supply-chain rules that universe-kit must guarantee in Rust before it builds third-party recipes at scale:
- **Archive member validation** (zip-slip/tar-bomb defense used while reading `.conda`, source tarballs, and any downloaded fragment): reject absolute paths, `..` traversal, backslashes, Windows drives/colons, symlinks/hardlinks unless explicitly allowlisted, and enforce per-file, total-bytes, and member-count caps (the ODM defaults of 64 MiB per file, 256 MiB total, 10,000 members are the starting point, made configurable in `universe.toml`).
- **Source fetch discipline**: source URLs must be credential-free HTTPS; the TLS scheme is re-checked after every redirect (a downgrade to `http://` fails); cached archives are re-hashed against the pinned SHA256 on every use rather than trusted by path; downloads enforce a hard size cap and stream-hash while writing to a temp file that is atomically renamed only after the digest matches.
- **Hermetic source policy**: the recipe linter (UNI-7) refuses enabled submodules (`.gitmodules`), and CMake `FetchContent`/`ExternalProject_Add`/`file(DOWNLOAD ...)` tokens in enabled source trees, mirroring ODM's build-time refusal of unstaged network bootstraps; everything a build needs comes from a pinned, hash-checked archive declared in the recipe.
All rules live in reusable `universe-core` modules with `thiserror` typings and are covered by property tests using in-memory archive builders — no live network in tests (wiremock only).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 In-memory fixture archives containing an absolute member, a `../` traversal, a symlink, an over-cap member, and an over-cap member count are each rejected with a distinct error before any byte is extracted to disk.
- [ ] #2 A wiremock source server that redirects HTTPS→HTTP, serves a digest mismatch, or streams beyond the size cap is rejected in every case; a cached archive whose file was mutated fails on re-hash even though the path still exists.
- [ ] #3 Recipe lint tests prove submodules and the three CMake fetch tokens fail validation for enabled source trees while disabled/vendor trees remain allowed when explicitly declared.
- [ ] #4 No test or production path performs a non-HTTPS source download, and no archive extraction call site can bypass the shared validator (enforced by making the safe API the only public constructor).
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 The caps and their defaults are documented in `doc-2` and surfaced as typed `universe.toml` settings with `deny_unknown_fields` schema coverage under UNI-30.
<!-- DOD:END -->
