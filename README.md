# Offline sandbox (orphan branch)

Built 2026-10-31T00:56:49Z from commit `8e4e444` for platform `linux-64`.
`pixi.lock` sha256 `6c33668bab95e46f7c8662f8802966cf5e054de155e40596b3edaf319c596ac2`.

The verified self-bootstrap binary is stored at `.pixi-sandbox/tools/linux-64/pixi-sandbox`. The branch root intentionally contains documentation only.

| env | platform | packed | unpacked | files |
| --- | --- | ---: | ---: | ---: |
| `default` | linux-64 | 635.0 MiB | 2432.5 MiB | 89 |

Cargo dependencies: **418 crates**, 391.8 MiB (loose) from `Cargo.lock` sha256 `bf4ed7683b88…`; restore materialises them to `.pixi-sandbox/vendor/`. Built with cargo 1.99.1 (5f94df478 2026-08-27); rustc 1.99.1 (58cf8f9c3 2026-10-13).


## Restore on the disconnected machine

```bash
./.pixi-sandbox/tools/linux-64/pixi-sandbox doctor --branch-location . --verify
./.pixi-sandbox/tools/linux-64/pixi-sandbox restore --branch-location . --output-path <project> --force
# then, from <project> with no network, use pixi as the sole entrypoint:
pixi install --frozen --offline
pixi run --frozen -- cargo build --offline
```

Every manifest blob is verified before it is written into the working tree.
