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

Eight things are worth noticing, because they are the design:

- **Publishing is split by destination, not by ecosystem.** Each job holds exactly one write
  scope and one secret. Artifact stores upload one file per step, serially — two writers racing
  on one `repodata.json` produce an index that silently omits a package. A branch or static
  store is different: `publish` force-pushes one whole orphan snapshot tree, so `ci render`
  emits **one command naming every package** for that store. Three pushes of one package each
  would leave only the last tree on the branch, which is what run `43045274207` did before the
  renderer was corrected. The command also carries `--branch <ref>` when the store declares
  `branch`, so the ref a release writes is the one the manifest names rather than whatever the
  tool's default happens to be that week.
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
- **A hosted index is generated, not committed.** When the manifest declares a `pages` store,
  `release.yml` gains `<store>` and `<store>-deploy` after the destination jobs. The first
  downloads every build artifact, runs `universe index --output output/pages --base-url
  <store.url>`, and uploads that directory with `actions/upload-pages-artifact` as the
  `github-pages` artifact; the second needs no
  checkout and no Pixi, holds `pages: write` with `id-token: write`, names the store's
  `environment` (default `github-pages`), and runs `actions/deploy-pages` alone. Splitting them
  is what lets the deployment job carry the two OIDC permissions and nothing else. The tree is
  rebuilt from each release's own fragments, so a page can only ever describe bytes the run
  published, and no generated file enters the repository. One setting is outside the renderer:
  GitHub creates the `github-pages` environment with a deployment policy that names a branch,
  and a release is a tag, so the environment needs a tag rule for the pattern the release uses
  (`v*`) before a tag run can deploy. Without it the upload succeeds and the deployment is
  refused with `Tag "v0.1.0" is not allowed to deploy to github-pages due to environment
  protection rules`.
- **GitHub Releases is implicit.** A declared `releases`-kind store receives every package
  with `release = true` without the package naming it; `release = false` opts out. The job
  holds `contents: write` and exports the store's `GITHUB_TOKEN`, and the rolling release is
  named `<tag_prefix>-<subdir>` (`packages-linux-64` by default).
- **The `v0.1.0` artifact stores are allowed to replace changed bytes.** A rebuild can occupy
  the same package filename with a different digest. The renderer adds `--force` to the
  `publish-artifacts` commands for `prefix-dev` and `releases` stores, so the hosted channel
  is asked to accept the upload with `?force=true` and the rolling release replaces its asset.
  This is a scoped reversal of the no-overwrite release semantics, recorded in `decision-132`;
  it does not make the build reproducible. prefix.dev grants force-replacing only to a
  Repository Access entry with read-write-delete. Run `42920755513` attempt 1 received `403
  Not authorized`; the owner removed the occupied variant manually and reran the failed jobs,
  so attempt 2 is green. Run `43045274207` is green on attempt 1 for all three packages after
  the occupied variants were cleared a second time, and a fresh cache-busted prefix.dev
  repodata read (publication `87457`) resolves all three by digest. Those uploads landed on
  cleared names and did not prove force-replacement. After the owner granted read-write-delete
  on the Repository Access entry, run `43061441347` re-watched the corrected renderer with no
  manual removal and is green on attempt 1: three forced uploads replaced occupied filenames
  under changed digests with no `403`, and a fresh cache-busted repodata read (publication
  `87896`) resolves all three by the new digests. The plain URL and publication `87577` were
  briefly CDN-stale. Force-replacement with this entry is now proven. `publish --store branch`
  takes no `--force`, because it already replaces one snapshot tree — and now does so in one
  command naming every package, so the branch carries all three. The offline branch solve is
  still unrecorded. The tag run `43073492712` is green with a new set of three digests, read back
  at all three destinations; the `v0.1.0` tag is cut from `1de18891`. That run's artifacts were
  built before the recipe fix, so a `pixi install --offline` against a clone of `channel` failed
  on `libgcc >=16`, a run export from `stdlib('c')` that a static channel does not carry. The
  channel was rebuilt from the fix and force-republished without cutting a tag: run
  `43230051708` is green on attempt 1, and its three packages, the three GitHub Release assets,
  the one-snapshot `channel` branch at `0743618978d44b3393511f8777a9dfafe513cc95` and a fresh
  cache-busted prefix.dev read (publication `93323`) agree on three digests that declare no
  `libgcc`. Three Pixi projects, one per package, each ran `pixi install --offline` against a
  fresh clone of `channel` and each installed, so the offline solve is recorded rather than
  assumed. A forced artifact upload replaces the stored file at once while the channel index
  changes only when prefix.dev reindexes, which took more than 28 minutes on the first attempt
  and left the index naming the previous publication: read the index after the reindex, not
  after the upload. A clean-container restore transcript is still unrecorded: no container
  runtime is available here. The owner-lane manifest declares a restore scope now, pinned at a
  published `bootstrap` ref, so the entrypoint exists and runs against the real remote.
  https://github.com/Archont561/universe/actions/runs/43230051708

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
