---
id: UNI-51
title: Adopt Pixi lockfiles as a mirror transport
status: To Do
assignee: []
created_date: '2026-10-31'
updated_date: '2026-10-31'
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
because a real Pixi lock mixes both: the development lock in this repository contains
164 conda entries and 4 pypi entries. An adapter that silently skipped the pypi half
would either break restore or refuse the repository it was written for.

The same trait boundary must accept further lock formats later (Cargo lockfiles,
`package-lock.json`, `uv.lock`) without touching the CLI or the workflow renderer; the
adapter returns pinned artifacts, and publication is shared. Each adapter gets its own
branch namespace so one ecosystem's growth does not rewrite another's history.

Publication is a producer operation on a connected host, and it is explicit: the
connected host publishes; a disconnected host only ever restores.
<!-- SECTION:DESCRIPTION.END -->

## Acceptance Criteria
<!-- AC.BEGIN -->
- [ ] #1 A Pixi lockfile is parsed into pinned artifacts with URL, digest, platform, and package kind, using the lock itself as the only source of version truth.
- [ ] #2 Every artifact named by the lock is fetched, digest-verified, and published to the adapter's branch under a content-addressed path; a mismatch or a missing artifact fails the run with a structured diagnostic naming the package.
- [ ] #3 The mirror is idempotent: republishing an unchanged lock produces no new commits, and republishing a changed lock adds only the new artifacts.
- [ ] #4 `pixi.lock` and `package-lock.json` share one adapter trait; adding the second lock format requires no change to the CLI surface or the rendered workflows.
- [ ] #5 Tests run against a fixture lock and a local artifact server; no test requires network egress, and a tampered fixture is rejected.
<!-- AC.END -->

## Definition of Done
<!-- DOD.BEGIN -->
- [ ] #1 A disconnected host can restore the development environment's artifacts from the mirror branches alone.
- [ ] #2 `pixi run gates` passes, including the audit and crate-boundary checks.
<!-- DOD.END -->
