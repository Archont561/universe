---
id: doc-106
title: Offline verification and acceptance gates
type: specification
created_date: '2026-10-25T23:49:00Z'
updated_date: '2026-10-27T09:20:00Z'
---

# Offline verification and acceptance gates

**Status: proposed product proof plan, not evidence that Universe Airlock is already offline-capable.** Current VERSION tests and developer/release tooling checks do not satisfy these gates.

## Evidence levels

| Level | What it establishes |
|---|---|
| Inventory verified | Listed bytes, integrity/trust policy, object/view references, and selected graph coverage validate |
| Install verified | Named native-client scenarios materialize packages under the recorded script and isolation policy |
| Build verified | Named approved builds/tests, including required tools/assets/system dependencies, pass under that policy |
| Authenticity verified | The release manifest carries a valid signature from the pinned key, so origin is established separately from correspondence (`decision-131`) |

Report the exact bundle/snapshot ID, target/system profile, input/lock hashes, client/helper fingerprints, transport, commands/exits, script policy, isolation mechanism, graph comparison, and descendant network attempts. A signed bundle is not a vulnerability-free or arbitrary-compatibility claim, and a digest match is not an
authenticity claim: the first three levels can all hold for a bundle somebody rewrote.

## Isolation and clean-state rules

Use clean HOME/XDG state, package caches, working trees, install prefixes, node_modules, and Cargo targets. Only explicitly bundled native-cache derivatives may be present in files mode. Provision required tools/root filesystems in advance; certification must not fetch container images, rustup components, or missing tools as a fallback.

- **HTTP:** start the read-only server and clients inside the same isolated network context, enable loopback, deny external IPv4/IPv6/DNS routes/proxies, and begin with empty native caches. Do not blindly enable client offline flags that would block the intended loopback fetches.
- **Files:** run no target package server, block Internet-family network operations for the full process tree, and use client offline/frozen settings.
- **Both:** unprivileged execution, dropped capabilities/no-new-privileges, no host network namespace handles or privileged/Unix proxy sockets, and trace children including build scripts/proc macros.

A network namespace is not a complete hostile-code sandbox. If the required enforcement is unavailable, fail certification or explicitly label a non-certified/operator-delegated diagnostic. Distinguish denied attempted egress from no attempted egress.

## Required native-client matrix

| Area | Required proof |
|---|---|
| Projectless conda | Rattler resolution/acquisition without Pixi/prefix export; consumers install from HTTP and file channels |
| Effective conda metadata | Known upstream hotfix dependency/constraint survives filtered metadata |
| Pixi locked replay | All selected environments/solve groups; empty-cache install; original inputs preserved |
| Pixi files relocation | Typed local-channel inputs pass locked/offline validation and target-qualified graph equivalence |
| Pixi wheels | Index and artifact origins, Python ABI, wheel hashes, and name mapping remain local |
| npm locked replay | Lock v2/v3, scopes, aliases, peers, optional/platform variants, workspaces, and original SRI |
| npm files mode | Persistent packuments and tarballs survive transfer to fresh state; no listener; offline ci/add succeeds |
| Cargo | Sparse and local-registry source replacement; unchanged lock; build/dev/optional/renamed/target entries retained |
| Curated additions | A root absent from each original project can be added within the admitted finite universe in both modes |
| External assets | Declared assets are consumed without download fallthrough; Playwright/browser launch needs native target proof |
| Relocation | Different import path/filesystem works without exporter-specific paths or hardlink assumptions |

A successful HTTP install does not prove filesystem-only support. A successful solve does not prove installation; script-disabled materialization does not prove a working native/browser application.

## Mandatory negative and recovery tests

- Missing package/version, incompatible roots/features/peers, missing cache metadata, and a changed native-client fingerprint fail intelligibly without public fallback.
- Corrupt blobs, shards, indexes, inventory, signatures, or metadata commitments fail before activation; untrusted embedded keys do not become trusted.
- Reject unsafe archive paths/links/devices, duplicate/unlisted files, traversal/percent-decoding attacks, oversized extraction, and incomplete downloads.
- Interrupted fetch/import/publication is resumable or safely restarted without exposing partial active state. Snapshot drift requires explicit replanning.
- Hostile project registry/mirror/proxy configuration cannot bypass generated policy. Check effective project config, not only environment/global settings.
- An approved test child deliberately attempting curl/DNS/IPv6 egress is denied and recorded. No host service socket bypass is available.
- Tokens, auth headers, signed-URL secrets, private keys, and .env content do not enter transferred data/logs. Reject unsafe original inputs instead of secretly redacting them while claiming byte-identical preservation.
- Same-name packages from distinct sources cannot overwrite one another. No advertised artifact lacks verified bytes.

## Implementation sequence

First prove tiny conda/Pixi, mixed-wheel, npm, Cargo, and egress fixtures in both modes with exact clients. Then build durable CAS/inventory/import, broaden replay, add curated scenarios, and harden transport/bootstrap. Do not certify additional platforms or unsupported source builds merely because release/configuration files mention them.

## Lazy-fetch acceptance extensions

Test both modes in the [catalog design](doc-109%20-%20Git-backed-catalogs-and-on-demand-command-execution.md): allowed-source cache fills and fully offline local-only consumption. Prove cache hits need no remote access; missing offline/promisor objects fail without attempted fetch; concurrent misses verify once; index/object drift cannot be served; source tokens never leak to a different origin; mixed catalog heads are rejected; and only admitted endpoints are available to descendants. Test every platform the native command resolves, not just the host. Bootstrap availability alone is not a complete profile's install/build proof.
