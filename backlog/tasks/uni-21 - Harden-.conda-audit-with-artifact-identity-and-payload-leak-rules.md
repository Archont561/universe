---
id: UNI-21
title: Harden .conda audit with artifact identity and payload-leak rules
status: To Do
assignee: []
created_date: '2026-10-27 09:20'
updated_date: '2026-10-27 09:20'
labels:
  - audit
  - testing
milestone: m-2
dependencies:
  - UNI-10
documentation:
  - backlog/decisions/decision-10 - Adopt-ODM-conda-release-governance-and-supply-chain-hardening-patterns.md
  - backlog/docs/architecture/doc-2 - Universe Kit-CLI-and-manifest-specification.md
  - backlog/docs/architecture/doc-3 - Static-channel-and-SvelteKit-UnoCSS-portal-specification.md
priority: high
type: feature
ordinal: 21000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The ODM-conda `audit_package.py` proves that an artifact is exactly what its source-of-truth manifest claims and that nothing leaked into the package. Port those checks into the pure-Rust auditor started in UNI-10 (no Python, no external tar/zstd binaries):
- **Identity consistency**: the package filename must match `name-version-build.conda` derived from `info/index.json`; `name`, `version`, `build_number`, and `subdir` in the index must equal the values declared in the typed `PackageMeta`/plan inputs, and `about.json`'s SPDX expression must equal the recipe declaration.
- **Payload allowlist**: every `pkg-*.tar.zst` member must match an explicit prefix-relative allowlist (or a manifest-declared set); any `.git`, `.pixi`, `.cache`, `__pycache__`, absolute path, or other unexpected member fails the audit with a deterministic diagnostic instead of a warning.
- **Native checks**: executable members carry the expected ELF/PE/Mach-O magic for the target `subdir` (extending the goblin work in UNI-10), and every RPATH/RUNPATH entry is `$ORIGIN`-relative (reject absolute build-root leaks beyond the existing pattern checks).
- **Declared dependencies**: the runtime `depends` set includes every dependency the package actually links against and includes no undeclared system library outside `package.toml [audit].allowed_system_libs`.
- **License metadata**: non-empty `info/licenses/*` texts are present whenever `audit.enforce_license_file = true`, and the count is at least the number of declared notices.
Emit the findings through the UNI-30 `Diagnostic`/dotted-path machinery and add the results to the fragment's `audit_summary` (`FragmentJson`) so the portal can render them.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Synthetic `.conda` fixtures with a wrong filename/index identity mismatch, an unexpected payload member, a non-`$ORIGIN` RPATH, and a missing license file each fail the audit with a distinct stable diagnostic code; a clean fixture passes with zero diagnostics.
- [ ] #2 The auditor never writes outside its output directory and reads archives solely through the streaming UNI-10 path (no temp extraction of untrusted archives).
- [ ] #3 Audit verdicts are serialized into `FragmentJson.audit_summary` and round-trip through the ts-rs/schemars generated contracts without schema drift.
- [ ] #4 `rstest`/proptest coverage includes arbitrary but bounded payload trees proving no member outside the allowlist is ever accepted, and the new checks run under `cargo nextest` alongside the UNI-10 suites.
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 The checks are reusable by both the owner-lane release flow (UNI-20) and the consumer-lane `universe-kit audit` command, and schema coverage for the changed modules stays at or above the project threshold.
<!-- DOD:END -->
