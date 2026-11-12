---
id: UNI-33
title: Implement Airlock bootstrap verify and restore skeleton
status: In Progress
assignee: []
created_date: '2026-10-27'
updated_date: '2026-11-10'
labels:
  - airlock
  - bootstrap
  - offline
milestone: m-5
dependencies:
  - UNI-32
documentation:
  - backlog/decisions/decision-131 - The-release-root-is-signed,-and-the-bootstrap-pins-the-key.md
  - backlog/decisions/decision-136 - The-bootstrap-restores-a-closed-payload-into-an-immutable-environment-and-swaps-one-link.md
  - backlog/docs/doc-103 - Init-bootstrap-and-restore-lifecycle.md
priority: high
type: feature
ordinal: 33000
---

## Description
<!-- SECTION:DESCRIPTION:BEGIN -->
Give `universe-airlock-bootstrap` its minimal real verbs—`version`, `verify`,
`restore`, and `doctor`—while keeping adapters, prompts, TUI, HTTP serving, and
publishing out of the artifact. Restore must verify a complete local payload
before activation and preserve the previous active state on any error.
Generated POSIX/PowerShell entrypoints only locate and launch the pinned native
bootstrapper; they must not become an independent package manager. Keep this
consumer restore flow distinct from UNI-36's developer-toolchain transport.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Bootstrap verifies manifest and payload before changing target state. — 2026-11-08: `verify` is read-only and answers manifest → commit → artifact → authenticity in that order; `restore` runs the same verification before its first write. A process-level test asserts the state root does not exist after `verify`.
- [x] #2 Restore installs the pinned Pixi-managed `universe-airlock` from local payload and activates transactionally. — 2026-11-08: partially, and the boundary is deliberate: the payload is materialised into `envs/<version>-<manifest prefix>/` and activated by one `rename` plus one re-pointed symlink (`decision-136`). It does not yet hand installation to a bundled Pixi, because the payload format for a Pixi-managed environment is what `UNI-51`'s mirror publishes and does not exist yet; that half is tracked there. **2026-11-12:** installation no longer stops at copying the archives — `restore` now materialises a real prefix from each `.conda`, so the row is closer to its own wording, but it is still not a *Pixi-managed* environment: the prefix is built by this binary's own extractor (`UNI-52` records the choice) and `UNI-51` remains the Pixi half.
- [x] #3 Failure leaves the previous active state untouched. — 2026-11-08: tested both as a unit (`a_failure_leaves_the_previous_environment_active_and_intact`) and through the binary (`a_failed_restore_leaves_the_previous_version_active`), including that no staging directory is left behind.
- [x] #4 The release signature is verified against the key compiled into the bootstrap, never a fetched key, before any target state changes; the developer lane may restore unsigned only with a diagnostic that names the missing signature (`decision-131`). — 2026-11-08: the policy and its order are implemented and tested; the Ed25519 check is not. With no key compiled in, a *signed* payload is refused on both lanes (`airlock.signature.no-key`) rather than waved through, sealed mode refuses an unsigned payload, and the developer lane proceeds unsigned with a warning that names it. The verifier and the key land with `UNI-55`.
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [x] #1 Clean-environment fixture restores on Linux x86_64. — 2026-11-08: `the_handover_contract_restores_from_the_callers_working_directory` (renamed 2026-11-10) runs the real binary with the exact invocation the rendered entrypoint makes (`restore --commit <pin>`, no other flags) and asserts the artifact is materialised under the default state root. **2026-11-10 correction:** the original note said `cwd = the checkout`. The rendered entrypoint execs the binary from the pinned checkout but does not change the working directory, so the payload and state defaults resolve in the project the script was run from. `UNI-53` DoD #3 now proves both halves in one process tree.
- [ ] #2 `pixi run gates` passes.
<!-- DOD:END -->

## Notes
<!-- SECTION:NOTES:BEGIN -->
2026-11-06 — the entrypoint that will call this binary is rendered and tested (`UNI-53`,
`crates/universe-kit/src/airlock.rs`). It fixes the invocation the bootstrapper should accept:
`universe-airlock-bootstrap restore --commit <40-hex>`, with the pin verified by the entrypoint
before the handover, so the binary must still verify the payload against it.

**2026-11-10 correction of the paragraph above.** It originally said the binary is "run with
the pinned checkout as the working tree" and must "verify the payload inside it". Neither is
true. The entrypoint `exec`s the binary from the checkout but keeps the caller's working
directory, and the closed payload lives in the project's `.universe-airlock/payload` — it
cannot be inside the ref, because a release manifest that named the commit of the tree holding
itself could never be written. The text is corrected in place rather than left to mislead the
next reader.

2026-11-08 — the verbs exist: `version`, `verify`, `restore`, `doctor`. What is still open, and
why, so the next slice does not have to rediscover it:

- **No Ed25519 verifier.** `signature::RELEASE_KEYS` is empty, so any signature is refused. The
  arithmetic belongs with the key (`UNI-55`); a verifier with no trusted key and a key with no
  verifier are both worse than an honest refusal. `decision-136` records the split.
- **No Pixi handover.** The payload is materialised as files; nothing yet creates a Pixi
  environment from it. That needs the payload shape `UNI-51` publishes.
- **Dependencies went in through the relock bot, not around it.** The first cut carried a
  hand-written SHA-256 and a scratch-directory helper to avoid a relock mid-slice; both were
  removed in favour of `sha2` and `tempfile` (dev-only) once the manifests could be edited for
  the bot to lock, and the signature verifier came with them as `minisign-verify` — zero
  transitive dependencies, which is why it is the only crypto crate here.
- **The end-to-end loop was covered in two halves**, not one process tree: the rendered
  entrypoint is executed against a real bare repository in `universe-kit` (with a stub
  bootstrapper, because a test cannot portably reach another crate's binary path), and the real
  binary restores under the exact same invocation in this crate. Both are needed; neither alone
  proves the loop. **2026-11-10 correction:** this bullet describes the state at the time, not
  today. `UNI-53` DoD #3 publishes the real binary into the pinned ref and restores in one
  process tree; the portable seam is the directory that `CARGO_BIN_EXE_universe-kit` names.

2026-11-10 — the help text now says the payload and state defaults resolve under the caller's
working directory, and the handover test is named for that fact; `the_usage_says_where_the_payload_really_lives`
holds the correction. A sealed restore and a Pixi handover remain open for the reasons above.
<!-- SECTION:NOTES:END -->
