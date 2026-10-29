---
id: decision-117
title: Deprecate pixi-sandbox after first Universe Airlock release
date: '2026-10-27T10:30:00Z'
status: proposed
---

## Context

pixi-sandbox is an interim developer transport mechanism (see [decision-116](decision-116%20-%20Use-pixi-sandbox-for-interim-developer-transport.md)). It provides locked developer environments and Cargo source vendoring before Universe Airlock implements its own reusable catalog and release mechanisms.

The publish-sandbox workflow currently depends on pixi-sandbox as a Pixi utility dependency. Once Universe Airlock has its first stable release (v0.1.0 or later), Universe Airlock itself will provide the necessary tooling for catalog management, artifact publishing, and developer environment transport.

## Decision

The pixi-sandbox dependency and the publish-sandbox workflow are **transitional**. They must be removed from the workspace when the following conditions are met:

1. Universe Airlock has published its first stable release (v0.1.0 or later on the `release` channel)
2. Universe Airlock's native catalog and transport mechanisms are implemented and validated
3. The publish-sandbox workflow has been replaced by Universe Airlock's native publishing pipeline

### Deprecation criteria

When Universe Airlock v0.1.0 is released:

- Remove `pixi-sandbox` from `[feature.utils.dependencies]` in `pixi.toml`
- Remove the publish-sandbox workflow (`.github/workflows/publish-sandbox.yml`)
- Remove sandbox-related tasks from `pixi.toml` (sandbox, sandbox-init, sandbox-check, sandbox-plan, sandbox-bootstrap, sandbox-publish)
- Remove sandbox scripts (`scripts/sandbox.py` references)
- Remove sandbox test suites (`tests/automation/test_sandbox.py`)
- Update `config/sandbox-bootstrap.json` or remove entirely

### Interim policy

Until deprecation:

- The publish-sandbox workflow runs on manual dispatch (workflow_dispatch) and on lockfile changes (pixi.lock, Cargo.lock) to main
- The workflow does not inherit release/OIDC rights
- pixi-sandbox remains pinned to the reviewed version (0.9.0)
- All sandbox-related checks remain in CI until native mechanisms are validated

## Consequences

### Positive

- Clear deprecation path once native mechanisms are available
- Prevents permanent dependency on external tooling
- Maintains security boundary (sandbox workflow does not have release rights)

### Negative

- Requires tracking deprecation criteria and triggering removal
- Dual maintenance burden until deprecation
- Transition requires careful validation of native mechanisms before removing sandbox

## Status boundary

Decision is proposed. Implementation will occur when Universe Airlock v0.1.0 is released and native catalog/transport mechanisms are validated. The pixi-sandbox dependency remains active until explicit deprecation.

The removal list under "Deprecation criteria" predates the current layout: there is no `utils` feature, `scripts/sandbox.py`, `tests/automation/test_sandbox.py`, or `config/sandbox-bootstrap.json` in this repository. `UNI-27` owns the removal and must re-derive the file list from the tree at that time instead of following the list above; `doc-110` states the transport, bootstrap, and `UNI-27` boundary.
