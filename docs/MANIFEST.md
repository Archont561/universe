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
notes = "docs/releases/0.1.0.md"     # the body the release page shows, read from this file

[[stores]]
name = "conda-static"                # a real conda channel: <channel>/<subdir>/repodata.json
kind = "static"
branch = "universe/conda-channel"    # the ref the snapshot tree is pushed to
writable = true
needs_index = true                   # we generate and upload the index, so it is reviewable
read_auth = "none"                   # consumers need no credentials to restore
verifier = "file"

[[stores]]
name = "pages"                       # the same channel, served as a static site
kind = "static"
writable = false                     # nothing uploads here: the site is generated at release time
needs_index = true                   # a deployment serves an index; render one from the build
url = "https://someone.github.io/channel"
pages = true                         # adds the generate-and-deploy jobs to release.yml

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

[[stores]]
name = "releases"                    # the one implicit destination: no package names it
kind = "releases"
channel = "Archont561/universe"      # owner/repo for the GitHub REST API
writable = true
auth = "token"
secrets = ["GITHUB_TOKEN"]
verifier = "registry"
```

### How a package finds its stores

There is no autodetection and no "publish everywhere" switch: a package names its destinations by
**store name**, and everything else is validation.

- `publish` lists names, never kinds — two `npmjs` stores (a company scope, a public scope) are
  distinguishable in the manifest and indistinguishable by kind.
- **`releases` is the only implicit destination.** Declare one `releases`-kind store; the
  renderer wires every package with `release = true` to it without the package naming it.
  Opting out is `release = false`. The store carries the `owner/repo` channel and the token,
  so the destination stays reviewable even though the package does not name it.
- **Kind compatibility is a lint error, not a publish-time surprise.** `conda` may name
  `static`/`prefix-dev`/`anaconda`; a crate may name `crates-io` and an npm store, but only if it
  also declares the npm output — a wrapper that was never built cannot be uploaded.
- **A writable store must have something that can publish to it.** `crates-io`, `npmjs`,
  `github-packages`, `pypi` and `command` are declared but not implemented as destinations yet,
  so a writable one is an error: its publish job would build, pass its verdict, and ship
  nothing. The same store read-only is legal and only warns where it is useless, because
  reading a registry works long before writing to it does. Whether a kind publishes is
  `StoreKind::publishes`, and the workflow renderer derives its verb table from it, so the two
  cannot drift apart.
- **The reverse is checked too.** A writable store that no package names is a half-applied
  rename, and it warns.
- **A snapshot store names the ref it publishes to.** `branch` on a `branch` or `static` store is
  where the tree is pushed; the renderer emits `--branch` from it, and a store that omits it gets
  the tool's default, `universe/conda-channel`. Declaring it is how a project keeps a name it
  chose, and how a rename stays visible in the diff rather than in the tool's history. A registry
  has no ref, so `branch` there is an error rather than a no-op, and an empty one is an error
  too.
- **A release's body is a file, and its name carries the version.** `notes` on a `releases`
  store is a repository-relative path; the renderer emits `--notes-file` from it, so the job
  publishes the text a reviewer read in the pull request instead of a sentence the publisher
  composed. The file is read verbatim and refused when it is missing or empty. Only a release
  has a front page, so `notes` on any other kind is an error. The release's tag stays
  `<tag_prefix>-<subdir>` because a tag is an address a link keeps using, while its title is
  `universe (linux-64) 0.1.0` — project, subdir and the version the assets agree on. When the
  title or body no longer matches, the publisher patches the release rather than leaving a stale
  name on a tag that moved. A subdir holding two versions names only the platform.
- **A hosted index is a store that publishes nothing.** `pages = true` on a `static` store tells
  the renderer to add two jobs: one renders the channel from the release's own fragments with
  `universe index` and uploads it as a Pages artifact, the other deploys that artifact. The
  store must also be `writable = false`, declare `needs_index = true`, and name the `url` the
  index advertises as its own base. Those four facts are checked together because each of the
  alternatives is a silent failure: a writable Pages store would render a publish job whose kind
  has no upload, an index-less one would deploy an empty directory over the site, and a site
  whose address is derived from repository naming is wrong for any layout but a project page.
  Nothing is committed: the tree exists only inside one release run, so it cannot describe a
  channel the release did not publish. GitHub is not asked to trust a tag by default — the
  `github-pages` environment it creates allows the default branch — so add the release tag
  pattern (`v*`) as a deployment tag rule in that environment once, or the deployment is refused
  after the artifact uploads.
- **A trusted publisher can name a GitHub environment.** Declare it with `environment` on the
  store, for example `environment = "prefix"`. The renderer puts that name on the one publish
  job for that store, so its OIDC token carries the matching `environment` claim. An empty name
  is an error; naming one on a store that does not publish with `auth = "oidc"` warns, because
  no identity token is minted there.
- **`prefix-dev` and `releases` stores render `--force`.** Both keep one file under a stable
  name and refuse a changed digest by default; their `publish-artifacts` commands carry the
  flag so a later `v0.1.0` publish run asks to replace those bytes. prefix.dev accepts
  `?force=true` only from a Repository Access entry with read-write-delete. The first forced
  run returned `403 Not authorized`; after the owner manually removed the occupied variant,
  the rerun is green. That rerun did not prove force-replacing an occupied name with the
  entry as it stood then; run `43061441347` proved force-replacement after the owner granted
  read-write-delete. This is a scoped reversal of
  `decision-126`, recorded in `decision-132`. The CLI default is unchanged, and the `publish`
  command for a branch or static channel never carries `--force`.

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
