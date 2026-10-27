---
id: decision-11
title: Headless v0.1.0 first, browser portal deferred to v0.2.0
date: '2026-10-27 13:40'
status: accepted
---

## Context

The approved roadmap (doc-4, decision-5, decision-6) placed the SvelteKit/UnoCSS browser
portal in M1, ahead of every CLI capability and of the `v0.1.0` release in M4. Reviewing the
actual repository coupling after M0 showed that the portal and the headless Conda channel are
separable, and that the release machinery is the riskiest unproven part of the plan:

- A CEP-15/CEP-16 channel is fully functional without any HTML: `repodata.json(.zst)`,
  CEP-16 shards, `run_exports.json`, and `channeldata.json` are the complete solver contract
  consumed by `pixi`, `conda`, `mamba`, and `rattler-build`. The browser layer is additive.
- The portal is nevertheless hard-wired into compilation and packaging today:
  `crates/universe-assets/build.rs` panics when `packages/portal/build/index.html` is absent,
  `include_dir!` embeds that directory into every CLI binary, the co-located recipe lists it as
  a source path, and `release.yml` makes the `portal-bundle` job a hard dependency of the
  4-platform build and publish jobs.
- `UNI-12` bundled the channel indexer and the `lol_html` HTML hydration in one task, with a
  dependency on the portal test/embedding task `UNI-6`, even though only the hydration half
  needs the portal.
- The end-to-end release loop (four-platform matrix, OIDC Trusted Publishing, Sigstore
  attestations, the UNI-20 fail-closed guard, isolated publisher, and the external install
  smoke test) has never run in this repository. Proving it on a smaller headless binary first
  is substantially cheaper than proving it together with the full SSG stack.

The project's differentiator remains the browser-accessible universe, so the portal is being
deferred, not cancelled: decision-6 and doc-3 stay approved and unchanged as specifications.

## Decision

Split the first delivery into two releases:

1. **`v0.1.0` — headless universe (revised M1–M4).** The CLI, DAG planner, build/test
   wrappers, auditor, multi-target publisher, fragment-driven CEP-15/CEP-16 channel indexer
   (`universe-kit pages build` emitting solver data only), consumer `init`, watcher, and the
   owner-lane release ship without browser HTML. GitHub Pages serves a Conda channel; no
   catalog explorer is rendered.
2. **`v0.2.0` — browser portal (new milestone M6).** `UNI-5` (SvelteKit 3 + Svelte 5 + UnoCSS
   SSG), `UNI-6` (portal test suites and real asset embedding), new task `UNI-25`
   (`lol_html` hydration and `catalog.json`/`package.json` generation split out of `UNI-12`),
   and new task `UNI-26` (restore the real `portal-bundle` release stage and cut `v0.2.0`)
   deliver the doc-3 portal as a discrete, marketable release.

To keep the existing `include_dir!`, recipe, and workflow wiring intact without the SSG, a
minimal, fully offline placeholder is committed inside the consuming crate at
`crates/universe-assets/placeholder/index.html` — deliberately outside the SvelteKit output
directory, so a portal build can never clobber a tracked file and no `.gitignore` negation is
needed. `build.rs` embeds `packages/portal/build` when its `index.html` exists (or the
`UNIVERSE_KIT_PORTAL_DIST` override when set), and falls back to the placeholder otherwise;
the generated SSG output always wins when present. The recipe and its gitignore-aware root
source stay unchanged, so no source-path collisions are possible. No Cargo feature flags or
conditional compilation are introduced — the embed path stays the same code with one fallback
branch.

Backlog changes: `UNI-5`/`UNI-6` move from M1 to M6; `UNI-12` is rescoped to the headless
indexer with its `UNI-6` dependency removed (the `lol_html` work becomes `UNI-25`); `UNI-15`
stages the headless release against the committed placeholder; M5 dogfooding verifies
solver-served channels only, with portal verification moving to `UNI-26`.

## Rejected / not adopted

- **Cargo feature-gating the portal embed** (e.g. a `portal` feature). It would multiply build
  matrices and recipe variants for a temporary two-release transition; the in-crate
  placeholder achieves the same decoupling with one static file and a single fallback branch.
- **Committing the placeholder under `packages/portal/build/`.** The SSG build empties and
  rewrites that directory, so a tracked file there is overwritten on every `pixi run build`
  (dirty working tree, flaky tests) and, once un-ignored, enters the recipe's gitignore-aware
  root source where it collides with the explicit `packages/portal/build` source entry.
  Placeholder files therefore live in `crates/universe-assets/placeholder/`, which no JS build
  ever writes to.
- **Shipping `v1.0.0` headless and `v2.0.0` with the portal.** SemVer 0.x explicitly exists for
  exactly this kind of scope stabilization; a major version before any public release would
  misstate the maturity of the API and manifest contracts.
- **Dropping the portal from the product.** The browser UI for humans alongside the solver API
  is the core positioning of universe-kit; it is rescheduled to the very next minor release.
- **Removing `universe-assets` from the v0.1.0 binary.** Keeping the crate and embed path
  stable avoids touching the workspace dependency graph and keeps `UNI-26` a content swap
  rather than an architecture change.

## Consequences

- `v0.1.0` is a real, installable, attested Conda product: four-platform `.conda` packages,
  CEP-15/16 static channels, and multi-target publishing, all verifiable through
  `pixi`/`conda`/`mamba` without a browser.
- The critical path to first release shortens by the two M1 portal tasks and the hydration
  half of `UNI-12`; the portal work can still proceed in parallel because it depends only on
  the finished `UNI-30` schema contracts.
- M5 skills.sh rollout and dogfooding are unaffected in substance — both require installable
  packages and solver channels, neither requires HTML; their portal assertions move to M6.
- The placeholder must never grow external URLs or dynamic behavior: it is covered by a
  repository test that asserts its presence, offline content, the `build.rs` fallback order,
  and that generated SSG output stays git-ignored.
- doc-4, doc-3, doc-1, the milestone files, and the affected task files are updated to record
  the re-vision; decision-5, decision-6, and doc-3 remain approved specifications, deferred in
  delivery order only.
