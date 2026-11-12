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
- [ ] #1 A pinned lock restores into an empty prefix on Linux x86_64 with network egress denied, and the restored environment runs a native client. — partially: since 2026-11-12 `replay` takes a mirrored lock into an empty prefix and the installed client runs (`replay_installs_a_mirrored_lock_into_a_prefix_that_runs`), but egress was not experimentally denied while it ran; see the note at the end of this file.
- [x] #2 Every installed artifact's digest matches the lock; a corrupted or missing artifact aborts before any state is activated, leaving the previous state untouched. — 2026-11-12: `prefix::replay` verifies every artifact's size and digest and reads every archive's index in a first pass, then installs in a second, so a damaged or absent artifact leaves the prefix exactly as it was; `a_replay_checks_every_artifact_before_it_installs_any` asserts the prefix is empty after the refusal, and `restore` now installs through the same path.
- [ ] #3 Prefix relocation is verified: no installed file references the original build prefix, and the check is part of the acceptance fixture rather than a manual step.
- [x] #4 `noarch: python` and native packages are both covered by fixtures, including at least one package that carries an entry point. — 2026-11-12: `prefix::materialise` installs a `noarch: python` package under the prefix's own `lib/python3.*`, relocates its `python-scripts/` into `bin`, and generates the commands `info/link.json` declares; `crates/universe-airlock-bootstrap/tests/prefix.rs` covers that case, the `noarch: generic` one, and four refusals, next to the native fixtures (31 tests in the file).
- [x] #5 The restore path contains no resolver and no channel metadata request; the run fails rather than falling back to a channel when an artifact is absent. — 2026-11-12: `universe-airlock-bootstrap replay` reads a local mirror index and links no HTTP client; an artifact the index pins and the mirror does not hold fails with `airlock.replay.missing`, and a dependency the set does not hold fails with `airlock.replay.dependency` rather than being fetched.
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
- Criterion 4 was refused rather than met when this was written; it is met as of the 2026-11-12
  note at the end of this file, which also records the one ordering it depends on.
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

2026-11-12 — `noarch` packages install, and the ordering they depend on is the caller's.

`noarch: generic` needed nothing: its recorded paths are prefix-relative. `noarch: python` needed
three rules, all of them Conda's. Its payload is a `site-packages/` tree, so it is installed under
the prefix's own `lib/python3.X/site-packages`; its `python-scripts/` entries become executable
files in `bin`; and the `entry_points` in `info/link.json` become generated scripts whose shebang
is the prefix's interpreter, because the package cannot know which prefix it lands in. The
archive's own bytes are never rewritten by this: only where they land.

The refusals are the four the archive cannot answer: no interpreter, two interpreters, a payload
path under neither root, and an entry point that is not `name = module:function` — the last one
because everything accepted there is written into a script the environment then executes.

The limit that follows, and it is a real one: a `noarch: python` package has to be installed
*after* the package that carries its interpreter, and nothing in this module orders a payload. A
lock replay therefore has to install the closure in dependency order, which is the next thing this
task needs rather than a defect in the installer.

Two things in that note above are worth restating rather than leaving implied, because they are
what a reader will ask. Egress was **not** denied while the live replay ran: this sandbox cannot
build a netns or a container, so criterion 1's "egress denied" is asserted from the code — the
bootstrapper links no HTTP client and reads only the payload directory it is given — and not from
an experiment. And the noarch refusal is a scope boundary, not an oversight: installing a
`noarch: python` package writes `site-packages` entries against an interpreter the payload does
not yet carry.

2026-11-12 — the lock half lands, and it is a replay in two passes.

`prefix::replay` installs a set of archives into one prefix. The first pass checks every artifact
against the size and digest the index recorded and reads its `info/index.json`; nothing is written
until all of them pass, which is what makes a damaged or absent artifact leave the prefix as it
was. The second pass installs in dependency order.

The order comes from each archive's own `info/index.json`, and only the dependency's name is read:
the set is already solved, so the constraints were the solver's business. Three rules fall out of
that and each of them is a decision:

- **A dependency the set does not hold is a refusal.** A replay has no channel, and fetching one
  package would make it a resolver with a lock for a fig leaf.
- **A virtual package is not a dependency of the set.** `__glibc` and friends describe the host,
  and the lock never pins them because no artifact provides them.
- **A cycle is broken, not refused.** Conda-forge ships packages that depend on each other, so a
  cycle is an order the set does not determine rather than a set that cannot be installed; the
  replay picks the first name and says so in a note.

`replay` takes a list of paths and digests, not a lock. That is the seam: `airlock.mirror` already
reduced a lock to `mirror.json`, so the bootstrapper never learns a lock format and the binary
still links no YAML or TOML reader. `replay_mirror` reads that index, and an artifact whose kind is
not `conda` is counted, named in a note, and left out — a Conda prefix holds Conda packages.

What this does **not** close. Criterion one's subject is a pinned lock restoring with egress
denied, and egress was not denied in this sandbox: what is asserted is structural (the
bootstrapper links no HTTP client, and `replay` reads only the directory it is given) and
behavioural (an absent artifact fails rather than being fetched). Criterion three still needs
installed-file evidence from a published package carrying a placeholder, and the definition of
done still needs a clean-container restore of this repository's own locked environment from mirror
branches — with 97 Conda records, that is the next piece of work rather than a formality.
