---
id: UNI-55
title: Package and ship the Airlock bootstrap artifact
status: In Progress
assignee: []
created_date: '2026-10-31'
updated_date: '2026-11-12'
labels:
  - airlock
  - bootstrap
  - release
milestone: m-5
dependencies:
  - UNI-33
  - UNI-38
documentation:
  - backlog/decisions/decision-115 - Bootstrap-readiness-is-distinct-from-sealed-offline-readiness.md
  - backlog/decisions/decision-122 - One-product-two-editions.md
  - backlog/decisions/decision-127 - Manifest-facets-packages-stores-and-airlock-scopes.md
priority: high
type: chore
ordinal: 55000
---

## Description
<!-- SECTION:DESCRIPTION.BEGIN -->
Close a shipping gap: `crates/universe-airlock-bootstrap` builds a binary, but it has no
recipe, no run dependency anywhere, and therefore never reaches a channel or a release
asset. The workspace describes it as a distinct artifact that other records depend on,
so the absence is an inconsistency rather than a deliberate omission.

Package and ship it as its own artifact. It stays deliberately small: it verifies a
local payload and activates a restore, and it must not acquire producer, registry, or
network capability. It is a phase of the Airlock edition rather than a third product,
so it shares the edition's version and release train and carries no independent
version line.

Also decide and record whether it is a separately installable package or a bundled
asset of the consumer edition, and make the answer visible where consumers look — the
release assets and the umbrella package's runtime requirements.
<!-- SECTION:DESCRIPTION.END -->

## Acceptance Criteria
<!-- AC.BEGIN -->
- [x] #1 A recipe builds the bootstrap executable for every supported target and installs only that executable; its runtime requirements add no producer, registry, or network tooling. — 2026-10-31: `crates/universe-airlock-bootstrap/recipe/` plus `pixi run package-airlock-bootstrap`; `repo_check` now enforces its version parity.
- [x] #2 The artifact is smoke-tested in a clean environment and its version contract matches the workspace version. — 2026-11-12: two independent smokes. The recipe declares `tests: - script: universe-airlock-bootstrap --version` and `package-airlock-bootstrap` passes no `--test=skip`, so rattler-build runs that test in a fresh test prefix during the build and fails the build if it does not pass — run `43252390113` built the package that way. Separately, a Pixi project asking only for `universe-airlock-bootstrap = "0.1.0"` from a clone of `refs/heads/channel` ran `pixi install --offline`, installed, and printed `universe-airlock-bootstrap 0.1.0` from `.pixi/envs/default/bin`; that prefix holds the one package, whose only declared dependency is `__glibc >=2.17,<3.0.a0`. The version contract is enforced rather than read: `repo_check` compares this recipe's `context.version` and package name with the workspace's, and the binary reports `0.1.0`.
- [x] #3 The release lane publishes it alongside the consumer artifact with checksums, and the version policy in `decision-126` applies to it as part of the same train. — 2026-11-12: `release.yml` run `43252390113` (head `412412f8`, `workflow_dispatch` on `main`) is green on attempt 1, all five jobs. `publish / build` audited four packages and reported the bootstrapper at `sha256:1fb284e59d46fd54a97cae2c2e61c72afbb64e24011d06e9436acb5dc1a88861`, 298467 bytes; `publish / releases` uploaded it as a fourth asset of the `packages-linux-64` release, which reports that digest; `publish / prefix-dev` forced it in, and a cache-busted repodata read (publication `93807`) resolves it by that digest; and `publish / branch` pushed one snapshot at `5424f2a40a5efb5f9067deb90877848a222c6084` whose `linux-64/repodata.json` lists all four packages with the run's digests. It carries the same `0.1.0` version and the same tag train as the other three, so `decision-126`'s promotion rules apply to it unchanged. https://github.com/Archont561/universe/actions/runs/43252390113
- [x] #4 Whatever the packaging decision is, the consumer edition's documentation and the release asset list agree with it. — 2026-11-12: the decision is **separately installable**, recorded in `decision-132`'s fourth-package amendment. `README.md`, `docs/releases/0.1.0.md` and `universe.toml` all call it separately installable on the same version and train, with no runtime dependency; the facade does not depend on it. `scripts/tests/test_release_scope.py::test_the_bootstrap_is_a_separately_installable_package` and `::test_the_consumer_docs_and_the_asset_list_agree_on_the_bootstrapper` assert the agreement, and both were red before the manifest entry and the prose existed.
- [ ] #5 The public verification key is compiled into the bootstrap artifact and its rotation procedure — a bootstrap signed by both the old and the new key — is written down and exercised on a fixture (`decision-131`).
<!-- AC.END -->

## Definition of Done
<!-- DOD.BEGIN -->
- [ ] #1 No artifact referenced by another Backlog record is missing a shipping path.
- [ ] #2 `pixi run repo-check` and `pixi run gates` pass, with recipe/workspace version parity enforced.
<!-- DOD.END -->

## Notes
<!-- SECTION:NOTES:BEGIN -->
- **2026-11-12 — the shipping gap is declared closed, and how.** `UNI-59` could not gain a
  restore entrypoint without this: the entrypoint clones a pinned commit and executes
  `universe-airlock-bootstrap` from that checkout, and no ref carried the binary — `channel` is a
  Conda tree, and no branch holds a bootstrapper. The recipe and `pixi run
  package-airlock-bootstrap` already existed, so the missing piece was a manifest declaration and
  the workflow it renders.
  `universe.toml` now declares `universe-airlock-bootstrap` with `kind = "conda"`, `linux-64`
  only, `outputs = ["output/universe-airlock-bootstrap/linux-64/*.conda"]`, a smoke command, the
  same `publish` stores as its siblings, and `release = true`; `ci render` put one
  `publish-artifacts` step for it under `prefix-dev` and `releases` and added it to the single
  branch snapshot. It carries no `depends_on`, and the facade does not name it.
  **Why no `--test=skip`:** the facade's task has one because its run dependencies are not in the
  channel until the same release publishes them. The bootstrapper declares no run requirements at
  all, so rattler-build's post-build test has a solvable environment and is worth keeping — it is
  the clean-environment smoke that criterion 2 asks for.
- **What is still open here.** Criterion 2 needs that post-build test to run and pass and its
  output recorded; criterion 3 needs the publish job's read-back at all three destinations, with
  checksums; criterion 5 needs a release key, which is `decision-131`'s remaining work, not a
  packaging change. The umbrella edition's own documentation is the facade's run requirements:
  they name `universe-kit` and `universe-airlock` and deliberately not the bootstrapper, so
  installing the facade never implies a binary that runs before an environment exists.
- **2026-11-12 — published, read back at all three destinations, and smoke-tested offline.**
  `release.yml` run `43252390113` is green on attempt 1 with all five jobs. The build audited
  four artifacts; the bootstrapper is
  `sha256:1fb284e59d46fd54a97cae2c2e61c72afbb64e24011d06e9436acb5dc1a88861` at 298467 bytes,
  alongside `universe-airlock` `fa33b2a0cfa4b8f9ad548b64fc0b1d584b70f06ba5f3f9b5988851fb61b3ed58`
  (298824), `universe-kit` `aece6a4e4b1aa1dfb5b0eca5d26272c165aea573a5218c00c4a55a2c7ac38cf3`
  (3493512) and the facade `universe`
  `a767f730a96e7bb07ee835e354a70dc1f8e5c60c79d8940bd8c636224afdb4dd` (195251). The four GitHub
  Release assets of `packages-linux-64`, the one-snapshot `channel` branch at
  `5424f2a40a5efb5f9067deb90877848a222c6084` and the cache-busted prefix.dev read at publication
  `93807` all resolve those four digests. This run's reindex landed in a few minutes, unlike the
  earlier forced republish that left the index a publication behind for more than 28 minutes —
  the lesson (`UNI-59`) is to publish once and read after the reindex, and this run followed it.
  https://github.com/Archont561/universe/actions/runs/43252390113
- **The clean-environment smoke, as run.** From a fresh `git clone --depth 1 --branch channel`,
  one Pixi project asked only for the bootstrapper, the same shape as the offline solves recorded
  in `UNI-59`:

  ```
  [workspace]
  name = "solve-bootstrap"
  channels = ["file:///tmp/uni55/channel"]
  platforms = ["linux-64"]

  [dependencies]
  universe-airlock-bootstrap = "0.1.0"
  ```

  `pixi install --offline` printed `The default environment has been installed.` and exited 0;
  `./.pixi/envs/default/bin/universe-airlock-bootstrap --version` printed
  `universe-airlock-bootstrap 0.1.0`; and the one `conda-meta` record in that prefix declares
  only `__glibc >=2.17,<3.0.a0`, which is the empty-requirements claim checked from the outside.
  The recipe's own `tests:` block is the same command run by rattler-build in a throwaway test
  prefix at packaging time, and `package-airlock-bootstrap` passes no `--test=skip`, so a failure
  there fails the build rather than the release.
- **The republished ref is a separate artifact, not a second `channel`.** 2026-11-12: the binary
  a restore entrypoint runs is now also published as a Git ref — `bootstrap` at
  `991324f5dee0cb02d18ea66e35e1e12206a35b14` — by `universe airlock publish-bootstrap`. It holds
  exactly two entries: `universe-airlock-bootstrap` at mode `100755` and a `bootstrap.json`
  descriptor naming its digest, size and version. It is **not** a package channel and not a slice
  of the `channel` branch: there is no `repodata.json`, no lockfile and no catalogue, so nothing
  can solve against it and nothing should try. The package channel stays the one place a solver
  installs from; this ref is the one place the entrypoint's `exec` finds a file. The binary bytes
  are the same released artifact (`sha256:929bd8f4…aed2`, extracted from package
  `sha256:1fb284e5…`), so the ref introduces no second build to keep in step — only a second
  transport for the same bytes, with the executable mode that a package archive's `bin/` already
  has and a branch channel's tree did not. Criterion 5 needs the public verification key compiled into the
  artifact and a rotation procedure exercised on a fixture (`decision-131`); that is key material
  and signing work, not packaging, so the task stays `In Progress`. `UNI-59` criterion 5 and
  DoD #3 stay open on the clean-container transcript. No tag is cut by this change.
**Status note — the ref now explains itself (2026-11-13).** `universe airlock
publish-bootstrap` publishes a generated `README.md` beside the executable and
`bootstrap.json`, so the ref a person lands on states what it is, what the binary beside it
hashes to, and that it is not a Conda channel — the first wrong assumption a reader arriving
from a manifest pin makes. It is generated from the bytes in the same commit, so republishing a
different binary rewrites the digest on the page; and because nothing in it varies with time, an
unchanged binary still yields an unchanged ref and the pin in every consumer's manifest stays
put. `crates/universe-kit/tests/bootstrap_ref.rs` asserts the three-entry tree and the
no-stale-digest property. This does not touch criterion 5 or DoD #3, which stay open on key
material and the clean-container transcript.
<!-- SECTION:NOTES.END -->
