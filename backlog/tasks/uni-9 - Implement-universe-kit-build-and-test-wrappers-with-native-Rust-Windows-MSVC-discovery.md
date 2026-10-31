---
id: UNI-9
title: Orchestrate configured native build and test commands without owning builders
status: To Do
assignee: []
created_date: '2026-10-26 20:15'
updated_date: '2026-10-27'
labels:
  - architecture
  - testing
  - security
milestone: m-2
dependencies:
  - UNI-7
  - UNI-8
  - UNI-31
  - UNI-41
documentation:
  - backlog/docs/architecture/doc-2 - Universe-Kit-CLI-and-manifest-specification.md
  - backlog/docs/architecture/doc-111 - V1-package-layout-and-offline-readiness.md
priority: high
type: feature
ordinal: 9000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Implement Kit's build/test orchestration as a thin, auditable command runner. Package
builders remain external native tools: examples include rattler-build for Conda recipes,
Cargo tooling for crates, Python wheel builders, and npm packing. Kit reads the declared
command, working directory, environment allowlist, output paths, and target from the
versioned project manifest; it runs the command in the configured isolated job and collects
only declared outputs. Kit does not implement a package compiler, bundle a builder as a
runtime dependency, or infer build steps from ecosystem names.

Record command/tool identity and exit status. Validate that each collected artifact matches
the expected package identity, version, target, size, and digest before passing it to audit,
indexing, or publication. Keep credentialed publication outside the untrusted build process.

The adapter request/result boundary — capabilities, the plan/build/audit requests, the six
outcomes, and produced artifacts — lives in `crates/universe-core/src/adapter.rs` (`UNI-31`).
This task consumes those records instead of defining a per-command shape of its own.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Schema validation rejects undeclared commands, invalid paths, output globs escaping the workspace, and forbidden environment/credential forwarding.
- [x] #2 Command-recording fakes prove exact argv/cwd/allowlisted environment behavior, output collection, non-zero exit handling, timeout behavior, and deterministic receipts without network access. — 2026-10-31: `universe-kit build` runs each package's declared argv in the project root with a cleared environment carrying the package's `env` plus a named ambient allowlist (PATH, HOME, temp, locale, TLS trust, proxies — every credential excluded by name), collects the declared output globs, and refuses a non-zero exit (`build.command.failed`, carrying the tail of stderr), an unrunnable program (`build.command.unrunnable`), a build that produced nothing (`build.output.missing`), and a command that outlives `--timeout` (`timed out after Ns`). `--receipts <dir>` writes one receipt per package naming command, cwd, duration, and each output's path, size, and digest. Integration tests in `crates/universe-kit/tests/verbs.rs` cover all of it, including a `sleep 30` bounded at one second. Correction to the earlier note on this criterion: the exclusion set is asserted by unit test rather than by mutating the process environment, because `set_var` is `unsafe` in Rust 2024 and a test that calls it races every other test in the binary.
- [x] #3 A fixture command can emit a Conda package, wheel, crate, or npm tarball fixture and Kit verifies/collects its declared artifact without invoking a builder implementation. — 2026-10-31: a fixture `.conda` is placed by the test harness and `build` collects it, then verifies it by opening the archive and comparing `info/index.json` with the package the manifest declares (`build.output.identity`). Kit never invokes a builder: the Conda package itself is produced by `rattler-build` through the manifest's declared command, and Kit only runs that command.
- [ ] #4 Release credentials are unavailable to build/test child processes; publication consumes verified artifact receipts in a separate job/lane.
- [ ] #5 Tests prove the command runner does not silently add network fallback or mutate the user's persistent native-client configuration.
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 Rust unit/BDD/property tests pass under the offline gate using command-recording fixtures.
- [ ] #2 No `rattler-build` runtime dependency or platform-specific MSVC discovery is added to Kit solely to build product packages.
<!-- DOD:END -->
