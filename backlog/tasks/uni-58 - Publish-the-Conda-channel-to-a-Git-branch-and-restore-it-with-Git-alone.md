---
id: UNI-58
title: Publish the Conda channel to a Git branch and restore it with Git alone
status: Done
assignee: []
created_date: '2026-11-01'
updated_date: '2026-11-08'
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
CEP-15/16 metadata side by side, at standard subdir paths, in one commit with no
parents. A consumer then restores by cloning that branch, because Git is the one
transport this repository already relies on for its developer bundle and the one thing a
disconnected host is likely to have.

`decision-5` puts the bytes on "static hosting or object storage" and leaves the host
open; a branch is one of those, and it is the only one that needs no HTTP endpoint in
front of it. The cost is real and belongs in the record rather than in a footnote: a Git
branch is a poor CDN. Every published build grows the repository, cloners pay for history
they do not want, and GitHub's per-file and per-repository limits arrive long before a
channel is large. The measurements that make this concrete are worth carrying into the
implementation: a blob above 100 MiB is refused outright, a repository is soft-limited to
about 1 GB packed, and a single push to about 2 GB.

Three shape decisions follow from that, and each is an acceptance criterion below rather
than a preference:

- **One snapshot, never a changelog.** The branch is orphaned and force-pushed, so it
  holds exactly one tree. History is replaced, not appended.
- **Whole files, never fixed-size chunks.** Git content-addresses files, so an unchanged
  `.conda` costs nothing on the next publish. Cutting every file into even pieces would
  destroy that; only a single file above the blob ceiling is split, into `.partNNN`
  pieces.
- **Budgets refuse before the push.** A blob ceiling, a transport ceiling, and a staged
  push ceiling, each reporting the limit it hit and the input that hit it.

Retention is stated as what it is. `--keep N` rebuilds the branch to the N most recent
snapshots, which bounds what the branch *serves*; a force-push followed by a server-side
`gc` does not shrink an orphan repository, so the command says so instead of implying it
reclaimed space.

Two more constraints keep it honest. Publication is idempotent: re-running against the
same artifacts produces no commit at all, so a retried job cannot manufacture a diff. And
it fails closed: a fragment whose digest disagrees with the bytes beside it stops the
publish, naming the artifact, rather than writing a channel a solver will choke on later.

The branch is an ordinary public branch, like the mirror branches in `decision-127`.
Nothing here depends on a ruleset, and a future ruleset that matches it would fail the
publish at the push rather than silently — which is the behaviour to keep in mind before
adding one.

The branch documents itself: a generated `README.md` carries the subdir table, the digest
column, and the exact `pixi.toml` channel stanza a consumer pastes, rendered from the same
fragments the metadata comes from. A channel whose usage instructions are re-derived by
each consumer is a channel with as many readings as it has consumers.
<!-- SECTION:DESCRIPTION.END -->

## Acceptance Criteria
<!-- AC.BEGIN -->
- [x] #1 Publication writes the `.conda` bytes and the CEP-15/16 metadata for a subdir into one orphan branch — a single commit with no parent, force-pushed — so the branch is a snapshot rather than a changelog; two runs over identical artifacts produce no second commit. 2026-11-06: the tree also carries each artifact's audit receipt at `meta/<subdir>/<filename>.conda.json` (`decision-5`, amended), copied verbatim from what `universe audit` wrote. — 2026-11-04: `universe publish --remote <path> --branch <name>` builds the tree, force-pushes one parentless commit (`git commit-tree` + `git push --force`), and republishing identical fragments reports `PushOutcome::Unchanged` and pushes nothing. Proven against a bare repository the test creates.
- [x] #2 A fragment whose `sha256` disagrees with the bytes beside it fails the publish, exits non-zero, and names the artifact — no channel is written. — 2026-11-04: `verify` hashes every artifact before a tree is assembled and `publish` and `index` share it; the failure names the artifact path and the code is `channel.artifact.digest` / `channel.artifact.size`.
- [x] #3 Files are published whole and only a single file above the 95 MiB blob ceiling is split into `.partNNN` pieces; a test asserts that republishing after a one-file change adds only that file's bytes to the new tree, which is the property fixed-size chunking would destroy. — 2026-11-04: whole files are the transport unit and `ChannelTree::split_oversized` cuts only `.conda` files, above the store's `budgets.blob`; the blob-reuse test asserts an unchanged artifact keeps its git blob across publishes. — 2026-11-08: `the_real_blob_ceiling_splits_a_file_of_ninety_five_mebibytes` drives the production constant rather than a scaled-down one — a `.conda` of `BLOB_CEILING + 1` bytes becomes exactly two parts of 95 MiB and 1 byte, and a file that fits is not cut. `rattler-build` was never the obstacle: the splitter only cares that the file ends in `.conda`.
- [x] #4 Budgets are enforced before anything is pushed — blob, total transport, and staged push size — and a refusal names the limit, the measured value, and the manifest or config field that would change it. — 2026-11-04: `[stores.budgets]` in the manifest narrows `blob`, `transport`, and `staged`; each refusal prints the limit, the measured value, and the field, and happens before any staging directory is created. One trap found while writing the scenarios: a publish that omits `--store` measures against the crate defaults, so a budget declared in the manifest is not applied — the rendered job always names its store, but a hand-run publish can walk past a ceiling the manifest states. Requiring `--store`, or resolving the single writable store, is follow-up work, not a change this task's scenarios made.
- [x] #5 Git access sits behind a semantic seam (`publish`, `fetch_into`) with an in-memory fake, so a failed push, a diverged remote, and an idempotent republish are all tested without a network or a real `git`; a command-recording runner covers the exact argv a dry run would issue. — 2026-11-04: `universe_channel::transport::Transport` is the seam with `MemoryTransport` as the in-memory fake covering a refused push, an idempotent republish, and a diverged remote. — 2026-11-08: the seam gained `staged_tree`, so `--check` no longer needs Git either, and `publish` holds a `Box<dyn Transport>` rather than a `GitTransport`. `git.rs` runs every command through `GitRunner`; `the_exact_argv_a_publish_issues_is_one_init_one_commit_tree_and_one_forced_push` records it (`init --quiet`, `rm -r --cached`, `add --all --force`, `write-tree`, `ls-remote`, `commit-tree … -m …`, `push --quiet --force …`), and `a_token_travels_in_the_environment_and_never_in_argv` keeps the credential out of it.
- [x] #6 Publishing never writes into the payload directory: the index and objects go to scratch, removed afterwards, and a symlink in the payload is refused up front because a transport contains plain files. — 2026-11-04: the tree the verb assembles is scratch rather than payload. — 2026-11-08: staging moved out of the project entirely (`scratch.rs`): a temporary directory by default, removed on `Drop` whether the publish succeeded or failed, and `--output` is refused when it sits where the artifacts are (`publish.scratch.inside-payload`). A symlink is refused twice over — a fragment reached through one and an artifact that *is* one (`channel.payload.symlink`), checked with `symlink_metadata` before any read follows the link.
- [x] #7 `--check` verifies every fragment against its bytes and every declared digest against its file, using the same code path the publisher runs, and stops before any push. — 2026-11-04: `publish --check` runs the same read/verify/render path, stages the tree locally to obtain its git tree id, and compares it with the branch, stopping before any push; drift reports both tree ids and the content digest.
- [x] #8 A `pixi` solve against the branch, checked out as a local directory with network egress denied, resolves, downloads, and installs the package, proving the tree is complete without a CDN. — 2026-11-06: `crates/universe-kit/tests/channel_resolves.rs` publishes a real `.conda` (built in the test, so `rattler-build` is not needed) to a bare repository, clones the branch with `git`, writes a `pixi.toml` that names nothing but the checkout, and runs `pixi install --offline`; the installed entrypoint is then run and prints its version. `--offline` is a stronger claim than a denied egress: the solve cannot reach anything. Two layout facts the client settled: a solve asks for `noarch` whatever the project builds, and `.conda` entries belong under `packages.conda` with an empty legacy `packages`.
- [x] #9 Retention prunes deterministically to the declared number of builds per version, asserts the channel still solves afterwards, fails closed when a pruned artifact is still pinned, and states in its own output that a force-push bounds what is served rather than what is stored. — 2026-11-08: `universe_channel::retention::prune` keeps the highest build numbers of each `(package, version)`; `--keep N` overrides the store's `max_builds_per_version`, which now applies to `branch`/`static` stores as well as releases. Order-independence is a test, not a hope. A pruned build that a surviving fragment pins exactly — `name ==version=build` or `name version build`; a range is not a pin — refuses the publish (`publish.retention.pinned`) instead of publishing metadata nothing satisfies. The report says, in human and JSON form (`reclaims_storage: false`), that the force-push bounds what the branch serves rather than what the repository stores. `a_retained_branch_still_solves_and_serves_only_the_build_it_kept` publishes two builds with `--keep 1`, clones, and resolves the result with `pixi install --offline`.
- [x] #10 The branch carries a generated `README.md` — subdir table, digests, and the consumer's channel stanza — rendered from the fragments by a pure function, with two renders byte-identical. — 2026-11-04: `universe_channel::readme::render` is pure, carries the subdir/package/version/build/size/sha256 table and the `[project] channels` stanza (the URL when the store declares one, otherwise how to serve the tree), and two renders of one fragment set are byte-identical.
- [x] #11 Every test drives a temporary Git fixture or the in-memory fake. No test writes to, reads from, or assumes a branch of the live repository. — 2026-11-04: every publish test creates its own bare repository under a `TempDir`; nothing reads or writes a branch of this repository.
- [x] #12 The published tree carries no timestamp or run identifier that would make two identical publishes differ, so "no second commit" is a property of the content rather than of the clock. — 2026-11-04: `repodata.json` carries no `timestamp` and no `md5`, the commit message is derived from the package list only, and the commit identity and dates are fixed, so the commit id is a function of the content.
<!-- AC.END -->

## Definition of Done
<!-- DOD.BEGIN -->
- [x] #1 `rstest-bdd` scenario (`channel.feature`) states the loop in domain language: an audited package is published to the branch, and a consumer restores it with Git alone. — 2026-11-04: `crates/universe-kit/tests/features/channel.feature` holds five scenarios driven by `tests/channel_bdd.rs` against a bare repository and a real `git clone --depth 1 --branch channel` under a `TempDir`: publish-then-restore (the clone's package hashes to the digest its metadata names), republish-is-a-no-op (one commit), a tampered artifact refuses publication, a store budget refuses the snapshot before any push, a rebuilt package makes the branch drift, and the audit receipt travels in the tree with the digest its artifact hashes to. Recorded honestly as **acceptance automation written after the publisher shipped**, not as behaviour-first design: the loop existed and these scenarios pin what it does.
- [x] #2 A property test proves the metadata is a pure function of the fragment set, so any permutation of input fragments yields byte-identical output. — 2026-11-04: `channel::permutation_tests` rotates and reverses arbitrary fragment selections and asserts the rendered file set is unchanged; the renderer sorts by channel path first.
- [x] #3 `pixi run gates` passes. — 2026-11-04: 16 gates, exit 0.
<!-- DOD.END -->
