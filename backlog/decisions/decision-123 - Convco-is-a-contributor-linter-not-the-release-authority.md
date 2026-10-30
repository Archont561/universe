---
id: DECISION-123
title: Convco is a contributor linter, not the release authority
status: accepted
created_date: '2026-10-30'
updated_date: '2026-10-31'
---

## Context

`decision-8` and `doc-1` describe the owner release lane as computing a version with
`convco`, rewriting version literals across `Cargo.toml`, `package.json`, three recipe
files, a skill document, and `CHANGELOG.md`, committing, and tagging.

That description entangles a development tool with the product in three ways:

- **It makes a third-party tool load-bearing policy.** A tool that computes the next
  version and rewrites seven files across four ecosystems has become the release engine of
  a product it has no stake in.
- **It cannot hold for consumer repositories.** A generated workflow cannot invoke a tool
  the consumer may not have, and Universe cannot impose a commit-message convention on
  repositories it does not own.
- **It puts an unreviewable decision behind a command.** The bump is derived from commit
  types, so the version that ships was never chosen by a human. The trigger in this
  repository is already a deliberate tag push, which means the human had already decided.

Three documents disagree with each other about the mechanism, and one has a plain error:
`doc-107` says convco reads `.versionrc`, while the configuration is `.config/convco.yml`.

**That disagreement was then resolved in the wrong direction, and reversed on 2026-10-31.**
This record originally fixed `doc-107` to match the custom path. The custom path was the
error: `.versionrc` is what convco discovers without a flag, and what every editor plugin
written against the conventional-changelog spec looks for. The configuration moved to
`.versionrc` at the repository root, `.config/convco.yml` is gone, and `doc-107` was right
the first time.

## Decision

**Convco is a contributor-side linter and a drafting aid. It is not the release authority.**

Two invariants, both testable:

1. **No generated workflow invokes convco.** A rendered workflow that mentions the tool is
   a defect.
2. **`universe.toml` has no tool-provider key.** No `provider = "convco"`, no version
   computation strategy. Any tool may produce the release inputs; Universe only checks them.

The division of labour:

| Concern | Owner |
| --- | --- |
| Commit-message lint | convco, in the Pixi environment, as a gate of *this* repository |
| Drafting changelog text | convco or a human writer; output is committed and reviewed |
| Declaring the version | a human, in a reviewable promotion commit |
| Verifying version parity across manifests, recipes, and the changelog heading | Universe |
| Refusing to publish without release notes | Universe |
| Composition of the release body | Universe, from the declared changelog section plus the artifact report |

Version authority is therefore a **declared literal that is asserted**, not a value that is
computed. `repo-check` already owns version parity in this repository; the same rule becomes
a lint for consumer repositories instead of a repository-specific script.

## What is amended

| Record | Change |
| --- | --- |
| `decision-8` | Remove the observe-mode version computation and the literal-rewrite list. The lane validates and tags what a promotion commit declared. |
| `doc-1` | Same correction in the release-lane description, plus the changelog named as a curated input. |
| `doc-107` | State convco's scope as commit lint and changelog drafting, explicitly not release automation. Its `.versionrc` path was correct: see the reversal above. |
| `decision-2` | Keep convco in the tool list; scope it "commit message lint". |
| `.config/convco.yml` | Drop the `header:` block, which frames the configuration file as the owner of the changelog. |
| `.versionrc` | The same policy at the path convco discovers, still without `header:` and now also without `increment:` — a bump rule there would be the second version authority this record exists to prevent. |

## Consequences

- Deleting convco from the environment breaks commit linting and changelog drafting. It
  breaks nothing in the product, the release lane, or any generated workflow. That is the
  test of decoupling and it now passes by construction.
- Convco remains a dependency of *this* repository. It is never a prerequisite for a user.
- The generated `universe-release.yml` asserts that a version section exists for the tag and
  that the version matches every declared manifest. It does not care how the text was
  written.
- `CHANGELOG.md` becomes authoritative at publish time: curated by humans, checked by
  Universe, and never silently regenerated.
- If per-package release notes are ever wanted for a multi-ecosystem repository, the
  mechanism is filtering the single curated file by declared scope, not introducing one
  changelog file per package.
