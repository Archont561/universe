---
id: decision-111
title: Optional UX testing and update libraries
date: '2026-10-25T23:49:00Z'
status: proposed
---

## Context

The discussion considered CLI/core layering, richer diagnostics, interactive init, optional browsing, self-update, and a larger testing stack. The authorized application implementation remains VERSION-only.

## Decision

**No additional stack is adopted by this record.** Preserve candidates for later evaluation: Ratatui for optional browsing, Inquire for human init questions, thiserror/Miette (and narrowly useful Anyhow) for errors/diagnostics, rstest/proptest and limited rstest-bdd feature contracts, plus CLI/snapshot/mock-HTTP test utilities where justified.

Clap is already used for minimal version/help handling. A future standalone update may evaluate an installer-receipt-aware axoupdater/self_update approach. Pixi-owned installations must update through Pixi rather than self-overwriting another installer's files.

## Consequences

No TUI, questionnaire, updater, BDD files, or speculative dependencies should be added just to satisfy this recommendation. CI remains noninteractive. Keep UI out of core/bootstrap, choose libraries together against real requirements, and retain tests outside src.

## Status boundary

**Proposed**, not approved implementation work. See the [status index](../docs/doc-101%20-%20Universe Airlock-design-and-status.md) and [lifecycle](../docs/doc-103%20-%20Init-bootstrap-and-restore-lifecycle.md).
