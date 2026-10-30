---
id: DECISION-131
title: The release root is signed, and the bootstrap pins the key
status: accepted
created_date: '2026-10-31'
updated_date: '2026-10-31'
---

## Context

Every record about integrity answers the wrong half of the question. A lock names a sha256 per
artifact and `UNI-32` fails closed on anything that does not match; that proves the bytes are the
bytes the lock named. Nothing proves *who wrote the lock*. Searched across all decisions, docs,
and tasks on 2026-10-31, there is no key material of any kind: no `cosign`, no `minisign`, no
`gpg`, no public key, no signed commit, no transparency log.

The exposure is concrete rather than theoretical. Mirror branches are public, unprotected, and
mutable by anything holding `contents: write` (`decision-127`); a catalog is pinned as
`github:<owner>/<repo>@<commit>`, and a commit hash is a content address whose authority comes
from wherever the reader learned the hash. The falsifying test is one line: rewrite a lock,
republish the artifacts it names to a mirror branch, run `airlock restore --verify`. On the
design as it stands, it passes.

That is the difference between tamper-evident and tamper-proof, and "sealed" claims the second.

## Decision

**The release root is signed, the bootstrap carries the verification key, and sealed mode has no
unsigned path.**

### 1. What is signed

Every release publishes a detached signature over `universe-release.json`. That manifest names
each lock and its digest, so signing the manifest transitively signs the lock set: a verifier
checks the manifest signature once, then trusts the digests inside it. Artifacts are not
individually signed — they are covered by the manifest's digests, and signing thousands of blobs
buys nothing that signing their index does not.

### 2. How it is verified

The verifier is a public key compiled into the bootstrap at build time. Verification is offline,
needs no certificate authority, no transparency log, and no network, and runs before any state
changes. A missing, malformed, unknown-key, or non-matching signature fails closed.

There is no override in sealed mode. A developer-lane restore may continue without a signature,
because that lane is already builder-controlled, and when it does it says so in its diagnostics
rather than proceeding quietly.

### 3. The mechanism, chosen for the constraint

Ed25519 with a minisign-compatible detached format. It was chosen because the verifier must run
on a disconnected host: verification is a few hundred lines and one embedded key, with no CA
bundle, no Rekor lookup, and no clock dependency. Keyless signing tied to release-workflow OIDC
identity is recorded as the rejected alternative — it is the better answer to *attribution* and
the wrong answer to *offline*, because verifying a short-lived certificate offline requires the
whole bundle including its inclusion proof.

### 4. Custody

The private key lives only in the release lane as a repository secret, and only in a job that
runs no untrusted code — the signing step runs after promotion, never before, so a build that
never earns promotion never holds the key. Rotating it means publishing a bootstrap signed by
both the old and the new key, which is a ceremony that must be rehearsed before it is needed
rather than invented during an incident.

### 5. What the signature does and does not say

It says: this release came from the holder of key K. It does not say who K is, and the verifier
must not pretend otherwise. Attribution is governance — published in the repository and in the
bootstrap's release notes — and it is deliberately not inferred from the artifact.

## What argues against this

- **One key is one point of compromise.** A stolen release key forges a universe that verifies
  perfectly, and with no transparency log there is no public record that would expose it.
- **The first download is still trust-on-first-use.** Whoever tampers with the bootstrap itself
  can ignore its key entirely. The signature protects everything after the first binary; claiming
  more would be false.
- **Rotation is a bootstrap release**, which is awkward for a project whose bootstrap is supposed
  to be the stable thing.
- **It is new ceremony in the release lane**: a secret to hold, a step that can fail, and a
  promotion that now has a precondition it did not have.

## Conditions that would change this decision

- Restore hosts can be assumed online at verification time: keyless signing with a transparency
  log becomes strictly better, and attribution becomes checkable rather than asserted.
- A key compromise occurs: rotate, dual-sign, and publish a revocation entry consumed by the next
  bootstrap.
- Foreign catalogs or third-party publishers enter the model: per-publisher keys with a policy
  file replace one embedded key.

## Consequences

- `UNI-32` gains authenticity as a distinct property from integrity, with a fail-closed test for
  each of: absent signature, wrong key, tampered manifest, tampered artifact.
- `UNI-33` verifies the signature before it changes target state, using the compiled-in key
  rather than a fetched one.
- `UNI-54` signs after promotion, and `promote` refuses an artifact set that cannot be signed.
- `UNI-55` ships the public key inside the bootstrap artifact and owns the rotation procedure.
- `doc-106` gains authenticity as an evidence level separate from integrity, and `doc-108`
  records the key's custody and the rotation ceremony.
- The claim language tightens: "sealed" means sealed against substitution, not merely against
  corruption, and only from the first bootstrap onwards.
