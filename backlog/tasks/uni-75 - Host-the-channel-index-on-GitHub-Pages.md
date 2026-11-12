---
id: UNI-75
title: Host the channel index on GitHub Pages
status: In Progress
assignee: []
created_date: '2026-11-13'
updated_date: '2026-11-13'
labels:
  - conda
  - publishing
  - ci
  - docs
milestone: m-12
dependencies:
  - UNI-11
  - UNI-12
documentation:
  - backlog/decisions/decision-132 - v0.1.0-is-a-pre-release-packaging-milestone,-not-a-v1-cut.md
  - backlog/decisions/decision-128 - Rendered-workflows-expose-one-verdict-job-per-file.md
  - backlog/decisions/decision-140 - Build-the-Universe-docs-site-from-pinned-Fumadocs-UI-at-build-time.md
priority: high
type: feature
ordinal: 75000
---

## Description
<!-- SECTION:DESCRIPTION.BEGIN -->
A Conda channel that only exists as a Git branch and a registry is hard to point a solver
at. The owner's ask for `v0.1.0` was a **hosted index** next to the release: one address a
consumer configures, no browser UI, no separately maintained site.

The channel a consumer reads is therefore served twice from the same bytes. prefix.dev and
the `channel` branch already hold them; GitHub Pages is the third read of the same tree, and
the one with a stable URL and no client.

It is not a byte destination. Nothing uploads to a Pages site — the site is generated at
release time from the release's own build fragments by `universe index`, uploaded as a Pages
artifact, and deployed by a job that runs one action. No generated file enters the
repository, so the site cannot describe a channel that a release did not produce.

The declaration is one manifest flag. `pages = true` on a store tells the renderer to emit
the two jobs, and the store must also be `static`, `writable = false`, `needs_index = true`,
and name the `url` the index advertises as its own base. A hosted site hand-written into
`.github/workflows/` would be the thing this repository exists to remove.

`decision-140` remains the answer for a browsable portal built from pinned catalog snapshots
with a Fumadocs UI. This is the simpler thing the owner asked for in `v0.1.0`: a static
channel index at an address, with no JavaScript and no build step beyond the one the release
already ran.
<!-- SECTION:DESCRIPTION.END -->

## Acceptance Criteria
<!-- AC.BEGIN -->
- [x] #1 A `static` store can declare `pages = true`, and the renderer emits the generate-and-deploy jobs for it. — 2026-11-13: `crates/universe-workflow/src/lib.rs` renders `<store>` (checkout, Pixi, download every `universe-*` artifact into `output`, `universe index --output output/pages --base-url <store.url> --json`, `actions/upload-pages-artifact` on `output/pages`) and `<store>-deploy`, and `a_pages_store_renders_a_deployment_and_no_publish_job` asserts the store produces no publish job.
- [x] #2 The index is generated from the build the release depends on, not from a tree in the repository. — 2026-11-13: `the_pages_job_generates_the_index_from_the_build_it_depends_on` asserts the job needs `build`, downloads the artifacts with `pattern: universe-*` and `merge-multiple: true`, and writes to `output/pages`, which is deliberately not the fragments directory.
- [x] #3 The deployment is a job of its own, with the permissions and environment a Pages deployment needs and nothing else. — 2026-11-13: `the_deployment_is_its_own_job_with_the_privileges_it_needs` asserts one step (`actions/deploy-pages`), `pages: write` with `id-token: write`, `environment: github-pages`, no checkout and no Pixi; `the_verdict_waits_for_the_deployment` asserts `verdict` needs it.
- [x] #4 A manifest that declares a Pages store without the four facts it needs is refused, by name. — 2026-11-13: `crates/universe-core/src/manifest.rs` reports `manifest.store.pages.kind`, `.writable`, `.index` and `.url`, and four tests in the same file assert each code on the declaration that omits it.
- [x] #5 A project with no Pages store renders no Pages job. — 2026-11-13: `a_project_without_a_pages_store_renders_no_pages_job` asserts the rendered publish file holds no deployment job.
- [x] #6 This repository declares the store, and the address a consumer is told is the address the workflow deploys. — 2026-11-13: `universe.toml` declares `pages` at `https://archont561.github.io/universe`; `pixi run ci-render` regenerated `release.yml` and `ci-check` agrees; `scripts/tests/test_release_scope.py::test_the_hosted_index_is_declared_and_deployed` reads the manifest, the rendered workflow and the README together.
<!-- AC.END -->

## Definition of Done
<!-- DOD.BEGIN -->
- [ ] #1 A tag run deploys the site, and the deployed address serves a `repodata.json` that resolves the published packages by digest. — Not yet: GitHub Pages is not enabled on the repository (`gh api repos/Archont561/universe/pages` answers 404), so no deployment has run and the address serves nothing.
- [x] #2 `pixi run gates` passes and no test requires network egress. — 2026-11-13: the renderer and manifest tests are fixtures; the release-scope test reads files in the repository.
<!-- DOD.END -->

## Notes

2026-11-13 — why a flag and not a store kind.

The obvious design is a Pages destination like any other, and it is wrong three times over.
`crates/universe-kit/src/artifacts.rs` maps `StoreKind::Static` to no channel kind, so a
publish job for a static store downloads the artifact and uploads nothing: writable, it
renders a job that does nothing and reports success. Adding a `pages` kind would say the
same thing with a new word. And the thing a Pages deployment needs — an index rendered from
the build, a deployment with an OIDC token, an environment name — is not a property of a
destination, it is a second view of one.

So the flag says "this store is a site", and the store is the non-writable static store that
already describes `needs_index` and `url`. The renderer emits two jobs after the destination
jobs and before `verdict`: one that generates and uploads, one that deploys. Two jobs rather
than one because the deployment needs `pages: write` and `id-token: write` and no checkout —
melting them together would hand a build step a deployment token.

`output/pages` instead of `output` is not cosmetic: the downloaded artifacts merge into
`output`, so an index written to `output` would be uploaded alongside the fragments it was
generated from, and any later fragment change would silently alter the site.

The address is declared, not derived. `https://<owner>.github.io/<repo>` is right for a
project page and wrong for a user page, a custom domain, or a repository renamed after the
fact, and the value is also the base URL the generated `repodata.json` advertises — so
guessing it would produce an index that points at the wrong place.

`configure-pages` is the first step of the generate job on purpose: it fails loudly when the
repository has no Pages site configured, which is the state of this repository today. The
deployment is therefore rendered and drift-checked before it can run, and the missing
repository setting shows up as a failed step rather than as a site that serves nothing.
