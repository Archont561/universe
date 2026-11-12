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
| **pixi** | `pixi.lock` — conda archives *and* PyPI wheels | every recorded URL with its sha256 | a prefix, with build-time paths relocated, `noarch` packages placed for the interpreter and their entry points generated |
| **bun** | `bun.lock`, `package-lock.json` | `resolved` + `integrity` per package | `node_modules` from the mirror only |
| **cargo** | `Cargo.lock` | crate downloads with checksums | a vendor directory and `.cargo/config.toml` |

The two scopes are separate on purpose: restoring *your own toolchain* and consuming *someone
else's package set* are different provenances, and a single block would let one restore mix a
local compiler with a foreign catalog.

Restore is a replay, not a solve: the lock names every artifact, Airlock verifies every digest,
and a missing artifact fails closed rather than falling back to a channel
(`backlog/decisions/decision-102`, `-103`).

## Mirror publishes what a lock pins

`universe airlock mirror --adapter <name>` is the producer half of a replay. It reads the
project's lockfile, fetches every artifact the lock names, verifies each digest, and publishes
the result as one tree on a branch named `<mirror prefix>/<adapter>`, which is `mirror/pixi` for
the default prefix.

The tree is content-addressed: an artifact lands at `blobs/sha256/<digest>/<file>`, so the path
is a function of the bytes. One lock therefore produces one tree, and two runs of the same lock
produce the same tree, which is why a republish of an unchanged lock creates no commit. A
changed lock adds its new artifacts beside the ones already there; an artifact a newer lock
dropped stays on the branch, because a consumer restoring an older pin still needs it.

`mirror.json` in that tree records what the lock pinned and where each artifact landed, so a
consumer can map a lock entry to a path without knowing the layout.

Two flags exist for the runs that should change nothing. `--dry-run` reports how many artifacts
the lock pins and how many the mirror is missing, without fetching or publishing. `--check`
does the work of a real run and then compares the staged tree with the branch, reporting whether
the destination already holds it, without pushing.

Three refusals are worth stating, because each of them is a failure mode that would otherwise
surface much later: an artifact whose bytes do not match its recorded digest, an artifact the
destination does not have, and a lock record that names no digest at all. Each names the package
and exits non-zero, and nothing is published.

The adapters today are `pixi` (`pixi.lock`, conda archives and PyPI wheels) and `cargo`
(`Cargo.lock`, registry crates). `bun` is a declared format whose reader has not landed, so
`--adapter bun` is refused with the two names this build does read rather than silently doing
nothing. A `Cargo.lock` dependency that comes from a Git revision is refused in the same way:
no registry serves those bytes, and there is no checksum to verify them against.

## Replay installs a mirrored lock

`universe-airlock-bootstrap replay --mirror <dir> [--prefix <dir>]` is the consumer half. It
reads the `mirror.json` that tree carries rather than a lock, because that index is what a lock's
format was reduced to: a path and a digest per artifact. The binary links no HTTP client, so the
run cannot reach a channel even by accident.

It works in two passes. The first checks every artifact against the digest and the size the index
recorded and reads its `info/index.json`; a set that is damaged, absent, or names two builds of
one package fails there, before a single file is written. The second installs in dependency order
— dependencies first, ties and cycles by name — because a `noarch: python` package installs into
an interpreter the prefix has to hold already. A dependency with no artifact in the set is a
refusal naming the package, never a fetch.

The prefix it produces is the environment: `prefix/bin` is what a caller puts on `PATH`. A lock
that also pins wheels replays what a prefix can hold and names what it left out, since a Conda
prefix holds Conda packages.

## Sandbox runs a set you just built

`universe-airlock sandbox --package <file.conda> [--package <file.conda>] -- <command>` installs
those archives into a prefix and runs the command inside it, so a package can be tried before it
is published. `--mirror <dir>` does the same from a mirrored lock instead of loose files.

The install is the bootstrapper's, so an archive is digest-checked and its `info/paths.json`
applied exactly as a restore does. A set that cannot be installed never runs the command: the
whole point of the verb is that a broken package produces a refusal rather than a program that
happens to work.

Inside the run, `<prefix>/bin` is first on `PATH`, `CONDA_PREFIX` names the prefix, and
`PYTHONPATH`, `PYTHONHOME`, `PYTHONSTARTUP`, `PYTHONEXECUTABLE`, `VIRTUAL_ENV` and `CONDA_PREFIX_1`
are removed while `PYTHONNOUSERSITE` is set. Without that, a `noarch: python` package imports
modules from outside the set it was installed with and the run passes for a reason that has
nothing to do with the packages.

It is an **environment** sandbox, not a machine one. No mount, user, or network namespace is
created, so the command can still reach the network and the filesystem outside the prefix, and
every run says so on stderr. What is real: the set is verified before the first command runs, the
environment is scoped to the prefix, and the prefix is deleted when the run ends unless `--keep`
or `--dir` asks for it. `--dir` also refuses a directory that is not empty, because installing one
set over another produces an environment that is neither.

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
