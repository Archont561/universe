---
id: DECISION-128
title: Rendered workflows expose one verdict job per file
status: accepted
created_date: '2026-10-31'
updated_date: '2026-10-31'
---

## Context

`decision-124` settled which files exist, what each contains, and the highest privilege each
file may hold. A renderer needs more than that: it must emit exact job identifiers, it must
be possible to add a job without silently changing what CI requires, and the privilege of a
job must be a property of the job rather than of the file it lives in.

Three specific gaps surfaced while specifying `universe-workflow`:

1. **Nothing said what branch protection should require.** If every job is required, then
   renaming any job turns a working repository into one with a permanently pending check,
   and the failure appears as "waiting for status" rather than as an error anyone can read.
2. **Privileges were stated per file.** A file-level `permissions:` block is inherited by
   every job that does not override it, so one publish job's `contents: write` quietly
   extends to every job in the file — including the ones running untrusted pull-request code.
3. **`[workflow] checks` was named as "a fixed list of commands"** without saying how many
   slots, which ones, or what a consumer does when it wants a fifth.

## Decision

**One verdict job per file, one write scope per job, and a closed check vocabulary.**

### 1. Every job of a pull-request file is a required check

| Rule | Consequence |
| --- | --- |
| The last job in every file is `<prefix> / verdict`, needing every sibling, running `if: always()`, failing on `skipped` | the file states its own outcome in one line |
| **Branch protection requires every job of every file that triggers on `pull_request`** | a specific failure is red in the settings page, not one aggregate |
| A file that does not trigger on `pull_request` contributes no required checks | its failure blocks the release rather than the merge |

Requiring every job is bounded by something that is easy to miss: **a required check must be
reported on the commit being merged.** `universe-publish.yml`, `universe-airlock.yml`, and
`universe-watch.yml` do not run on a pull request, so their jobs never report there, and
requiring one would leave every pull request waiting for a status that cannot arrive. The
required set is therefore exactly the jobs that run before a merge — every job of
`universe.yml`, and of the consumer-role file — and nothing from the other three.

What this buys and what it costs, stated plainly: a reviewer sees *which* job failed without
opening the run, and the cost is that renaming a job is now a repository-settings change. That
coupling is accepted deliberately, and made mechanical rather than remembered:
`universe ci render --print-required-checks` emits the list in the order the settings page
expects, so the edit is a copy. Renames are a template decision, recorded, not something a
refactor does in passing.

The `verdict` job survives this rule even though it is no longer the gate: it answers "did this
file pass" on a run whose matrix has twenty jobs, and it fails on `skipped` where a required
check would pass. That difference is the reason it stays.

### 2. Privileges are declared per job, and a job may hold at most one write scope

The workflow-level default is `contents: read`. Every job that needs more declares its own
block; no job inherits a write scope from its file. The write scope is named after what the
job writes, not after the ecosystem:

| What the job writes | Scope |
| --- | --- |
| release assets, drafts, mirror branches, rendered entrypoints | `contents: write` |
| a hosted package registry | `packages: write` |
| the Pages channel | `pages: write` |
| an upstream bump pull request | `pull-requests: write` |

`id-token: write` is listed separately because it is not a resource write: it is an OIDC
exchange, it grants no repository content on its own, and it may accompany whichever scope
needs it. The Pages channel therefore takes `pages: write` **and** `id-token: write`, and a
provenance attestation takes `id-token: write` alone.

**A job that appears to need two resource write scopes is two jobs.** This is the operational
form of `decision-124`'s privilege separation, and it is testable: the renderer refuses,
rather than merges, a job whose scopes exceed one.

### 3. `[workflow] checks` is a closed set of three static slots

The manifest may declare `format`, `lint`, and `typecheck`, each bound to exactly one command,
each rendered as one step inside the `validate` job in declaration order. An unknown slot is a
lint error; a known slot with no command is a lint error; the vocabulary does not grow by
configuration.

Three, not more, because `validate` is the static-check job: `build` already compiles, `audit`
is its own job, and `test` belongs with the artifact it exercises. A consumer that wants
end-to-end coverage composes it into its own `test` command — the escape hatch is the command,
not the vocabulary.

### 4. Two matrix shapes

| Job | Matrix over | Failure behaviour |
| --- | --- | --- |
| `build` | the declared platforms | default `fail-fast`, because a cross-platform matrix is one artifact set |
| `mirror` | the declared lock adapters | `fail-fast: false`, because one ecosystem's transport failure must not cancel another's branch |

### 5. Publish splits by destination, and serialises by package within one

One job per `writable` store, named `publish / <store name>`, plus `bootstrap-airlock` as its
own job because it writes a branch rather than a declared store.

The unit is the **store**, not the ecosystem and not the package, because the store is what
three separate mechanisms address: the journal key in `decision-126` is the destination, the
secret belongs to the destination, and the write scope follows the destination. Splitting by
ecosystem would put an npm store and a crates store in one job with two secrets and two
scopes, which §2 forbids. Splitting by package would multiply jobs without improving resume,
because the journal is keyed by destination anyway.

Within one store, packages publish **in declaration order, serially**. That is not a
simplification: for the `branch` and `static` kinds two concurrent writers race on the same
ref and on `repodata.json`, and the loser's index silently omits the winner's package.

What differs between a store that needs an index and one that does not:

| | `needs_index: true` (`static`, `prefix-dev`, `branch`) | `needs_index: false` (`releases`, `npmjs`, `github-packages`, `ghcr`) |
| --- | --- | --- |
| Upload | package bytes, then the regenerated index | package bytes through the store's own client |
| Index | generated by Universe, so it is reviewable before upload | generated by the store, and not ours to verify structurally |
| Verification | read the index back and assert every declared package sits at the built digest — the `file` verifier | read the store's own record back, e.g. `npm view` — the `registry` verifier |
| Failure that exists only here | bytes uploaded while the index was never updated | the store accepted the publish and mutated it later |

**Verification is placed by whether the read is credentialed.** A publish job verifies its own
destination immediately after writing it, and `verify-index` — the job `decision-124` already
lists — re-reads only the destinations whose reads are anonymous, holding no secret and no write
scope.

The reason is that independence is only real when the reader differs. For a store declaring
`read_auth: token`, a separate verification job must present the token that just published, so
it doubles the jobs without doubling the assurance. For a store declaring `read_auth: none`, a
job with no secrets at all is a genuinely different reader — the one a consumer will be — and
that check is kept. `read_auth` therefore never changes a publish job; it decides which job
verifies.

One case is structural rather than a preference: a `kind = "command"` store is always verified
in `verify-index`, whatever it declares, because self-verification there means trusting the exit
code of the thing that published.

The four placements and what each costs are worked through in `draft-7`; this is its option D.

**Every destination is mandatory, so publication is all-or-nothing.** One failed destination
fails the file: no destination's success is announced as a release until all of them have
succeeded. The record keeps that policy from `decision-126` and states the consequence here
because it is the publish jobs that pay for it — an expired npm token stalls a release whose
bytes are already on the channel, and the recovery is the resume path, not a partial release.

**The journal is a run artifact, not a branch.** Each publish job uploads
`journal-<store>.json` recording the destinations it completed, together with the digest it
published. Resume means re-running the failed jobs of the *same* run, which is exactly the
remediation `decision-126` describes; a fresh run starts with an empty journal and therefore
must refuse to start while a tag for the version exists. What is given up is cross-run resume:
there is no memory of a publish that happened two runs ago. That is accepted because every
cross-run memory is a thing to authenticate, expire, and audit, and the failure it would repair
— a lost artifact — is rarer than the failure it would create, a stale journal that claims a
destination succeeded.

### 6. Rendering owns the set of files, and deletion is part of rendering

The file set is derived from declared capabilities (`decision-124`), so a file whose capability
is gone is drift, not history. `universe ci render` deletes it; `universe ci check` fails and names it,
because the check never writes.

Deletion stays in `render` rather than in `check` for the same reason fixtures never mutate the
live repository: a check that repairs what it measures cannot be used to detect a change. A
consumer who removes their last store therefore sees one failing check naming one file, and
then a `render` that removes it — a two-step that is legible in a diff, which silently deleting
during a check would not be.

### 7. Action pins come from the renderer, not from the consumer

Third-party actions are pinned by full SHA, and the pins are **data the renderer carries**, not
values copied into each consumer's file by hand. Upgrading Universe upgrades the pins: a new
version embeds a new pin table, and the next `ci render` rewrites the pins in every emitted
file, so one bump here reaches every consumer as one commit they can read.

That is the whole point of the stamp. A consumer who pins by hand drifts; a consumer who pins by
tag takes whatever the tag points at today. A consumer who pins by SHA *and* gets those SHAs
from the tool they already upgrade has both immutability and a single place to review it.

### 8. Rendering is deterministic, with three permitted interpolations

A rendered file may contain a pinned commit, the setup line that distinguishes owner from
consumer mode (`decision-125`), and GitHub expressions untouched. It contains no timestamp, no
absolute path, no hostname, and no values read from the environment that rendered it. Two
renders of one manifest are byte-identical, which is what makes `ci check` meaningful and the
reviewed artifact — the YAML — the same bytes CI runs.

## Conditions that would change this decision

- **GitHub's required-check matching stops being name-based**, or gains per-workflow aggregates.
  Then requiring every job is no longer necessary, and the rename coupling in §1 is pure cost.
- **GitHub allows a required check to be satisfied by an absent workflow** — for example a
  required check scoped per workflow rather than per job. Then the tag-only files could join
  the required set and §1 would cover all four files instead of one.
- **Artifact retention becomes shorter than the interval between a failed publish and its
  repair.** Then the journal has to move out of run artifacts, and §5 is superseded rather than
  amended.
- **A job needs two write scopes for a reason that is not two jobs** — for example, if publishing
  an asset and attesting it cannot be split because the attestation must be produced in the same
  process. That would be a genuine limit of the rule, and would be recorded as one.
- **Consumers need different static slots.** If two independent consumers both need a fourth slot
  that cannot be composed into an existing command, the vocabulary was wrong, and the honest
  response is a wider vocabulary with a migration — not a free-form `checks` list.
- **A store kind makes per-package parallelism necessary** — for example a registry whose
  client refuses serial uploads inside the job's time limit. For that kind alone the split
  moves to one job per package, and the index rule above then has to say how two jobs avoid
  racing on one index.
- **`fail-fast: false` on `build` becomes necessary** because platforms diverge enough that a
  single failure hides unrelated ones. That is an observation about the artifact set, not a
  preference.

## Consequences

- The required-check set is one name per job of the pull-request file, printed by the renderer so
  a rename is a mechanical settings edit rather than a discovery.
- A file's privilege is the union of its jobs' privileges, and each job states its own — the
  renderer can prove the union rather than assert it.
- A `verdict` job exists in consumer repositories too. It reports the consumer role's four jobs
  (`plan`, `validate`, `restore-verify`, `verdict`) in the same shape, so one required-check rule
  covers both lanes.
- The renderer owns a finite switch for three slots, and printing an unknown slot is a
  template-level impossibility rather than a runtime surprise.
- `universe ci check` failures become attributable to a file and a job, because the name a
  reviewer sees in the Actions tab is the name the contract defines.
- `verify-index` holds no secrets at all, which makes it the one job in the publish file whose
  permissions are the workflow default — and the one whose result a consumer can reproduce
  locally with the same command the tool gives them.
