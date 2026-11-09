---
id: UNI-32
title: Implement Airlock sealed snapshot verify vertical slice
status: In Progress
assignee: []
created_date: '2026-10-27'
updated_date: '2026-11-09'
labels:
  - airlock
  - offline
  - testing
milestone: m-5
dependencies:
  - UNI-30
  - UNI-31
priority: high
type: feature
ordinal: 32000
---

## Description
<!-- SECTION:DESCRIPTION:BEGIN -->
Implement the smallest `universe-airlock` consumer slice: read a sealed
manifest, verify every object in a SHA-256 content-addressed store, expose a
local native view, and activate state transactionally. Treat archive input as
untrusted: validate digest syntax before deriving paths, stream large objects
with bounded resource use, stage writes before activation, and fail closed on
mismatch or corruption. The binary contains no producer, build, watcher, or
publishing capability. Do not reuse placeholder or non-cryptographic CAS code.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 A fixture snapshot verifies completely and is consumable through a native client view offline.
- [x] #2 Missing, extra, corrupted, and wrong-platform objects fail closed with structured diagnostics.
- [x] #3 Tests prove no network is required after import.
- [x] #4 A release manifest with an absent, unknown-key, or non-matching signature fails closed before any artifact is verified, and the test proves a structurally valid bundle with correct digests is still rejected (`decision-131`).
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 Evidence levels distinguish inspected, verified, and unknown artifacts.
- [x] #2 `pixi run gates` passes.
<!-- DOD:END -->

## Notes
<!-- NOTES.BEGIN -->
**2026-11-09 — the consumer can verify.** `universe-airlock verify --payload <dir>` is
read-only and offline. It assesses authenticity first (before a single artifact byte is
hashed), then calls the bootstrap's `payload::verify` for the manifest, size, digest, and
closed-set checks. A refusal names a stable code on stderr and exits 1; stdout stays empty,
including in `--json` mode. A successful `--json` run emits one versioned envelope.

**Red before green.** The scenarios and process tests were written against the consumer that
only knew `version`:

```
pixi run --frozen cargo nextest run -p universe-airlock --all-features --locked --no-fail-fast
  FAIL airlock_bdd a_sealed_snapshot_verifies_completely
  FAIL airlock_bdd a_missing_object_is_refused
  FAIL airlock_bdd a_corrupt_object_is_refused
  FAIL airlock_bdd an_extra_object_is_refused
  FAIL airlock_bdd a_wrong_platform_object_is_refused
  FAIL airlock_bdd the_sealed_lane_refuses_an_unsigned_payload
  FAIL airlock_bdd a_signed_payload_is_refused_before_artifacts
  FAIL verify the_binary_verifies_a_payload_with_no_reachable_network
  FAIL verify the_recipe_fixture_verifies_from_the_installed_layout
  FAIL verify verify_json_is_one_versioned_envelope_and_errors_stay_off_stdout
  Summary 24 tests run: 14 passed, 10 failed, 0 skipped
```
Every failure was `error: unknown argument: verify`. Green after the slice: 25 tests.

**Corrections recorded as corrections.** (1) The corrupt-object fixture was first written
with 19 bytes where the manifest named 18, so the size check fired before the digest check.
It was made the same length as the original artifact; the test still observes a digest
mismatch, not a weakened assertion. (2) My earlier plan said to flip
`verify_artifact`/`verify_published` in `universe.toml`. Those flags render `universe-kit`'s
`verify --artifact` / `--published` / `--index` commands, not the consumer's verb. They stay
false and the misleading comment in `universe.toml` now says which verb each flag gates.
(3) The smoke command stays `--version`: it is a build-time installability check with no
payload available. The recipe test is where the installed consumer demonstrates verification.

**What AC#2 covers, and what it does not.** Missing (`airlock.payload.missing`), extra
(`airlock.payload.extra`), corrupted (`airlock.payload.digest`, and `.size`), and an object
addressed under another platform's subdir (`airlock.payload.name`, because a payload is a
closed flat set) all fail closed with structured codes. AC#1's "native client view" and
DoD#1's evidence levels are **not** met: the report has one verified level. AC#3's test runs
the binary with every proxy pointed at a dead address; that is a regression guard, not a
proof that no network path exists in the code, which follows from the verifier taking only a
path. AC#4 is met for absent signatures (sealed lane), unknown keys and non-matching
signatures (no key compiled in, so any signature is refused), with a scenario proving a
structurally valid signed payload is still refused and another proving a signed payload with
a missing artifact reports only the signature code. The real Ed25519 key lands with `UNI-55`.

**Why the consumer depends on the bootstrap library.** `decision-14` names `universe-core`
the shared library and says hashing belongs to the component that owns the artifact; moving
hashing into core would reverse a documented boundary. `decision-122` calls the bootstrap a
phase of Airlock rather than an edition, so reusing its verification library keeps one set of
integrity rules. If the two executables ever need different rules, that is the moment to
extract a shared crate — not a change to make speculatively now.
<!-- NOTES.END -->
