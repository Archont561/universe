---
id: UNI-51
title: Adopt Pixi lockfiles as a mirror transport
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
  - UNI-31
  - UNI-41
documentation:
  - backlog/decisions/decision-114 - Pinned-Git-catalogs-and-selective-artifact-retrieval.md
  - backlog/docs/architecture/doc-104 - Ecosystem-adapters-and-external-assets.md
priority: high
type: feature
ordinal: 51000
---

## Description
<!-- SECTION:DESCRIPTION.BEGIN -->
Read a committed `pixi.lock` as the index of a mirror, and publish the artifacts it
names to a dedicated orphan branch. The lockfile already carries an exact URL and a
sha256 per artifact per platform, so the adapter resolves nothing: it fetches each
recorded artifact, verifies the digest, and stores the bytes under a content-addressed
path. A digest mismatch is a hard failure, never a warning.

The first adapter covers conda archives (`.conda`, `.tar.bz2`) **and** PyPI wheels,
because a real Pixi lock mixes both: the development lock in this repository holds 97
distinct conda archives and 7 PyPI wheels (counted 2026-11-09). An adapter that silently
skipped the PyPI half would either break restore or refuse the repository it was written
for.

The same trait boundary must accept further lock formats later (Cargo lockfiles,
`package-lock.json`, `uv.lock`) without touching the CLI or the workflow renderer; the
adapter returns pinned artifacts, and publication is shared. Each adapter gets its own
branch namespace so one ecosystem's growth does not rewrite another's history.

Publication is a producer operation on a connected host, and it is explicit: the
connected host publishes; a disconnected host only ever restores.
<!-- SECTION:DESCRIPTION.END -->

## Acceptance Criteria
<!-- AC.BEGIN -->
- [x] #1 A Pixi lockfile is parsed into pinned artifacts with URL, digest, platform, and package kind, using the lock itself as the only source of version truth. — 2026-11-12: `lock.rs` in `universe-kit` reads schema 6 and 7 into `Pinned { name, version, kind, subdir, url, sha256, size }`, and its own test reads this repository's `pixi.lock` as 97 conda archives and 7 wheels, the counts the notes below recorded independently.
- [x] #2 Every artifact named by the lock is fetched, digest-verified, and published to the adapter's branch under a content-addressed path; a mismatch or a missing artifact fails the run with a structured diagnostic naming the package. — 2026-11-12: `blobs/sha256/<digest>/<file>` on `mirror/<adapter>`. `tests/mirror.rs` serves wrong bytes and a 404, and asserts exit code 1, `airlock.mirror.fetch.digest` / `.request` naming `libgcc 16.2.0`, and an untouched remote.
- [x] #3 The mirror is idempotent: republishing an unchanged lock produces no new commits, and republishing a changed lock adds only the new artifacts. — 2026-11-12: against a real bare repository, an unchanged lock leaves `refs/heads/mirror/pixi` where it was and reports `unchanged`; a lock with one more artifact moves it once, keeps the old blob, and the artifact server records exactly one new request.
- [x] #4 `pixi.lock` and `package-lock.json` share one adapter trait; adding the second lock format requires no change to the CLI surface or the rendered workflows. — 2026-11-12: one `Adapter` trait, two implementations. The second format is `Cargo.lock` rather than `package-lock.json`; the note below records why. `--adapter` selects, so the rendered `mirror/<adapter>` job and the CLI grammar are unchanged.
- [x] #5 Tests run against a fixture lock and a local artifact server; no test requires network egress, and a tampered fixture is rejected. — 2026-11-12: every URL in `tests/mirror.rs` points at a loopback listener serving a map of bytes, and the remote is a bare repository in a temporary directory. No test opens an outbound socket.
<!-- AC.END -->

## Definition of Done
<!-- DOD.BEGIN -->
- [ ] #1 A disconnected host can restore the development environment's artifacts from the mirror branches alone.
- [x] #2 `pixi run gates` passes, including the audit and crate-boundary checks. — 2026-11-12: green locally.
<!-- DOD.END -->

## Notes
<!-- SECTION:NOTES:BEGIN -->
2026-11-09 — correction, recorded as a correction. The description above quoted "164 conda
entries and 4 pypi entries". Counting `pixi.lock` on this date gives 97 distinct conda
archives and 7 PyPI wheels in `packages:`, with 192 and 14 references respectively across
the declared environments in `environments:`. Both counts drifted from the quoted figures; the point of the sentence — a
real Pixi lock mixes both kinds, so an adapter cannot skip wheels — is unchanged.
2026-11-12 — the mirror verb exists, and three things about it are decisions rather than details.

**The second adapter is `cargo`, not `package-lock.json`.** The criterion names npm's lock, but by
the time the verb was written the manifest already carried `LockAdapter = pixi | bun | cargo` and
the renderer already emitted a `mirror/<adapter>` job for each declared one, so the criterion's
substance — two formats, one trait, no CLI change — is reachable without a schema change. A fourth
name would have meant a manifest change and a new matrix entry for a format this repository does
not have. `bun` stays declared and is refused by name until its reader lands, and the refusal
lists the formats this build does read.

**The mirror is additive, and a republish is a function of the bytes.** An artifact lives at
`blobs/sha256/<digest>/<file>`, and `mirror.json` carries no timestamp, no run id, and no ordering
but the mirror path, so two runs of one lock produce one tree. That is what makes an unchanged
lock produce no commit without a "have I seen this before" record anywhere. Because the tree is
rebuilt from the mirror directory, an artifact a newer lock dropped stays published: a consumer
restoring an older pin still needs it, and removing bytes from the only copy is a retention
decision rather than a side effect of a smaller lock.

**A cached artifact is verified again.** The cache lives on a producer's disk, and a truncated
download from last week is exactly what a mirror would otherwise publish with confidence. A cached
file whose digest no longer matches is removed and fetched again.

Limits this slice does not cross, stated so they are not discovered later.

- The tree is assembled in memory, so a mirror larger than one publish's transport ceiling is
  refused with `airlock.mirror.budget` rather than sharded across branches. Splitting a mirror is
  `UNI-47`'s subject.
- No mirror of this repository's whole lock has been run. The seven PyPI records of it were: a
  lock holding only those, mirrored through the real verb, fetched 7 real wheels (21982312 bytes)
  from `files.pythonhosted.org`, verified every digest, and published commit `267866bc` to a real
  bare remote; a second run fetched nothing, reused all seven and reported `unchanged`, leaving
  the commit alone. The other 97 records are conda archives on `conda.anaconda.org`, which this
  sandbox cannot reach: that run fails in 0.2 s with `airlock.mirror.fetch.request` naming
  `_openmp_mutex 4.5` and its URL, which is the fetch path failing closed and is not a mirror.
  `universe.toml` therefore declares no adapter yet: turning the lane on renders a workflow that
  mirrors on every push to `main`, and that is a decision to take with a first real run in hand.
  That is why the Definition of Done stays open while every acceptance criterion is met.
- `verify` and `restore` still read a payload manifest, not a mirror. Pointing the replay at what
  this verb publishes is the next seam, and `mirror.json` is the handover.
<!-- SECTION:NOTES:END -->
