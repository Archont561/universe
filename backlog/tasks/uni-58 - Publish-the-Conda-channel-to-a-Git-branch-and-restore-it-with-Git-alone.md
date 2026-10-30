---
id: UNI-58
title: Publish the Conda channel to a Git branch and restore it with Git alone
status: To Do
assignee: []
created_date: '2026-11-01'
updated_date: '2026-11-01'
labels:
  - channel
  - release
  - git
milestone: m-12
dependencies:
  - UNI-11
  - UNI-12
documentation:
  - backlog/decisions/decision-5 - Static-Conda-channel-on-GitHub-Pages-+-GitHub-Releases-via-CEP-15-base_url,-CEP-16-shards,-and-immutable-JSON-fragments.md
  - backlog/decisions/decision-114 - Pinned-Git-catalogs-and-selective-artifact-retrieval.md
  - backlog/decisions/decision-127 - Manifest-facets-packages-stores-and-airlock-scopes.md
  - backlog/decisions/decision-132 - v0.1.0-is-a-pre-release-packaging-milestone,-not-a-v1-cut.md
  - backlog/docs/architecture/doc-3 - Static-channel-and-SvelteKit-UnoCSS-portal-specification.md
priority: high
type: feature
ordinal: 58000
---

## Description
<!-- SECTION:DESCRIPTION.BEGIN -->
Publish a channel as a tree on a Git branch: the audited `.conda` bytes and the
CEP-15/16 metadata side by side, at standard subdir paths, in one deterministic commit.
A consumer then restores by cloning that branch, because Git is the one transport
`UNI-51` already established for this repository's mirror and the one thing a
disconnected host is likely to have.

`decision-5` puts the bytes on "static hosting or object storage" and leaves the host
open; a branch is one of those, and it is the only one that needs no HTTP endpoint in
front of it. The cost is real and belongs in the record rather than in a footnote: a
Git branch is a poor CDN. Every published build grows the repository, cloners pay for
history they do not want, and GitHub's per-file and per-repository limits arrive long
before a channel is large. So this transport is scoped to the `v0.1.0` checkpoint and to
a channel holding one package, with retention pruning as an acceptance criterion rather
than a later improvement. When the channel outgrows it, publication to a hosted store
(`UNI-11`) is already the general path and nothing here has to be unpicked.

Two constraints keep it honest. Publication is idempotent: re-running against the same
artifacts produces no commit at all, so a retried job cannot manufacture a diff. And it
fails closed: a fragment whose digest disagrees with the bytes beside it stops the
publish, naming the artifact, rather than writing a channel that a solver will choke on
later.

The branch is an ordinary public branch, like the mirror branches in `decision-127`.
Nothing here depends on a ruleset, and a future ruleset that matches it would fail the
publish at the push rather than silently — which is the behaviour to keep in mind before
adding one.
<!-- SECTION:DESCRIPTION.END -->

## Acceptance Criteria
<!-- AC.BEGIN -->
- [ ] #1 Publication writes the `.conda` bytes and the CEP-15/16 metadata for a subdir into one branch at standard paths, in a single deterministic commit; two runs over identical artifacts produce no second commit.
- [ ] #2 A fragment whose `sha256` disagrees with the bytes beside it fails the publish, exits non-zero, and names the artifact — no channel is written.
- [ ] #3 A `pixi` solve against the branch, checked out as a local directory with network egress denied, resolves, downloads, and installs the package, proving the tree is complete without a CDN.
- [ ] #4 Retention pruning keeps the declared number of builds per version, prunes deterministically, and asserts the channel still solves afterwards; a pruned artifact that a pin still references fails closed.
- [ ] #5 Every test drives a temporary Git repository fixture. No test writes to, reads from, or assumes a branch of the live repository.
- [ ] #6 The published tree carries no timestamp or run identifier that would make two identical publishes differ, so the "no second commit" criterion is a property of the content rather than of the clock.
<!-- AC.END -->

## Definition of Done
<!-- DOD.BEGIN -->
- [ ] #1 `rstest-bdd` scenario (`channel.feature`) states the loop in domain language: an audited package is published to the branch, and a consumer restores it with Git alone.
- [ ] #2 `pixi run gates` passes.
<!-- DOD.END -->
