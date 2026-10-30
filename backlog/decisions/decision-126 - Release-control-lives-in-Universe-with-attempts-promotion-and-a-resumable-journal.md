---
id: DECISION-126
title: Release control lives in Universe, with attempts, promotion, and a resumable journal
status: accepted
created_date: '2026-10-30'
updated_date: '2026-10-31'
---

## Context

Release control for this repository has been provided by a separate controller tool with its
own policy file. That tool is being retired in favour of Universe, because Universe is
intended to own the whole chain: one manifest, one renderer, one set of verbs, and the same
code path in owner and consumer mode.

Retiring it is only safe if the semantics worth keeping are carried over as Universe's own
requirements. A policy file can be copied in an afternoon; the expensive part is the
behaviour that was learned by failing in production:

- a release that rests on evidence a previous run already recorded,
- a publish that can be resumed instead of repeated,
- proof that the artifact works, not that the build shell worked,
- and a rule that every declared destination is mandatory.

Those become requirements of this decision, together with the version policy in
§Version policy: a minor bump is breaking before 1.0.

## Decision

**`universe-release` becomes a crate in this workspace, and release control becomes a set of
`universe release` verbs.**

```
universe release evidence --out evidence.json
universe release plan     --out plan.json [--dry-run]
universe release prepare  --plan plan.json
universe release cut      --plan plan.json [--push]
universe release classify --plan plan.json
universe release build    --plan plan.json --out report.json
universe release publish  --plan plan.json --root dist --journal journal.json
universe release resume   --plan plan.json --journal journal.json
universe release promote  --from X.Y.Z-rc.N --to X.Y.Z
```

State machine, idempotent at every edge:

```
plan -> prepared -> cut (tag | draft) -> classified -> built -> published*
  |
  +-- dry run: writes nothing, same JSON shape
```

Four schema-versioned artifacts, each digest-stamped: `evidence.json`, `plan.json`,
`report.json`, `journal.json`. The journal is append-only and keyed by
`(release-id, target)`; `resume` runs exactly the targets without a `published` record.

### Invariants

1. **The release rests on recorded evidence.** `plan` reads the conclusion the gate workflow
   already recorded for the commit being released, and refuses without it. It does not run a
   second suite.
2. **Every declared destination is required.** A partially published release is not a
   release.
3. **Publish is resumable, never repeatable.** A second publish of the same
   `(release-id, target)` is an error, not an overwrite. A killed publish is repaired by
   re-running the failed jobs.
4. **Proof of bytes.** Verification runs the artifact in an isolated environment, executes
   its declared smoke commands, and asserts the version it reports. The release proves the
   artifact, not the shell that built it.
5. **Dry-run parity.** A dry run differs from a real run only in writes.
6. **Immutability check per destination.** The published digest equals the digest in
   `report.json`, verified by reading back from the destination.
7. **Version parity.** Every declared manifest, the recipe version, and the changelog section
   for the version carry the same literal. A `prepare` that stamps three of four is a
   failure, not a later surprise.
8. **Scope determinism.** Tag parsing uses the longest declared scope prefix. An unmatched
   tag is a lint error, never an inference.
9. **One concurrency group per scope**, with cancellation disabled: a cancelled tag push can
   split a release train.
10. **Bot identity and an annotated tag.** Release notes come from the tag annotation and
    the curated changelog section.

### No tag lock-in: attempts and promotion

1. **Attempt.** A dispatch with a prerelease version builds from the current commit, creates
   a **draft** release, and uploads assets. A draft creates no tag — GitHub creates the tag
   only on publish — so attempts leave nothing behind.
2. **Numbering.** `-rc.N` increments per attempt, and each attempt has its own journal.
3. **Promotion.** `release promote` asserts the attempt's digests, publishes the draft (the
   tag is created at that moment), marks the release non-prerelease, and fans out to stable
   destinations.
4. **Promotion never rebuilds.** Bytes are identical; only the tag and the destination
   pointers change. This is the entire point.
5. **Stable destinations receive final versions only.** Prereleases go to a prerelease
   channel, a dist-tag, and a draft. Discarding an attempt means deleting a draft, leaving
   zero tags behind.

### Scoped releases

Multiple packages in one repository need independent release trains without tag guessing:

```toml
[release]
tag_pattern = "{package}-v{version}"   # "v{version}" alone is the whole-repository train
promote = "manual"

[[release.scope]]
name = "portal"
packages = ["portal"]
tag_prefix = "portal"
stores = ["npm"]
```

Parsing is by **longest declared prefix**; scopes are disjoint, and overlapping package lists
are a lint error. `plan` narrows both the package set and the destination set to the matched
scope, so `build`'s matrix shrinks accordingly.

### Fan-out from one origin

Builds happen once. The GitHub Release holds the bytes. Every other destination — hosted
channel, static index, portal, submission to an upstream review queue — fetches that release,
re-verifies each digest, and publishes a copy or an index entry. None of them rebuild.

A static channel is not a list of release links: it serves artifacts at the protocol's
standard subdirectory paths with regenerated metadata. Release assets may remain the
immutable copy; they cannot be the channel by themselves.

A submission to an upstream review queue is separate again: that queue rebuilds from source
and cannot consume your artifacts, so it is a declaration, not a destination.

## Version policy: a minor bump is breaking before 1.0

**Chosen: `minor-is-breaking`.** While the major version is zero, the compatibility boundary is
the *minor* version. In Cargo terms a release is compatible with `^0.Y.Z`, and in conda terms
with `>=0.Y.Z,<0.(Y+1)`.

| Change class | Pre-1.0 bump | Post-1.0 bump |
| --- | --- | --- |
| Breaking | minor (`0.3.1` -> `0.4.0`) | major |
| Feature | minor | minor |
| Fix, performance | patch | patch |
| Documentation, build, CI, chore | none | none |

This is not a new convention imposed on the design; it is the convention the existing records
already assume. `decision-1` §3 scaffolds consumer manifests with
`universe-kit = { version = ">=0.1,<0.2" }`, which is `minor-is-breaking` written as a range.
The alternative — treating a pre-1.0 breaking change as a major bump, so that the first one
produces `1.0.0` — was rejected because it would force a 1.0 the product has not earned and
would contradict that pin.

**`1.0.0` is a human decision**, not derived from a diff. This policy governs bumps *within*
the `0.y.z` series; it never promotes the project to 1.0 on its own.

### What counts as breaking

Detection is mechanical (`!` in the type, or a `BREAKING CHANGE` footer), but the mechanical
signal is not sufficient on its own. These listed changes are breaking even when the commit
message does not say so, and a release that contains one requires a minor bump:

- removing or renaming a CLI verb, flag, or positional argument;
- changing an exit code that is part of the documented contract;
- removing or renaming a `universe.toml` key, or changing a key's accepted values;
- bumping `STDOUT_SCHEMA_VERSION` in `universe-core`, because the JSON envelope is a
  documented interface;
- changing a default that alters published artifact bytes.

The last two are the ones a commit message will never mark correctly, which is why they are
enumerated rather than inferred.

### Two related parameters

Chosen here because both are usually left implicit: unparseable commits are ignored unless
they would have changed the version, in which case the plan fails; and the commit range is
everything since the last reachable tag, with an explicit `--from` for attempts.

## Consequences

- One release path, one set of semantics, one place to fix a defect in it.
- The verbs are identical in dev and release mode; only the controller identity differs
  (`decision-125`).
- Docker and hosted-registry destinations collapse build and publish into one job, because
  pushing is publishing. That job class is tag-only and never reachable from a pull request,
  and it needs its own gated template rather than a flag on `build`.
- The retirement of the previous controller is gated on parity: the semantics above must be
  implemented and exercised before the old path is removed. An archive, not a deletion,
  keeps pins working for anyone still referencing it.
- This record states requirements in Universe's own terms. No Backlog record may reference
  the retired tool's repository (house rule 9).
