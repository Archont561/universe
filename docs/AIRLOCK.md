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
