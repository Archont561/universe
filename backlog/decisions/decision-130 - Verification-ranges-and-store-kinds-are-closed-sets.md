---
id: DECISION-130
title: Verification ranges and store kinds are closed sets
status: accepted
created_date: '2026-10-31'
updated_date: '2026-10-31'
---

## Context

`decision-127` named two `[workflow]` booleans — `verify_artifact` and `verify_published` — in
prose ("switch the two verification ranges") and never in the field table, so neither had a
definition or a default. The same record gives a store a `verifier` of `file` or `registry`,
while the jobs that do the verifying are `verify-artifact` and `verify-index` (`decision-128`),
and nothing says which job reads which evidence source.

Separately, `ghcr` is a declared store `kind` with no adapter task, no first-party publisher,
and no mention anywhere outside the two tables that list it. A manifest may therefore declare
`kind = "ghcr"`, pass lint, and render a job for a destination nothing can write to — the
failure mode the compatibility lint exists to prevent, arriving through the lint itself.

## Decision

**Both vocabularies are closed, and both are defined by what they read rather than by what they
are called.**

### 1. `[workflow]` has exactly two verification booleans, both defaulting to `true`

| Field | Default | Meaning | Job |
| --- | --- | --- | --- |
| `verify_artifact` | `true` | install each built artifact into a clean environment and run its declared `smoke` commands | `verify-artifact` |
| `verify_published` | `true` | re-read each writable store declaring `read_auth: none` and assert the published copy sits at the built digest | `verify-index` |

Either may be set to `false`, because a project with no runnable artifact — a pure asset
catalogue — has nothing to smoke. The renderer omits the job and prints the reduced assurance in
the same output that carries the store→package mapping, so the loss is visible at render time
rather than discovered during an incident.

### 2. `verifier` keeps its two values and gains a mapping

| `verifier` | Evidence read | Read by |
| --- | --- | --- |
| `file` | the index we generate, compared to what we built | the publishing job immediately after upload, and `verify-index` independently when `read_auth: none` |
| `registry` | the store's own record of what it accepted, e.g. `npm view` | the publishing job immediately after upload; no separate job |

`registry` gets no second reader on purpose: a verification job would have to present the
publisher's credential to read the store back, which doubles the jobs without doubling the
independence (`decision-128`). The value names the evidence source, not the job that reads it —
`file` does not mean "the `file` job", and that ambiguity is what this section removes.

### 3. A store kind ships only when something services it

`ghcr` is removed from the `kind` list until an OCI adapter exists. A kind is admitted when one
of four things services it: a first-party adapter (`cargo`, `pypi`, `npm`), the static indexer
(`static`, `prefix-dev`, `branch`), the releases publisher (`releases`), or the generic
`command` runner (`command`, `s3`, `anaconda`, `quetz`, `artifactory`, `filesystem`,
`github-packages`, `npmjs`). A kind in none of those groups is a destination the renderer can
describe and nothing can reach.

## What argues against this

- Removing `ghcr` is a **narrowing**, and narrowing vocabularies is how a format becomes too
  small for the next user. Someone publishing container images will now file an adapter task
  instead of a config line — which is the point, but it is friction.
- Two booleans with `false` allowed means a project can turn verification off and still call
  itself released. The mitigation is that the render prints the reduction; it is not a refusal.
- `verifier` remains a two-value enum where a `command` store might want a third ("ask the
  destination's CLI"). Adding one later is an amendment, not a config change.

## Conditions that would change this decision

- An OCI adapter task is filed: restore `ghcr` in the same change, with its servicing mechanism
  named in the adapter task rather than here.
- A store appears whose evidence source is neither a file nor a registry record: add a third
  `verifier` value and the job that reads it.
- `verify_published` proves too slow or too flaky at real publish counts: the honest response is
  a per-store opt-out with the same visible reduction, not a silent default flip.

## Consequences

- `decision-127` loses `ghcr` from its `kind` table; `decision-128` loses it from the
  `needs_index: false` column.
- `UNI-41`'s schema admits exactly the kinds above, and its compatibility lint rejects the rest
  as unknown rather than unsupported.
- Adding a kind later is two edits in one change: an adapter task and an amendment to this
  record. Neither is optional.
