---
id: draft-7
title: Four ways to place verification in the publish file
status: draft
created_date: '2026-10-31'
updated_date: '2026-10-31'
labels:
  - architecture
  - release
  - ci
related_tasks:
  - UNI-54
  - UNI-56
---

## Origin

Open question in `decision-128` §5: `verify-index` must read every destination back, and
`decision-127` makes reads from a `github-packages` store credentialed. So an independent
verification job needs the same token that publishes. This draft lays out four placements and
what each costs, so the choice is made against a picture rather than a description.

All four assume the split in `decision-128` §5: one job per writable store, named
`publish / <store name>`, and one resource write scope per job. Only the placement of the
read-back changes.

## A — one verification job after every publish

The current proposal. Verification is one job that holds every read secret and no write scope.

```yaml
jobs:
  publish-releases:
    permissions: { contents: write }
  publish-npmjs:
    permissions: { contents: read, id-token: write }
    env: { NODE_AUTH_TOKEN: '${{ secrets.NPM_TOKEN }}' }
  publish-github-packages:
    permissions: { contents: read, packages: write }
    env: { NODE_AUTH_TOKEN: '${{ secrets.GHPKG_TOKEN }}' }

  verify-index:
    needs: [publish-releases, publish-npmjs, publish-github-packages]
    permissions: { contents: read }        # no write scope at all
    env:
      NPM_TOKEN: '${{ secrets.NPM_TOKEN }}'     # every read secret lives here
      GHPKG_TOKEN: '${{ secrets.GHPKG_TOKEN }}'
```

Secrets per job: 1 · 1 · 1 · **2**. Jobs: 4. A bad publish is confirmed by a job that never
wrote anything and cannot write anything.

## B — one verification job per destination

Each secret appears in exactly two jobs: the one that publishes and the one that verifies.

```yaml
  verify-releases:
    needs: [publish-releases]
    permissions: { contents: read }
  verify-npmjs:
    needs: [publish-npmjs]
    permissions: { contents: read }
    env: { NPM_TOKEN: '${{ secrets.NPM_TOKEN }}' }
  verify-github-packages:
    needs: [publish-github-packages]
    permissions: { contents: read }
    env: { GHPKG_TOKEN: '${{ secrets.GHPKG_TOKEN }}' }
```

Secrets per job: 1 each. Jobs: 6. Same independence as A, without concentrating the secrets —
at the cost of two more jobs and two more setup steps per release.

## C — verification inside the publishing job

```yaml
  publish-npmjs:
    permissions: { contents: read, id-token: write }
    env: { NODE_AUTH_TOKEN: '${{ secrets.NPM_TOKEN }}' }
    steps:
      - run: universe publish --store npmjs
      - run: npm view @scope/name@version dist.integrity   # same job, same token
```

Secrets per job: 1 each. Jobs: 3. Attribution is immediate — the failing job is the destination
— but the credential that published is the credential that confirms, so a store that accepts a
wrong artifact and reports it back consistently passes.

## D — split by whether reads are credentialed

Publish jobs verify their own destination, and one independent job re-checks only the stores
whose reads are anonymous.

```yaml
  publish-npmjs:
    permissions: { contents: read, id-token: write }
    steps: [ publish, verify-self ]              # npm reads are anonymous — see note
  publish-github-packages:
    permissions: { contents: read, packages: write }
    steps: [ publish, verify-self ]              # read_auth: token — independence was nominal

  verify-index:
    needs: [publish-releases, publish-static]
    permissions: { contents: read }               # no secrets, no write scope
    steps:
      - run: universe verify --store releases --anonymous
      - run: universe verify --store pages --anonymous
```

Secrets per job: 1 · 1 · **0**. Jobs: 3.

The note is the whole argument: for a store that declares `read_auth: token`, a separate
verification job must present the same token, so it adds a job without adding independence. For
a store that declares `read_auth: none`, a separate job with no secrets at all is genuinely a
different reader, and that is worth the job.

## Comparison

| | Jobs | Max secrets in one job | Independent reader | Bad publish caught by |
| --- | --- | --- | --- | --- |
| A | 4 | 2 (all of them) | yes, for every destination | a job with no write scope |
| B | 6 | 1 | yes, for every destination | a job with no write scope |
| C | 3 | 1 | no | the job that published |
| D | 3 | 1 | yes, where it means something | a credential-free job for public stores |

## Recommendation

**D.** Concentration is the thing to avoid — A puts every read token in one job for the sake of
independence it only actually delivers where reads are anonymous. B delivers that independence
everywhere, but pays two jobs for two destinations where the verifier and the publisher present
an identical credential, which is independence in name only. D spends the job where the property
exists and declines to pretend elsewhere.

What would change this: if `decision-127`'s `github-packages` store gains anonymous reads for
public packages, D collapses into B for that destination. If a store's client cannot verify its
own publish at all — the `command` escape hatch in `decision-127` — then it must land in the
independent job regardless of `read_auth`, because self-verification there means trusting the
command's exit code.
