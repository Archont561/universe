---
id: UNI-10
title: Implement streaming .conda and pure-Rust goblin binary/license auditor + fragment emitter
status: In Progress
assignee: []
created_date: '2026-10-26 20:15'
updated_date: '2026-11-01 09:20'
labels:
  - audit
  - channel
  - testing
milestone: m-2
dependencies:
  - UNI-30
  - UNI-31
documentation:
  - backlog/docs/architecture/doc-2 - Universe Kit-CLI-and-manifest-specification.md
  - backlog/docs/architecture/doc-3 - Static-channel-and-SvelteKit-UnoCSS-portal-specification.md
  - backlog/decisions/decision-3 - Hybrid-Rattler-library-+-subprocess-architecture-and-pure-Rust-rustls-dependency-policy.md
  - backlog/decisions/decision-5 - Static-Conda-channel-on-GitHub-Pages-+-GitHub-Releases-via-CEP-15-base_url,-CEP-16-shards,-and-immutable-JSON-fragments.md
priority: high
type: feature
ordinal: 10000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Replace `CloudCompare-conda/scripts/audit_package.py` entirely in pure Rust via `rattler_package_streaming`, `rattler_digest`, `spdx`, and `goblin` (`doc-2`, `doc-3`, `decision-3`, `decision-5`):
- Stream `info-*.tar.zst` and `pkg-*.tar.zst` from `.conda` archives without external `tar` or `zstd` binaries; verify `sha256` and `md5` archive digests and `info/paths.json` integrity.
- Verify `info/about.json` SPDX license validity and non-empty bundled `info/licenses/*` files when `audit.enforce_license_file = true`.
- Inspect compiled binaries cross-platform via `goblin::Object::parse`:
  - `linux-64` / `linux-aarch64`: ELF64 machine type (`EM_X86_64` / `EM_AARCH64`), `DT_RUNPATH`/`DT_RPATH` leak check (`$ORIGIN` allowed; absolute build roots forbidden), `DT_NEEDED` inventory.
  - `win-64`: PE32+ machine type (`IMAGE_FILE_MACHINE_AMD64`), imported DLL inventory, debug CRT leak refusal (`*d.dll` MSVC debug runtime).
  - `osx-64` / `osx-arm64`: Mach-O 64-bit (`CPU_TYPE_X86_64` / `CPU_TYPE_ARM64`), `LC_RPATH` and `LC_LOAD_DYLIB` leak check (`@rpath`/`@loader_path`/`/usr/lib`/`/System/Library` allowed; runner build roots forbidden).
  - `noarch`: forbid compiled ELF/PE/Mach-O native binaries inside `noarch` packages.
- Verify `package.toml` `audit.expected_binaries` existence.
- Implement `--emit-fragment <dir>` writing deterministic ~1 KB `<subdir>/<filename>.conda.json` (`FragmentJson`) files for `pages build`.

The adapter request/result boundary — capabilities, the plan/build/audit requests, the six
outcomes, and produced artifacts — lives in `crates/universe-core/src/adapter.rs` (`UNI-31`).
This task consumes those records instead of defining a per-command shape of its own.
<!-- SECTION:DESCRIPTION:END -->

### Progress (2026-10-31)

The adapter crate `crates/universe-conda` exists and `open_package` reads an
archive's `info/index.json`, size, and sha256 through the streaming reader, with
fixtures built in-test rather than downloaded. The dependency enrollment and the
policy records it forced (`BSD-3-Clause`, `bzip2-1.0.6`, `MPL-2.0`,
`RUSTSEC-2024-0449`) landed separately in #45 so that this slice could be written
test-first against a compilable API rather than from memory.

`audit_licence` now checks the two claims an archive can be held to before a binary
is inspected: the `about.json` expression parses as SPDX, and every licence file the
record names is present and non-empty. `CondaPackage` gained `read_info_member` and
`info_member_names` so a rule can ask "is this record here?" without a second reader.

Not yet done: md5 and `info/paths.json` verification, the goblin binary audit
(ELF/PE/Mach-O arch, RPATH leak, `DT_NEEDED`), `audit.expected_binaries`, and
`--emit-fragment`.


## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Synthetic minimal ELF64, PE32+, and Mach-O64 `.conda` fixtures can be audited on Linux (cross-platform pure-Rust header/RPATH parsing without `ldd`, `otool`, or `dumpbin`).
- [ ] #2 Auditor catches wrong-arch binaries (e.g., `x86_64` Mach-O inside `osx-arm64`), leaked `/home/runner/work/...` `DT_RUNPATH` or `LC_RPATH`, missing `info/licenses/*`, native binaries in `noarch`, and missing `expected_binaries`.
- [ ] #3 `--emit-fragment` outputs valid `FragmentJson` matching `@universe/schema` with `sha256`, `md5`, `index_json`, `about_json`, `run_exports_json`, and `audit_summary`.
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 `rstest` cross-format binary matrix + `rstest-bdd` (`audit.feature`) + `proptest` path/RPATH sanitization tests pass under `cargo nextest`.
<!-- DOD:END -->
