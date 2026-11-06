# Offline sandbox (orphan branch)

Built 2026-11-06T09:53:53Z from commit `dcaddd7` for platform `linux-64`.
`pixi.lock` sha256 `f4954736e9d93941290ade83508fd57209fb58b5d633fb10b593f702de3ffd95`.

The verified self-bootstrap binary is stored at `.pixi-sandbox/tools/linux-64/pixi-sandbox`. The branch root intentionally contains documentation only.

| env | platform | packed | unpacked | files |
| --- | --- | ---: | ---: | ---: |
| `default` | linux-64 | 666.8 MiB | 2541.6 MiB | 106 |

Cargo dependencies: **452 crates**, 463.8 MiB (loose) from `Cargo.lock` sha256 `15dafae663be…`; restore materialises them to `.pixi-sandbox/vendor/`. Built with cargo 1.99.1 (5f94df478 2026-08-27); rustc 1.99.1 (58cf8f9c3 2026-10-13).


## Restore on the disconnected machine

```bash
./.pixi-sandbox/tools/linux-64/pixi-sandbox doctor --branch-location . --verify
./.pixi-sandbox/tools/linux-64/pixi-sandbox restore --branch-location . --output-path <project> --force
# then, from <project> with no network, use pixi as the sole entrypoint:
pixi install --frozen --offline
pixi run --frozen -- cargo build --offline
```

Every manifest blob is verified before it is written into the working tree.
