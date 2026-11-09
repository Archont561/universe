# Airlock

The same pins that produced the release restore it. Airlock never solves, never resolves, and
never reaches a channel: it replays a lockfile against mirrored artifacts and verifies every
digest.

```toml
[airlock.developer]                  # my toolchain, from my own pins
adapters = ["pixi", "cargo"]
mirror = "universe/developer"

[airlock.delivery]                   # someone else's catalog, pinned by commit
catalog = "github:Archont561/universes@8f3c1aa"
adapters = ["pixi", "bun", "cargo"]
```

| Adapter | Reads | Mirrors | Restores |
| :-- | :-- | :-- | :-- |
| **pixi** | `pixi.lock` — conda archives *and* PyPI wheels | every recorded URL with its sha256 | a prefix, with build-time paths relocated |
| **bun** | `bun.lock`, `package-lock.json` | `resolved` + `integrity` per package | `node_modules` from the mirror only |
| **cargo** | `Cargo.lock` | crate downloads with checksums | a vendor directory and `.cargo/config.toml` |

The two scopes are separate on purpose: restoring *your own toolchain* and consuming *someone
else's package set* are different provenances, and a single block would let one restore mix a
local compiler with a foreign catalog.

Restore is a replay, not a solve: the lock names every artifact, Airlock verifies every digest,
and a missing artifact fails closed rather than falling back to a channel
(`backlog/decisions/decision-102`, `-103`).

## Verify reads, never writes

`universe-airlock verify --payload <dir>` answers four questions about a sealed payload
before anything is activated: is the release manifest readable, is the payload authentic,
is every named artifact present with the size and SHA-256 the manifest records, and does the
payload hold anything the manifest does not name. A refusal names a stable code on stderr
and returns exit code 1; stdout stays empty, including in `--json` mode. A successful
`--json` run writes one versioned envelope.

Authenticity is assessed first, before any artifact byte is hashed. The `developer` lane is
the default while no release key is published (`UNI-55`); it accepts an unsigned payload but
says so on stderr. The `sealed` lane refuses an unsigned payload, and a signed payload is
refused with `airlock.signature.no-key` until the compiled-in key arrives. That direction is
deliberate: an unverifiable payload fails closed rather than being accepted.

The rules themselves are the bootstrap's verification library. `decision-122` makes the
bootstrap a phase of Airlock, not a separate edition, so both executables call one
implementation instead of growing two sets of integrity rules.
