# The manifest

One `universe.toml` describes what you build and where it goes. This page is the full
example; the reference rules live in `backlog/decisions/decision-127`.

Kinds are per package, so `universe-kit` can be a conda package and a wheel at the same version,
and the npm package can be an alias of another — without a second config file anywhere.

```toml
# universe.toml — what you ship, and where it goes.

[project]
name = "universe"
version = "0.1.0"                    # one version for every artifact in the train (decision-122)
summary = "Self-hosted universes of software, data, models, and skills"

[release]
tag_pattern = "v{version}"           # one tag for the whole train
promote = "manual"                   # a prerelease attempt is a draft; promotion publishes it
changelog = "CHANGELOG.md"

# ─────────────────────────────────────────────────────────── packages: what to build ──
[[packages]]
name = "universe-kit"
kind = "conda"                       # conda | pypi | npm | cargo | r
command = "rattler-build build --recipe crates/universe-kit/recipe/recipe.yaml"
platforms = ["linux-64", "osx-arm64", "win-64"]
outputs = ["output/universe-kit/**/universe-kit-*.conda"]
smoke = "universe-kit version"       # mandatory: a package without a smoke test cannot publish
publish = ["conda-static", "conda-prefix", "conda-anaconda"]
submit = ["conda-forge"]             # a review queue, not a destination — see below

[[packages]]
name = "universe-airlock"
kind = "conda"
command = "rattler-build build --recipe crates/universe-airlock/recipe/recipe.yaml"
platforms = ["linux-64", "osx-arm64", "win-64"]
outputs = ["output/universe-airlock/**/universe-airlock-*.conda"]
smoke = "universe-airlock version"
publish = ["conda-static", "conda-prefix", "conda-anaconda"]

[[packages]]
name = "universe-kit"                # same name, different protocol
kind = "pypi"
command = "uv build packages/kit-py"
outputs = ["packages/kit-py/dist/*.whl"]
smoke = "uv pip install --find-links dist universe-kit && universe-kit version"
publish = ["pypi", "pypi-static"]

[[packages]]
name = "@archont561/universe"
kind = "npm"
command = "bun run --cwd packages/cli build"
outputs = ["packages/cli/dist/*.tgz"]
smoke = "npm install --prefix smoke ./dist/*.tgz && node smoke/bin.js version"
publish = ["npm", "github-packages"]

[[packages]]
name = "universe"                    # unscoped alias, same bytes, same version
kind = "npm"
command = "bun run --cwd packages/cli-alias build"
outputs = ["packages/cli-alias/dist/*.tgz"]
smoke = "npm install --prefix smoke ./dist/*.tgz && node smoke/bin.js version"
publish = ["npm", "github-packages"]

[[packages]]
name = "universe-core"
kind = "cargo"
command = "cargo package --workspace --locked"
outputs = ["target/package/*.crate"]
smoke = "cargo install --path crates/universe-core --locked"
publish = ["crates-io", "npm"]       # the npm copy is the wrapper built from the same crate
catalog = { title = "Universe core", license = "MIT" }

# ──────────────────────────────────────────────────────────── stores: where it goes ──
# `releases` is added to every package automatically. Opt one out with `release = false`.

[[stores]]
name = "releases"                    # the default destination for everything below
kind = "releases"
writable = true

[[stores]]
name = "conda-static"                # a real conda channel: <channel>/<subdir>/repodata.json
kind = "static"
writable = true
needs_index = true                   # we generate and upload the index, so it is reviewable
read_auth = "none"                   # consumers need no credentials to restore
verifier = "file"

[[stores]]
name = "conda-prefix"
kind = "prefix-dev"
writable = true
needs_index = true
verifier = "file"
secrets = ["PREFIX_DEV_TOKEN"]

[[stores]]
name = "conda-anaconda"
kind = "anaconda"
writable = true
needs_index = true
verifier = "registry"
secrets = ["ANACONDA_TOKEN"]

[[stores]]
name = "pypi"
kind = "pypi"
writable = true
verifier = "registry"
auth = "oidc"                        # trusted publishing: no secret held at all
                                     # the first publish of a new name needs auth = "token"

[[stores]]
name = "pypi-static"                 # PEP 503 simple/ index, no server process
kind = "static"
writable = true
needs_index = true
verifier = "file"

[[stores]]
name = "npm"
kind = "npmjs"
writable = true
verifier = "registry"                # `npm view` reads the published copy back
secrets = ["NPM_TOKEN"]

[[stores]]
name = "github-packages"
kind = "github-packages"
writable = true
read_auth = "token"                  # unlike npmjs, even consumers need a token
verifier = "registry"

[[stores]]
name = "crates-io"
kind = "crates-io"
writable = true
verifier = "registry"
auth = "oidc"                        # same shape via rust-lang/crates-io-auth-action
```

### How a package finds its stores

There is no autodetection and no "publish everywhere" switch: a package names its destinations by
**store name**, and everything else is validation.

- `publish` lists names, never kinds — two `npmjs` stores (a company scope, a public scope) are
  distinguishable in the manifest and indistinguishable by kind.
- **`releases` is the only implicit destination.** It is added to every package, and opting out
  is `release = false`.
- **Kind compatibility is a lint error, not a publish-time surprise.** `conda` may name
  `static`/`prefix-dev`/`anaconda`; a crate may name `crates-io` and an npm store, but only if it
  also declares the npm output — a wrapper that was never built cannot be uploaded.
- **The reverse is checked too.** A writable store that no package names is a half-applied
  rename, and it warns.

`ci render` prints both derived views — the job each store produces, and the packages that land
in it — so the mapping is inspectable instead of inferred from a YAML file.

### conda-forge is not a store

You cannot publish to conda-forge — it is a review queue with a recipe pull request. So it lives
under `submit`, and Universe opens the `staged-recipes` change for you, refuses duplicates
upstream, and reuses the branch on a retry. Everything else in the manifest above is a
destination Universe writes to directly.

## What it becomes

The jobs each file contains, and how they are ordered, are described in
[WORKFLOWS.md](WORKFLOWS.md).
