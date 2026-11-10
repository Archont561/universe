# Generated workflows

The manifest declares capabilities; the renderer writes the YAML. Nothing here is hand-edited
and there is no `--sync-workflows` flag: `universe ci render` writes, `universe ci check`
refuses to let the result drift.

```text
6 packages → 3 platforms → 1 release → 9 destinations

$ universe ci render
  universe.yml          plan · validate · build(6×3) · audit · verify-artifact · verdict
  release.yml           build(6×3) · publish/releases · publish/conda-static
                        publish/conda-prefix · publish/conda-anaconda · publish/pypi
                        publish/pypi-static · publish/npm · publish/github-packages
                        publish/crates-io · verify-index · verdict
  universe-airlock.yml  mirror/pixi · mirror/bun · mirror/cargo · render-airlock · verdict
```

Six things are worth noticing, because they are the design:

- **Publishing is split by destination, not by ecosystem.** Each job holds exactly one write
  scope and one secret. Within a destination, packages publish serially — two writers racing on
  one `repodata.json` produce an index that silently omits a package.
- **Jobs run in order of how reversible their destination is.** Releases, static channels and
  mirrors first, npm and PyPI next, crates.io last — because a crates.io version can only be
  yanked, never deleted, and a release that fails late has already written the earlier ones.
- **A file is generated only if you declared the capability.** No stores, no publish file; no
  lock adapters, no airlock file. A consumer repository gets one file.
- **The publish file builds what it publishes.** A workflow artifact belongs to the run that
  uploaded it, so a publish job that downloaded one would find nothing unless its own file
  built it first. The cost is a second compilation during a release, paid on purpose: a
  release should not depend on another run's artifacts still existing.
- **The tag-triggered file is named `release.yml` to match its trusted publisher.** A
  prefix.dev publisher entry names a workflow filename, and GitHub puts that basename in the
  job's `workflow_ref` claim. A descriptive name the entry does not know cannot mint a token.
  `ci render` removes the former `universe-publish.yml`, so one tag never starts two publish
  workflows.
- **GitHub Releases is implicit.** A declared `releases`-kind store receives every package
  with `release = true` without the package naming it; `release = false` opts out. The job
  holds `contents: write` and exports the store's `GITHUB_TOKEN`, and the rolling release is
  named `<tag_prefix>-<subdir>` (`packages-linux-64` by default).
- **The `v0.1.0` artifact stores are allowed to replace changed bytes.** A rebuild can occupy
  the same package filename with a different digest. The renderer adds `--force` to the
  `publish-artifacts` commands for `prefix-dev` and `releases` stores, so the hosted channel
  accepts the upload with `?force=true` and the rolling release replaces its asset. This is a
  scoped reversal of the no-overwrite release semantics, recorded in `decision-132`; it does
  not make the build reproducible. `publish --store branch` takes no `--force`, because it
  already replaces one snapshot tree.

The manifest declares capabilities; the renderer writes the YAML. No hand-editing, no
`--sync-workflows` flag buried on another command.

```toml
[workflow]
checks = { format = "pixi run fmt-check", lint = "pixi run lint", typecheck = "pixi run typecheck" }
verify_artifact = true
verify_published = true
```

`workflow_dispatch` and `workflow_call` expose a **closed input schema**, filled in at render time:

| Input | Type | Default | Declared by |
| :-- | :-- | :-- | :-- |
| `dry_run` | boolean | `false` | always |
| `scope` | choice | first scope | `[[release.scope]]` |
| `package` | choice | `all` | `[[packages]].name` |
| `platform` | choice | `all` | package platforms |
| `store` | choice | `all` | writable `[[stores]]` |

A dry run skips the step that writes and **keeps the verification**, so it proves the plan
resolved, the credentials are addressed, and the bytes match the digest they would have been
checked against. Two dry runs of one input produce identical output — which is what makes
"dry-run parity" a testable claim rather than a promise.

```sh
universe ci render --dry-run    # print the files, write nothing
universe ci render              # write them
universe ci check               # fail on any drift, name the file that drifted
```

## The rules behind the shape

`backlog/decisions/decision-128` is the contract: required checks, one write scope per job,
the closed `[workflow].checks` vocabulary, the publish split, the journal, and the dry-run
parity rule.
