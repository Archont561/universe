---
id: doc-113
title: Universe scope, problem statement, and refusals
type: specification
created_date: '2026-10-30'
updated_date: '2026-10-30'
tags:
  - architecture
  - scope
  - distribution
---

# Universe scope, problem statement, and refusals

This record states what Universe exists to solve, so that later records can cite one
authority instead of restating the argument. It is a synthesis of `decision-1` and
`decision-119`; where it appears to conflict with either, those decisions win and this
record is the thing to correct.

## The problem in one sentence

Distribute your own software, across several ecosystems at once, without a registry
provider, and be able to reproduce it byte-for-byte on a machine with no network.

That is the whole thesis. "Publish anywhere" and "take it offline" are one problem with
two halves, and the second half is the differentiator: anyone can upload artifacts,
almost nobody can prove that a disconnected host restores exactly what was published.

## The four problems

| # | Problem | Evidence | The answer |
| --- | --- | --- | --- |
| P1 | **CI sprawl per repository.** Each ecosystem-specific packaging repository re-implements the same actions YAML, binary-audit scripts, lint glue, and upstream watchers. | `decision-1` context | One tool, a declared manifest, and generated workflows |
| P2 | **No registry, no provider.** A conda channel, a crate index, a wheel index, and an npm surface are wanted on infrastructure the owner already has. | `decision-5` (CEP-15 `base_url`), `decision-119` §1-4 | Protocol-native static layouts on Pages, branches, or object storage |
| P3 | **Offline reproduction is unsolved and unverifiable.** A folder of downloads is not a lock; an installed prefix is not a portable payload. | `decision-102`, `decision-103`, `decision-109`, `decision-114`, `decision-115` | Locks as indexes, digests as the contract, a finite consumer |
| P4 | **Discovery.** What exists in this universe, for humans? | `decision-6`, `decision-119` §6 | A static portal that is a catalog, explicitly not a registry |

P1 is the origin: retire bespoke per-repository CI. P2 is what makes it self-hosted. P3 is
the differentiator. P4 is a reading surface over P2.

## The boundary invariant

`decision-119` states the load-bearing sentence, and every scope question resolves
against it: Universe *"validates configuration, runs explicitly configured external
build/test commands, collects their outputs, verifies and publishes artifacts/indexes, and
generates catalog/portal snapshots. It does not implement package compilers/build
systems."*

So Universe is an **orchestrator and verifier of declared commands**, plus an
**implementer of distribution formats** (indexes, catalog snapshots, locks). Nothing more.

| In scope | Out of scope |
| --- | --- |
| Reading lockfiles, generating indexes, verifying digests | Resolving dependencies |
| Running declared build and upload commands, validating their outputs | Building packages or compiling sources |
| Rendering workflows from a manifest | Being a CI platform |
| Restoring and staging offline views | Proxying or intercepting network traffic |
| Version parity, artifact identity, publication evidence | Deciding what version the project is |
| Composing release notes from declared inputs | Authoring changelogs or code |

## Refusals

These are refusals, not deferrals. Each one has been proposed at some point and rejected
for a stated reason, so that the next proposal can be judged on the reason rather than on
novelty.

1. **Universe does not become a build system.** It runs the command the manifest names.
   If a package cannot be produced by a declared command, that is a missing external tool,
   not a feature request.
2. **Universe does not solve dependencies.** A restore replays pins; it never resolves.
   The one place a solver is involved is the consumer's own package manager, which is not
   Universe.
3. **Universe does not become a transparent proxy.** `decision-119` forbids it explicitly;
   capture-based offline readiness is unverifiable and untestable.
4. **Universe does not own semantic versioning policy for user repositories.** It can
   compute a version from commits when asked, and verify parity between declared literals,
   but it does not invent a policy a consumer repository cannot state.
5. **Universe does not accept arbitrary user-supplied workflow steps.** A bounded command
   list is permitted (see `decision-124`); arbitrary YAML is not, because it would make the
   generated-workflow digest meaningless.
6. **Universe does not treat CI job artifacts as a registry.** Job artifacts are ephemeral,
   require authentication, and have no resolution semantics. They are an intra-workflow
   hand-off only.
7. **Universe does not ship a static registry surface for an ecosystem whose clients cannot
   consume one.** See `decision-127`; npm is the case in point.

## Three distribution surfaces

Every release produces, in this order, and the first is the origin for the rest:

1. **GitHub Release assets** — immutable, digest-addressed, anonymously readable. The
   origin.
2. **Static indexes** — protocol-native metadata at stable URLs, generated from the assets,
   for the ecosystems that have a documented static format.
3. **Hosted registries and portals** — upload or reference. A portal lists; a registry
   resolves; the two are not interchangeable.

The asymmetry that follows is recorded in `decision-126`: fan-out jobs fetch the release and
copy bytes, they never rebuild, because a rebuild cannot promise identical bytes.

## Consequences

- Scope arguments now have a home. A proposal that adds a compiler, a solver, or a proxy is
  answered by §Refusals rather than by re-reading the whole backlog.
- The remaining open questions are all *shape* questions, not scope questions: which
  ecosystems get a static surface (`decision-127`), what a generated workflow contains
  (`decision-124`), and how a release is cut and promoted (`decision-126`).
- This record deliberately contains no directory layout and no manifest schema; those are
  specifications (`doc-1`, `doc-2`) and change far more often than scope.
