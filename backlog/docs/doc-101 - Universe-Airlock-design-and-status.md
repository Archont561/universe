---
id: doc-101
title: Universe Airlock design and status
type: readme
created_date: '2026-10-25T23:49:00Z'
updated_date: '2026-10-27T09:20:00Z'
---

# Universe Airlock design and status

Consolidated on 26 October and updated on **27 October 2026** from the design discussion and the current repository. This is the primary design index; specifications and decisions live inside Backlog, not in a separate documentation or design project.

## Identity and implementation boundary

| Item | Selected value |
|---|---|
| Product, executable, application crate | Universe Airlock / `universe-airlock` / `crates/universe-airlock` |
| GitHub repository | `Archont561/universe` |
| prefix.dev channel | Planned; not yet configured or published |
| Conda build recipe | `crates/universe-airlock/recipe/recipe.yaml` (recipe scaffold; build not yet validated) |
| Application publication workflow and environment | Not implemented |
| Current application version | `0.1.0` |

**Implemented application behavior is deliberately limited to VERSION and version reporting.** The CLI/core Cargo workspace and external Rust tests exist. Developer tooling, hooks, CI, coverage configuration, and release automation are scaffolding, not an implementation of the cache product.

There is no implemented Universe Airlock `init`/`exec`, cache acquisition, payload publication, bootstrap restore, offline replay, self-update, questionnaire, or TUI. The separately installed pixi-sandbox is interim developer tooling, not Universe Airlock product functionality. Proposed command names and on-disk records below describe responsibilities, not callable interfaces.

## Reading order

1. [Product architecture and bundle model](doc-102%20-%20Product-architecture-and-bundle-model.md) — canonical artifacts, immutable storage, profiles, and format boundaries.
2. [Init, bootstrap, and restore lifecycle](doc-103%20-%20Init-bootstrap-and-restore-lifecycle.md) — the corrected initializer/target workflow.
3. [Ecosystem adapters and external assets](doc-104%20-%20Ecosystem-adapters-and-external-assets.md) — conda/Pixi, wheels, npm, Cargo, generic downloads, and Playwright.
4. [Git payload transport and curated requests](doc-105%20-%20Git-payload-transport-and-curated-requests.md) — data-driven additions, immutable payload references, sharding, and publication order.
5. [Offline verification and acceptance gates](doc-106%20-%20Offline-verification-and-acceptance-gates.md) — what must pass before any offline compatibility claim.
6. [Rust workspace and development tooling](doc-107%20-%20Rust-workspace-and-development-tooling.md) — source layout, Pixi/npm, Python automation, and Git hooks.
7. [Releases and trusted publishing](doc-108%20-%20Releases-and-trusted-publishing.md) — application distribution and account-side prerequisites.

8. [Git-backed catalogs and on-demand execution](doc-109%20-%20Git-backed-catalogs-and-on-demand-command-execution.md) — supported-command routing, pinned catalogs, selective Git fetch, and readiness modes.
9. [Interim development transport with pixi-sandbox](doc-110%20-%20Interim-development-transport-with-pixi-sandbox.md) — the pinned developer-tool integration and its limits.

## Decision register

- [Identity and version-only scope](../decisions/decision-101%20-%20Project-identity-and-version-only-scope.md)
- [Original conda artifacts and effective metadata](../decisions/decision-102%20-%20Original-conda-artifacts-and-effective-metadata.md)
- [Finite snapshots, locked replay, and curated additions](../decisions/decision-103%20-%20Finite-snapshots-locked-replay-and-curated-additions.md)
- [Init-generated entrypoint and Pixi-managed restore](../decisions/decision-104%20-%20Init-generated-entrypoint-and-Pixi-managed-restore.md)
- [Data-driven requests and immutable payload publication](../decisions/decision-105%20-%20Data-driven-requests-and-immutable-payload-publication.md)
- [Workspace-managed tooling and external Rust tests](../decisions/decision-106%20-%20Workspace-managed-tooling-and-external-Rust-tests.md)
- [Commit checks and affected-snapshot hooks](../decisions/decision-107%20-%20Commit-checks-and-affected-snapshot-Git-hooks.md)
- [Separate CI, release, and bundle workflows](../decisions/decision-108%20-%20Separate-CI-release-and-bundle-workflows.md)
- [Offline enforcement and evidence levels](../decisions/decision-109%20-%20Offline-enforcement-and-evidence-levels.md)
- [Generic assets and Playwright](../decisions/decision-110%20-%20Generic-assets-and-Playwright-are-separate-artifact-types.md)
- [Optional UX, testing, and update libraries — proposed](../decisions/decision-111%20-%20Optional-UX-testing-and-update-libraries.md)
- [Repository-only layout and Backlog documentation](../decisions/decision-112%20-%20Repository-only-layout-with-Backlog-design-records.md)

- [Native client execution through local package views](../decisions/decision-113%20-%20Native-client-execution-through-local-package-views.md)
- [Pinned Git catalogs and selective artifact retrieval](../decisions/decision-114%20-%20Pinned-Git-catalogs-and-selective-artifact-retrieval.md)
- [Bootstrap readiness is distinct from sealed offline readiness](../decisions/decision-115%20-%20Bootstrap-readiness-is-distinct-from-sealed-offline-readiness.md)
- [Use pixi sandbox for interim developer transport](../decisions/decision-116%20-%20Use-pixi-sandbox-for-interim-developer-transport.md)

**Accepted** records capture an agreed constraint/direction; they do not mean the feature has been implemented. **Proposed** records retain recommendations that have not been selected. Numerical defaults, wire formats, and future library choices remain proposals unless explicitly identified otherwise.

## Product direction

Build a Rust application around thin bootstrap, pinned Git-backed catalogs, local native package views and selective verified acquisition for a finite, verifiable dependency universe: locked-project replay plus curated additions, local HTTP and filesystem-only consumption, and actual denied-egress validation. Linux x86_64 is the first product-validation target. Configuring native Windows/macOS release jobs does not certify the offline product on those platforms.

The next product milestone is to prove the hard adapter and isolation boundaries using small fixtures, before implementing the full permanent cache/bundle architecture. No implementation tasks have been invented or seeded by this documentation consolidation.

## Unresolved choices

The bundle envelope/wire version, concrete signature serialization, target-side state directory, branch/ref spelling, shard budgets, source-build support, and optional UI/update crates are not frozen. The earlier illustrative glibc 2.28 profile is not a product guarantee; every deployment needs an explicit target profile. The current application-package recipe's glibc 2.39 requirement is a separate distribution choice.
