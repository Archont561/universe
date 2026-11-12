# Offline sandbox (orphan branch)

Built 2026-11-12T20:25:05Z from commit `98669e8` for platform `linux-64`.
`pixi.lock` sha256 `b853b7ad0c5a741074ce7d9ecfabc2650e130d76ea478af741c3e9ab7a5766eb`.

The verified self-bootstrap binary is stored at `.pixi-sandbox/tools/linux-64/pixi-sandbox`. The branch root intentionally contains documentation only.

| env | platform | packed | unpacked | files |
| --- | --- | ---: | ---: | ---: |
| `default` | linux-64 | 678.1 MiB | 2598.0 MiB | 107 |

Cargo dependencies: **457 crates**, 464.7 MiB (loose) from `Cargo.lock` sha256 `b6f861ba8df2…`; restore materialises them to `.pixi-sandbox/vendor/`. Built with cargo 1.99.1 (5f94df478 2026-08-27); rustc 1.99.1 (58cf8f9c3 2026-10-13).


## Restore on the disconnected machine

```bash
./.pixi-sandbox/tools/linux-64/pixi-sandbox doctor --branch-location . --verify
./.pixi-sandbox/tools/linux-64/pixi-sandbox restore --branch-location . --output-path <project> --force
# then, from <project> with no network, use pixi as the sole entrypoint:
pixi install --frozen --offline
pixi run --frozen -- cargo build --offline
```

Every manifest blob is verified before it is written into the working tree.
