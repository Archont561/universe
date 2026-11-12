---
id: UNI-52
title: Materialise a Conda prefix from a pinned lock
status: In Progress
assignee: []
created_date: '2026-10-31'
updated_date: '2026-11-12'
labels:
  - airlock
  - offline
  - adapters
milestone: m-5
dependencies:
  - UNI-32
  - UNI-51
documentation:
  - backlog/decisions/decision-102 - Original-conda-artifacts-and-effective-metadata.md
  - backlog/decisions/decision-103 - Finite-snapshots-locked-replay-and-curated-additions.md
  - backlog/docs/doc-106 - Offline-verification-and-acceptance-gates.md
priority: high
type: feature
ordinal: 52000
---

## Description
<!-- SECTION:DESCRIPTION.BEGIN -->
Turn a pinned lock plus mirrored artifacts into a working prefix on a host with no
network. A Conda package is not a tarball that can be dropped in place: paths carry a
build-time prefix placeholder that must be relocated, `info/paths.json` describes
symlinks and hardlinks, and `noarch: python` packages install differently from native
ones. Getting this wrong produces an environment that imports from the build host's
paths.

The restore is a replay, never a solve. The lock already names every artifact, so the
materialisation step builds an explicit, exact specification (URL or local path plus
the recorded sha256 for each entry) and asks an installer to realise it. Bundling a
small static installer is the accepted approach for the first version, because prefix
relocation is solved there and no dependency resolution is required; owning the
extractor in Rust is a later option, not a prerequisite.

Scope boundary to state plainly: restore reconstructs a prefix and its activation
environment. It does not reproduce the task runner's semantics, multi-environment
switching, or any solve-time behaviour. The lock is the whole contract.
<!-- SECTION:DESCRIPTION.END -->

## Acceptance Criteria
<!-- AC.BEGIN -->
- [ ] #1 A pinned lock restores into an empty prefix on Linux x86_64 with network egress denied, and the restored environment runs a native client.
- [ ] #2 Every installed artifact's digest matches the lock; a corrupted or missing artifact aborts before any state is activated, leaving the previous state untouched.
- [ ] #3 Prefix relocation is verified: no installed file references the original build prefix, and the check is part of the acceptance fixture rather than a manual step.
- [ ] #4 `noarch: python` and native packages are both covered by fixtures, including at least one package that carries an entry point.
- [ ] #5 The restore path contains no resolver and no channel metadata request; the run fails rather than falling back to a channel when an artifact is absent.
<!-- AC.END -->

## Definition of Done
<!-- DOD.BEGIN -->
- [ ] #1 A clean-container fixture restores Universe's own locked development environment from mirror branches only.
- [ ] #2 `pixi run gates` passes and no test requires network egress.
<!-- DOD.END -->


2026-11-12 — the payload this task replays now has a producer, and one thing is worth separating.

`universe airlock pack` writes the closed payload a bootstrapper reads: `universe-release.json`
naming each released artifact with its digest, size and destination stores, plus the files
themselves. Run against this repository's own `bootstrap` pin and the four packages of release
`43252390113`, the rendered entrypoint then restored end to end and printed `restored: 0.1.0`
for environment `0.1.0-9ad2990272994a06`, and `universe-airlock-bootstrap verify` read that
payload as `artifacts: 4 (4286054 bytes)`. That closes the gap this repository kept hitting at
`[airlock.payload.manifest] cannot read the release manifest`.

What it does **not** do is what this task is about. Activation copies the four `.conda` files
into one immutable environment directory and swaps a symlink; nothing extracts them into a
prefix, nothing relocates a build-time placeholder, and no `info/paths.json` is applied. So no
acceptance criterion here moves: #1 needs a prefix that runs a native client, #2 needs those
digests checked at install rather than at copy, #3 needs relocation evidence, and #5 needs the
replay path itself. The payload being closed is a precondition for those, not a substitute for
them.

## Notes

2026-11-12 — the extraction half exists; the lock half does not, and the difference matters.

`prefix::materialise` in `universe-airlock-bootstrap` installs one archive into one prefix. It is
the part of this task the archive can answer on its own: `info/paths.json` says which entries are
hardlinks, symlinks and directories, which digest and size each installed file must have, and
which of them carry a build-time prefix; the module applies all of that. Three decisions are worth
recording.

- **The digest is checked after relocation.** `paths.json` describes the file as installed, so
  hashing the bytes that landed is the only check that also covers the rewrite.
- **Both directions of `paths.json` are enforced.** An archive entry the record does not name is a
  file whose bytes nobody recorded; a name with no payload is an environment missing a file it was
  told it has. Both are refused rather than skipped.
- **Owning the extractor is the option the description already allowed**, taken now rather than
  bundling an installer. It costs this binary `zip`, `zstd`, `tar` and `serde_json`: compile-time
  weight, not a runtime dependency, so the "no preinstalled language ecosystem" property holds.

Measured, on this repository's own payload: the four packages of release `43252390113` restore
into a prefix holding `bin/universe-airlock`, `bin/universe-kit`, `bin/universe` and
`bin/universe-airlock-bootstrap`, all at mode `0755`, and `prefix/bin/universe-airlock --version`
prints `universe-airlock 0.1.0`. `restore` reports `installed: 6 entries (11904056 bytes)`.

**No acceptance criterion moves, deliberately.**

- Criterion 1 says a *pinned lock* restores. This restores a *payload manifest* — the artifact list
  `airlock pack` writes. Turning a Pixi lock into that list is `UNI-51`, so the criterion is
  about a seam this change does not cross. Its second half does now hold for the payload path: an
  empty prefix, and a native client that runs out of it.
- Criterion 2's digest check is per-artifact today (unchanged) plus per-file at install; "matches
  the lock" still needs the lock.
- Criterion 3's relocation is implemented and tested on fixtures, but no published package here
  carries a placeholder, so there is no installed-file evidence from the real artifacts yet.
- Criterion 4 is refused rather than met: `noarch` packages fail with `airlock.prefix.noarch`.
- Criterion 5 holds for this path — no resolver, no channel request, and a missing artifact fails
  the run — but it is one clause of a criterion whose subject is the replay.

2026-11-12 — what the new tests make checkable, and what they cannot.

`tests/prefix.rs` now builds one `.conda` per case and asserts the refusal, not just the success:
both directions of `info/paths.json`, size and digest separately, a symlink or hardlink that leaves
or names nothing, a placeholder that does not fit a fixed-width binary patch, a second payload
member, a payload that is not zstd, and an entry type this build cannot install. `state.rs` also
measures the prefix it reuses instead of reporting zero files. `pixi run gates` passes: 652 Rust
tests, 169 Python tests, Rust line coverage 90.2 percent against the gate's 90, `prefix.rs` itself
at 78 percent, and no test touches the network.

Two things in that note above are worth restating rather than leaving implied, because they are
what a reader will ask. Egress was **not** denied while the live replay ran: this sandbox cannot
build a netns or a container, so criterion 1's "egress denied" is asserted from the code — the
bootstrapper links no HTTP client and reads only the payload directory it is given — and not from
an experiment. And the noarch refusal is a scope boundary, not an oversight: installing a
`noarch: python` package writes `site-packages` entries against an interpreter the payload does
not yet carry.
