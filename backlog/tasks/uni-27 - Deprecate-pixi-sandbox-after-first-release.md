---
id: UNI-27
title: Deprecate pixi-sandbox after first Universe Airlock release
status: To Do
date: 2026-10-27
priority: medium
labels: [deprecation, infrastructure]
---

## Description

Remove pixi-sandbox dependency and publish-sandbox workflow once Universe Airlock's native catalog and transport mechanisms are implemented and validated.

## Acceptance Criteria

- [ ] Universe Airlock has published v0.1.0 or later stable release
- [ ] Native catalog management is implemented
- [ ] Native developer environment transport is validated
- [ ] Remove `pixi-sandbox` from `pixi.toml` `[feature.utils.dependencies]`
- [ ] Remove `.github/workflows/publish-sandbox.yml`
- [ ] Remove sandbox tasks from `pixi.toml` (sandbox, sandbox-init, sandbox-check, sandbox-plan, sandbox-bootstrap, sandbox-publish)
- [ ] Remove sandbox scripts from `scripts/sandbox.py`
- [ ] Remove sandbox test suites from `tests/automation/test_sandbox.py`
- [ ] Remove or update `config/sandbox-bootstrap.json`
- [ ] Update documentation to reflect native mechanisms

## Related

- Decision: [decision-117 - Deprecate pixi-sandbox after first Universe Airlock release](../decisions/decision-117%20-%20Deprecate-pixi-sandbox-after-first-universe-airlock-release.md)
- Original: [decision-116 - Use pixi-sandbox for interim developer transport](../decisions/decision-116%20-%20Use-pixi-sandbox-for-interim-developer-transport.md)

## Notes

This is a transitional task. Do not execute until all acceptance criteria are met. The pixi-sandbox dependency provides critical developer transport functionality until native mechanisms are ready.
