---
id: DECISION-129
title: Universe distributes itself through conda and release assets, not through language registries
status: proposed
created_date: '2026-10-31'
updated_date: '2026-10-31'
---

## Context

The question arrived as four: ship prebuilt binaries reachable through `cargo binstall`, publish
`@universe/universe` on npm, publish `universe` on PyPI, publish `universe` on conda. Those look
like one question about reach. They are two questions with opposite answers: *how does a user
install the tool* and *how does the tool prove it can publish into an ecosystem*.

The registration facts, checked on 2026-10-31:

| Candidate | crates.io | PyPI | npm |
| --- | --- | --- | --- |
| `universe` | taken — `FreddyWordingham`, `0.0.0`, 2024 | taken — OpenAI, `0.21.3` | taken — `tannerlinsley`, `0.8.1`, 2022 |
| `airlock` | — | taken — Grow SDK, `0.0.13` | — |
| `universe-core`, `universe-kit`, `universe-airlock`, `universe-cli` | free | free | free |
| `@universe/universe` | — | — | name free; scope ownership unverifiable from the registry |

So `decision-122` is right that "universe" is generic and taken, and wrong in the same paragraph
where it calls Airlock the distinctive mark that solves this: on PyPI, Airlock is taken as well.
The prefixed component names are free everywhere, which is the only naming lever that exists.

The shape of the tool decides the rest. `universe` is a facade that dispatches to `universe-kit`
and `universe-airlock` by name on `PATH`; the payload is a set of sibling executables, not one
binary. And a Rust binary on PyPI or npm is a wheel or tarball carrying a launcher that downloads
the real executable at install time or first run — an unpinned, mutable fetch at the front door
of a product whose reason to exist is pinned digests and sealed offline restore.

## Decision

**Universe's own artifacts are distributed as conda packages and signed release assets. No
language registry is a distribution channel for the tool.**

1. The canonical install is the conda umbrella package `universe` from the project's static
   channel, which stays `https://prefix.dev/archont561/archont561`. The channel name is
   permanent for practical purposes: it appears in every consumer `pixi.toml` pin, so changing it
   breaks installed projects rather than just redirecting new ones.
2. Each release publishes per-target archives (`linux-64`, `osx-arm64`, `win-64`, and the fourth
   the recipe matrix settles) to GitHub Releases alongside the conda artifacts. These archives
   are the same bytes the `releases` store in `decision-127` indexes; they are not a second
   packaging path.
3. `cargo binstall` compatibility is **git-source only**: `[package.metadata.binstall]` on the
   crates that ship binaries, `cargo binstall --git https://github.com/Archont561/universe
   universe`, with the archive fetched from the release described in that metadata. No crate is
   published to crates.io while `publish = false` is the workspace default, and flipping that
   requires its own record.
4. A binstall install is a **component set**, not a single binary, because the facade cannot work
   alone. The set is named explicitly in the metadata and documented as a unit; all members are
   built from one commit and carry one version. Since there is no solver to catch a mixed
   install, the facade reports the versions it resolves and fails when they disagree rather than
   forwarding to a sibling of another generation.
5. Cargo, PyPI, and npm remain publishing targets for **users'** artifacts (`UNI-42`, `UNI-43`,
   `UNI-44`). Universe's presence in those ecosystems is as a publisher, never as a package, so
   the two namespaces never collide and no fallback mark has to be invented for a facade that
   should not be there.

Refused, with reasons rather than preferences: `universe` on PyPI (OpenAI), `universe` on npm
(`tannerlinsley`), `universe` on crates.io (`FreddyWordingham`), and any launcher-and-download
package under a free alias. A free alias does not fix the second problem, which is the shape.

## What argues against this

- **Reach.** `pipx install` and `npm i -g` are the lowest-friction installs for a large share of
  users. Refusing both costs adoption, and the cost lands on exactly the users who do not have
  conda and will not clone a repository.
- **Two requirements, one artifact.** A user who wants to *publish into npm* needs an npm token
  and, on this design, no npm presence of ours. That is defensible but less obvious than "the
  tool that publishes to npm is on npm".
- **crates.io is the natural home for a Rust tool**, and the `0.0.0` placeholder squatting
  `universe` is the kind of thing that sometimes clears. If it does, the argument narrows to
  "publish the components, keep the facade out".
- **Binstall without crates.io is unusual**, and `--git` requires a reachable GitHub and a
  toolchain-less binstall install still needs `cargo-binstall` itself. That is a real dependency
  the conda path does not have.

The mitigation, if adoption evidence ever demands it, is a signed installer or a setup action
that verifies a digest against the release manifest — a delivery mechanism, never a registry
package and never a second version authority.

## Conditions that would change this decision

- The `universe` crate name becomes available and the facade ships as one self-contained binary:
  plain `cargo install universe` becomes viable and rule 3 relaxes.
- Support evidence that users cannot install without conda: revisit with an installer, not a
  registry package.
- `decision-122`'s two editions merge into a single executable: the component-set problem in
  rule 4 disappears and binstall becomes a single-artifact install.

## Consequences

If accepted, and not before:

- `UNI-40` gains an acceptance criterion: the facade detects and reports component version skew
  across the set it resolves, instead of forwarding to a sibling built from another commit.
- `UNI-34` gains the release-asset requirement: per-target archives plus binstall metadata are
  release outputs, produced by the same job that publishes conda artifacts.
- `UNI-43` and `UNI-44` gain one sentence each: publishing *into* PyPI and npm does not make
  Universe a package on either.
- `doc-108` gains the install matrix — conda canonical, release assets plus git-source binstall
  for component supply, no language registry — and `doc-2`'s consumer pin keeps the
  `archont561` channel by name.

## Status boundary

**Proposed.** It records a recommendation and the registration facts behind it; no task gains a
criterion and no `publish` flag flips until this is accepted. See
[doc-101](../docs/doc-101 - Universe-Airlock-design-and-status.md) for the status
index and `decision-122` for the naming hazard this record answers.
