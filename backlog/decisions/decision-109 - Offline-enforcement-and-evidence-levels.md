---
id: decision-109
title: Offline enforcement and evidence levels
date: '2026-10-25T23:49:00Z'
status: accepted
---

## Context

Package inventory, successful installation, application builds, and network isolation are distinct claims. Native offline flags cannot constrain arbitrary child processes or downloaded build code.

## Decision

Support local HTTP and filesystem-only product consumption with actual external-egress enforcement. HTTP mode allows only the intended loopback service; files mode runs no package server and blocks Internet-family operations. Use clean state and trace descendants, IPv6/DNS, and policy bypass attempts.

Report inventory-, install-, and build-verified evidence separately, naming exact profiles/clients/script policy and enforcement. Missing enforcement must fail certification or be explicitly labeled non-certified/operator-delegated.

## Consequences

Do not blindly enable offline flags in cold-cache HTTP tests. Do not infer working applications from script-disabled installs, or safety from signatures. Current VERSION/tooling tests do not satisfy these product gates.

## Status boundary

Accepted requirement; product acceptance tests are pending. See [verification plan](../docs/doc-106%20-%20Offline-verification-and-acceptance-gates.md).

## 27 October 2026 refinement

Approved-Git-connected lazy retrieval is not a full air gap. Sealed offline mode requires a local complete scenario closure, including suppression of unintended Git promisor fetching; see decision-115 and doc-109.
