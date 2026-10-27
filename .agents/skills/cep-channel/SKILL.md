---
name: cep-channel
description: Implement and verify CEP-15 base_url static Conda channels, CEP-16 sharded repodata, and per-subdir GitHub Releases indexing for Universe Kit. Use when working on channel indexing, fragments, or publish-artifacts.
metadata:
  author: Archont561
  version: "0.1.0"
---

# CEP-15 & CEP-16 Static Conda Channel Invariants

## Critical Channel Rules

1. **Single `base_url` per subdir (`repodata_version: 2`)**: CEP-15 specifies one `info.base_url` per `<subdir>/repodata.json`. Store `.conda` assets on rolling per-subdir GitHub Releases (`packages-linux-64`, `packages-win-64`, `packages-osx-64`, `packages-osx-arm64`, `packages-linux-aarch64`, `packages-noarch`), never scattered across versioned release tags.
2. **Empty subdir initialization**: Always emit valid `repodata.json(.zst)` and `repodata_shards.msgpack.zst` for all standard subdirs even when zero packages exist in a subdir so `pixi lock` never receives HTTP 404.
3. **Deterministic serialization**: Use `BTreeMap` ordering for all repodata maps and guard permutation invariance with `proptest`.
