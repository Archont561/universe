---
id: DRAFT-8
title: Vendor the BDD runner before claiming scenario coverage
status: promoted
created_date: '2026-10-31'
updated_date: '2026-10-31'
---

## Problem

`AGENTS.md` now requires a behaviour a user or a consuming repository can observe to be
established as a scenario first, through `rstest-bdd`. The workspace declares
`rstest-bdd` and `rstest-bdd-macros`, but the developer transport vendor tree
(`.pixi-sandbox/vendor`) does not contain them, and this environment cannot reach
crates.io. The consequence is concrete: the `ci render` / `ci check` slice shipped with
integration tests at the CLI seam and **no** `.feature` file, because writing one that no
runner can execute would be scaffolding, not a specification.

That is a recorded gap rather than a silent one: the CLI behaviour is proven by tests, the
scenario layer for it does not exist yet, and the next session should not assume otherwise.

## Proposed change

Add `rstest`, `rstest-bdd`, and `rstest-bdd-macros` to the vendored dependency set the
sandbox publishes, so `pixi run test` can execute `.feature` files offline. Then write the
first scenario for the renderer — "a manifest declaring two stores renders one publish job
per store" — and let it drive the next behaviour change instead of following it.

## Promoted (2026-10-31)

The dependency declaration and the renderer's first five scenarios are in
`crates/universe-workflow/tests/render_bdd.rs` with
`tests/features/render.feature`. The lock cannot be regenerated from this
environment, so the `relock` bot produces `Cargo.lock` and the sandbox
publication vendors `rstest*` on the next run. Tracked as `UNI-57` AC #8.

## Why it was a draft

It is transport work (a sandbox publication), not product work, and it should not be
scheduled ahead of `UNI-57`'s remaining acceptance criteria.
