---
id: DECISION-136
title: The bootstrap restores a closed payload into an immutable environment and swaps one link
status: accepted
created_date: '2026-11-08'
updated_date: '2026-11-08'
---

## Context

`UNI-33` gives `universe-airlock-bootstrap` its verbs. Three things had to be decided that
`doc-103` states as responsibilities rather than as shapes: what a *payload* is on disk, what
"activate transactionally" means for a target that has no package manager running yet, and
what the bootstrap does about `decision-131`'s signature when no key exists.

The constraint that shapes all three is that the bootstrap is the one binary a disconnected
host runs, so it may not: fetch anything, assume a language ecosystem, leave a target in a
state a human has to repair, or accept bytes nobody described.

## Decision

**A payload is a closed, flat directory.** `universe-release.json` (the existing release
manifest), one file per artifact it names, and an optional `universe-release.json.minisig`.
Verification is read-only and answers four questions in a fixed order — manifest readable and
valid, commit matches the pin, every artifact present with the right size and digest, payload
authentic — because a truncated download and a tampered artifact have different remedies.
Anything in the directory the manifest does not describe is refused: bytes nothing verified
are not part of a restore.

**Activation is a rename, not an edit.** A state root holds `envs/<version>-<manifest
prefix>/`, a `current` symlink and a `previous` symlink. A restore materialises the payload
into `envs/.staging-<pid>` — same filesystem, so the final step is one atomic `rename` — then
repoints `current` by writing a temporary link and renaming it over the old one. Two
consequences, both tested through the binary as a host runs it: nothing is ever written where
`current` points, and a failure removes the staging directory and leaves `current` untouched.
Restoring the same payload twice copies nothing the second time, because the environment's
name is derived from the manifest's bytes rather than from a clock or a run id.

**Each environment carries its own receipt**, `key=value` lines rather than JSON, so `doctor`
can describe what is active without the payload it came from. The bootstrap has no JSON
writer and adds no dependency for one; this record is written and read by the bootstrap alone.

**The signature policy is implemented; the verifier is not.** `signature::RELEASE_KEYS` is
empty and `signature::assess` refuses *any* signature it cannot check, on both lanes, with
`airlock.signature.no-key`. Sealed mode refuses an unsigned payload outright; the developer
lane proceeds and says, in a warning, that nothing proves who wrote the payload. That is
`decision-131`'s split, and it is the only ordering that cannot be mistaken for verification.
The Ed25519 arithmetic lands with `UNI-55`, which ships the key and owns rotation — a key
without a verifier is a claim, and a verifier without a trusted key is a suggestion.

## Consequences

- The `restore` verb is `restore --commit <40-hex>`, which is what the rendered entrypoint
  already calls; the pin is checked against the manifest's commit, so a payload built from
  another commit is refused before anything is written.
- `verify` and `doctor` never write. A test asserts the state root does not exist after
  `verify`, because "read-only" is a property worth failing a build over.
- Activation is Linux-only by construction: replacing a symlink atomically is `rename(2)`.
  That is the same platform scope as `decision-132`, and a non-Unix build reports it rather
  than pretending.
- A payload whose files change between verification and copying is caught by hashing while
  writing, and reports `airlock.payload.changed`. The window between the two is not closed for
  an attacker who can already write to the payload directory; it is closed for a concurrent
  writer, which is the case that happens.
- Sealed restores are not possible until `UNI-55` lands a key and a verifier. That is a
  deliberate refusal, not a gap: the v0.1.0 release is a pre-release packaging milestone
  (`decision-132`), and its restore lane is the developer one.
- The payload's default location is `<checkout>/.universe-airlock/payload`, and the state
  root's is `<checkout>/.universe-airlock/state` unless `UNIVERSE_AIRLOCK_STATE` says
  otherwise. A consumer host will move state out of the checkout; `doc-103` still records that
  the target-side state directory is undecided, and this makes it a flag, not a rewrite.

## Alternatives

- **Install into a Pixi environment from the payload with bundled Pixi.** That is where this
  is going (`UNI-51`'s mirror publishes artifacts; `UNI-52` restores them), but the payload
  format for a Pixi-managed environment does not exist yet, and inventing it here would mean
  guessing what the mirror lane publishes.
- **Copy into a versioned directory and never re-point anything.** Rejected: then "which
  environment is active" is a file a reader has to interpret, and a half-finished copy is
  indistinguishable from a finished one.
- **Write the receipt as JSON.** Rejected for now: it needs a JSON writer in the bootstrap,
  and the record is internal. If it ever becomes public it becomes JSON, and the schema field
  is what makes that migration refusable.
- **Allow the developer lane to accept an unverifiable *signature*.** Rejected. An unsigned
  payload is a known state; a signature this binary cannot check is an unknown one, and
  treating the two the same is how "it has a signature" becomes a reason to stop looking.
- **Refuse to run `verify` on the developer lane without a key.** Rejected: `verify` is how a
  builder confirms their own payload, and refusing it would push them to `restore`.
