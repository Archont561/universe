---
id: DECISION-128
title: Rendered workflows expose one verdict job per file
status: proposed
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

### 1. Every rendered file ends with exactly one verdict job

| Rule | Consequence |
| --- | --- |
| The last job in every file is `<prefix> / verdict` | the file has one place where its outcome is stated |
| It lists every other job in the file in `needs` and runs with `if: always()` | it runs even when a dependency failed, so the check always reports |
| It fails unless every dependency concluded `success` | a skipped dependency is a failure, not a pass |
| **Branch protection requires only the verdict jobs** | adding or renaming any other job cannot change the required-check set |

The third row is the one that carries weight. A `verdict` that treats `skipped` as acceptable
can report green on a run that never built anything, which is exactly the shape of failure
that looks like success in a pull request. There is no `optional` marker today; a job that may
legitimately not run belongs in another file with its own trigger, per `decision-124`.

The fourth row is why this is a decision rather than a detail. The alternative — requiring
each job individually — was rejected because it couples the required-check list, which lives
in repository settings, to the job inventory, which lives in a template. The two would drift
on the first rename, and the drift is invisible until a release is blocked.

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

Verification stays in the single `verify-index` job that `decision-124` already lists, after
every publish job, rather than inside each one. That job therefore holds the read secrets for
every destination — including stores where `read_auth: token` makes reads credentialed — and
still holds no write scope, so §2 continues to hold. `read_auth` never changes a publish job:
it changes what the consumer lane and `verify-index` must present.

### 6. Rendering is deterministic, with three permitted interpolations

A rendered file may contain a pinned commit, the setup line that distinguishes owner from
consumer mode (`decision-125`), and GitHub expressions untouched. It contains no timestamp, no
absolute path, no hostname, and no values read from the environment that rendered it. Two
renders of one manifest are byte-identical, which is what makes `ci check` meaningful and the
reviewed artifact — the YAML — the same bytes CI runs.

## Conditions that would change this decision

- **GitHub's required-check matching stops being name-based**, or gains per-job aggregates. Then
  the verdict job loses its purpose and should be deleted rather than kept as ceremony.
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

- The required-check set is one name per rendered file, and it never changes when jobs are added.
- A file's privilege is the union of its jobs' privileges, and each job states its own — the
  renderer can prove the union rather than assert it.
- A `verdict` job exists in consumer repositories too. It reports the consumer role's four jobs
  (`plan`, `validate`, `restore-verify`, `verdict`) in the same shape, so one required-check rule
  covers both lanes.
- The renderer owns a finite switch for three slots, and printing an unknown slot is a
  template-level impossibility rather than a runtime surprise.
- `universe ci check` failures become attributable to a file and a job, because the name a
  reviewer sees in the Actions tab is the name the contract defines.
