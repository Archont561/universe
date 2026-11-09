---
id: UNI-12
title: Implement standard static Conda channel indexing and artifact publication paths
status: In Progress
assignee: []
created_date: '2026-10-26 20:15'
updated_date: '2026-11-09'
labels:
  - channel
  - testing
milestone: m-3
dependencies:
  - UNI-10
  - UNI-11
documentation:
  - backlog/docs/architecture/doc-3 - Static-channel-and-SvelteKit-UnoCSS-portal-specification.md
  - backlog/docs/architecture/doc-111 - V1-package-layout-and-offline-readiness.md
  - backlog/decisions/decision-5 - Static-Conda-channel-on-GitHub-Pages-+-GitHub-Releases-via-CEP-15-base_url,-CEP-16-shards,-and-immutable-JSON-fragments.md
  - backlog/decisions/decision-119 - Full-multi-ecosystem-v1-and-static-catalog-distribution.md
priority: high
type: feature
ordinal: 12000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Implement deterministic Conda channel indexing and a static publish layout from verified
package artifacts and audit fragments. Emit standards-compatible metadata for all configured
and supported subdirs, including empty subdirs, and publish the actual package files at the
channel's canonical subdir paths (or an explicitly tested equivalent static object-store
layout). GitHub Release links alone are not a usable Conda channel and must not be treated as
substitutes for static package availability.

Emit CEP-15 repodata and CEP-16 shard files, `run_exports` metadata, channel metadata, and
`.nojekyll` as specified in doc-3. Resolve `base_url`/artifact references so each advertised
package can be downloaded by a native Conda client from the published static channel. Portal
HTML/catalog hydration is implemented separately in UNI-25/UNI-46; this task owns the
Conda solver and package artifact surface only.

The adapter request/result boundary — capabilities, the plan/build/audit requests, the six
outcomes, and produced artifacts — lives in `crates/universe-core/src/adapter.rs` (`UNI-31`).
This task consumes those records instead of defining a per-command shape of its own.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 `proptest` proves byte-identical repodata, compressed metadata, and shards for arbitrary input fragment order. — 2026-11-04 (partial): a property test proves arbitrary fragment order renders a byte-identical tree, but only plain `repodata.json` exists — no `.zst` metadata and no shards.
- [x] #2 Every configured subdir, including empty subdirs, has valid metadata and standard channel paths; every advertised artifact's bytes are present at the canonical static path and match its digest/size. — 2026-11-04: `universe index --output <dir>` writes one `repodata.json` per configured subdir plus `noarch` even when empty, `channeldata.json`, `run_exports.json`, `.nojekyll`, and the generated `README.md`, and copies each verified artifact to `<subdir>/<filename>`; a test asserts the artifact's bytes arrive unmodified.
- [ ] #3 Local HTTP integration proves a real Conda/Pixi client resolves, downloads, and installs a package from the generated static channel with network egress denied outside the local fixture.
- [ ] #4 Missing artifact bytes, digest mismatch, unsupported metadata, or unavailable static path fails publication closed; a GitHub Release URL alone fails the fixture acceptance test. — 2026-11-04 (partial): missing bytes and digest mismatches fail both verbs closed with exit 1 and name the file; compressed metadata and shards are not written yet, so the unsupported-metadata path has nothing to meet.
- [ ] #5 Output contains Conda metadata and package files only; portal extraction/HTML hydration is not invoked.
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 BDD/property tests and local HTTP client resolution tests pass with the offline fixture set.
- [ ] #2 The deployment contract documents the static hosting/object-storage paths and makes no claim that portal links are a channel.
<!-- DOD:END -->


## Notes
<!-- SECTION:NOTES:BEGIN -->
2026-11-09 — status corrected from `To Do` to `In Progress`. AC#2 is ticked with evidence and
AC#1/#4 carry recorded partial results; `To Do` no longer described the record.

Scope for the `v0.1.0` checkpoint (`decision-132`, and its amendment that the checkpoint's
static channel is a Git branch). This declares what the tag consumes, not a completion claim;
every unticked box stays unticked.

- Consumed: plain `repodata.json` per subdir including `noarch`, `channeldata.json`,
  `run_exports.json`, `.nojekyll`, the generated `README.md`, and package bytes at canonical
  `<subdir>/<filename>` paths with a byte-for-byte assertion (AC#2). `UNI-58` proves a real
  Pixi client resolves that tree from a Git checkout with `pixi install --offline`.
- AC#1 out of scope for the tag: `.zst` compressed metadata and CEP-16 shards. The property
  test proves byte-identical *plain* repodata under arbitrary fragment order; the compressed
  forms do not exist yet.
- AC#3 out of scope for the tag: a local HTTP integration. The amendment puts the channel on a
  branch precisely so a disconnected host needs only Git; `channel_resolves.rs` is the offline
  client proof. An HTTP endpoint belongs to the v1 hosting decision.
- AC#4 partially met: missing bytes and digest mismatches fail closed with exit 1 and name the
  file. The unsupported-metadata path has nothing to meet until compressed metadata and shards
  exist.
- AC#5 and both DoD boxes stay open. The branch tree also carries a generated `README.md` and
  `meta/` audit receipts; those are channel documentation and evidence rather than portal HTML,
  but the criterion says "Conda metadata and package files only", so ticking it would overstate
  the record.

`m-12` consumes this subset without re-scoping the task; M3 and `decision-119` still own the
rest.
<!-- SECTION:NOTES:END -->
