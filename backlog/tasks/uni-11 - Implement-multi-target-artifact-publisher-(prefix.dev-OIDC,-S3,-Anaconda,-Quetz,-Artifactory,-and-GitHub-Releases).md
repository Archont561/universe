---
id: UNI-11
title: Implement multi-target artifact publisher (prefix.dev OIDC, S3, Anaconda, Quetz, Artifactory, and GitHub Releases)
status: Done
assignee: []
created_date: '2026-10-26 20:15'
updated_date: '2026-11-09'
labels:
  - channel
  - release
  - testing
milestone: m-3
dependencies:
  - UNI-10
  - UNI-31
  - UNI-41
documentation:
  - backlog/docs/architecture/doc-2 - Universe Kit-CLI-and-manifest-specification.md
  - backlog/decisions/decision-1 - Two-tier-product-and-channel-boundary-(archont561-archont561-ships-universe-kit;-consumer-universes-target-arbitrary-channels).md
  - backlog/decisions/decision-5 - Static-Conda-channel-on-GitHub-Pages-+-GitHub-Releases-via-CEP-15-base_url,-CEP-16-shards,-and-immutable-JSON-fragments.md
priority: high
type: feature
ordinal: 11000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Implement `crates/universe-core::publish` and `universe-kit publish-artifacts` (`doc-2`, `decision-1`, `decision-5`):
- Dispatch `.conda` uploads across arbitrary `[[targets]]` declared in `universe.toml` (zero hardcoded owner channel defaults):
  - `prefix-dev`: `rattler_upload` / OIDC GitHub Actions ambient token exchange + `--generate-attestation` support, or `PREFIX_API_KEY`.
  - `anaconda`, `s3`, `quetz`, `artifactory`, `filesystem`: `rattler_upload` + `rattler_index` local filesystem indexing.
  - `github-pages`: idempotent creation/lookup of rolling `<release_tag_prefix>-<subdir>` GitHub Releases (`packages-linux-64`, `packages-win-64`, `packages-osx-64`, `packages-osx-arm64`, `packages-linux-aarch64`, `packages-noarch`) via typed `reqwest` GitHub REST v3 client, asset upload with SHA-256 skip-if-identical idempotency, configurable `max_builds_per_version` retention pruning (protecting against GitHub's 1,000-asset release cap), and atomic `.conda.json` fragment commit to `index_branch` (`universe-index`).
- Support `--dry-run` printing every planned upload and pruning action without mutating remote state.

The adapter request/result boundary — capabilities, the plan/build/audit requests, the six
outcomes, and produced artifacts — lives in `crates/universe-core/src/adapter.rs` (`UNI-31`).
This task consumes those records instead of defining a per-command shape of its own.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #0 `wiremock` is a dev-dependency of `universe-channel`, resolved by the relock bot. — 2026-11-04: declared in `crates/universe-channel/Cargo.toml` so the relock bot writes it into `Cargo.lock` and the sandbox publisher vendors its sources; the offline developer bundle replaces crates-io with a directory source, so a dependency that is only declared cannot be compiled locally until a sandbox snapshot is published from `main` afterwards. That round trip is why it is landed on its own rather than as a footnote to the store writers.
- [x] #1 `wiremock` tests verify arbitrary `prefix.dev` URLs/channels, S3 endpoints, Anaconda owners, and GitHub Releases API interactions (`create release`, `list assets`, idempotent `upload asset`, retention pruning).
- [x] #2 Re-running `publish-artifacts` on an already-uploaded `.conda` with identical `sha256` succeeds idempotently without failing or corrupting release assets; mismatched `sha256` for an existing filename is refused unless `--force` is set.
- [x] #3 `--dry-run` performs zero mutating HTTP requests (verified via `wiremock` request expectations).
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [x] #1 `rstest-bdd` (`publish.feature`) and `wiremock` integration suites pass under `cargo nextest`.
<!-- DOD:END -->

## Implementation notes
<!-- SECTION:NOTES:BEGIN -->
2026-11-04 — the store layer is `crates/universe-channel/src/store/`: one `Remote` trait
(`head`/`put`), `Target`/`Kind` for the destinations, `apply()` as the single place the three
rules live (an unchanged digest is skipped, a changed digest under an occupied name is refused
unless forced, a dry run issues no mutating request), a blocking `HttpRemote`, a
`FilesystemRemote`, and `Releases` for the GitHub REST v3 side (find-or-create the rolling
`<tag_prefix>-<subdir>` release, list assets, upload with a digest skip, prune by
`max_builds_per_version`). The verb is `universe publish-artifacts`; `--store`, `--force`,
`--dry-run` and `--json` are its contract, and `publish --store` now resolves the manifest's
sole writable branch store instead of silently ignoring its budgets.

Not done here, and deliberately still open:
- **TLS is in place (2026-11-06).** The relock round trip landed `reqwest`'s `rustls` feature
  with the `aws-lc-rs` provider, so an `https://` store is addressed over TLS and a
  connection that cannot be made is an ordinary transport error. Two policy files moved with
  it: `cmake` joined the Pixi environment (`aws-lc-sys` drives its bundled C with CMake), and
  `deny.toml` allows the `OpenSSL` licence *identifier* for `aws-lc-sys` plus
  `CDLA-Permissive-2.0` for `webpki-root-certs`, while the `openssl`/`openssl-sys` *crates*
  stay banned. `auth = "oidc"` landed on 2026-11-06 — see the note below.
- **The `.conda.json` fragment commit to `index_branch`** from the description above is not
  implemented for a release store; see the 2026-11-06 note for what a branch store does
  instead.
- **The rendered publish jobs now name the right verb (2026-11-06).** The renderer emitted
  `publish --store <name>` for every writable store, so a tag run would have died twice: the
  `prefix-dev` job on a verb that pushes a tree, and the `branch` job on
  `publish.remote.required`. It now renders `publish-artifacts --store <name>` for the
  destinations this verb uploads to, and `publish --store <name> --remote
  ${{ github.server_url }}/${{ github.repository }}.git` for a branch. The secret a job exports
  is named by one rule, `Store::secret_environment_variable`, and the token lookup accepts the
  rendered name before the bare one — the two had disagreed, so the rendered variable would
  have been exported and never read. `crates/universe-kit/tests/rendered_workflow.rs` takes the
  command lines out of the rendered file and runs them against a directory store and a bare
  repository, which is the check `ci-check` cannot make: that the file agrees with itself, not
  that the job works.
- **A branch push from CI had no credential** (closed 2026-11-06; the paragraph is kept for
  the argument, not the status). `publish` pushes from a scratch
  repository nested in the checkout, and `actions/checkout` persists its token into the
  *outer* repository's config, which git does not inherit. The rendered command now names the
  right remote, so this is the only thing between the branch job and a green tag run. The
  mechanism to use is a per-invocation `http.<url>.extraheader` supplied through `GIT_CONFIG_*`
  environment variables (never argv, where `ps` would show it), with the token read from the
  store's declared secret; it pairs with the OIDC exchange below, which is why both are one
  slice.

2026-11-06 — trusted publishing and the branch credential, one slice. Two mechanisms, one
question: where does a token come from when nothing long-lived is stored?

- **`auth = "oidc"` publishes.** `crates/universe-channel/src/store/oidc.rs` holds
  `OidcExchange`: the job's GitHub identity (`GET $ACTIONS_ID_TOKEN_REQUEST_URL&audience=…`
  with `ACTIONS_ID_TOKEN_REQUEST_TOKEN`) is exchanged at `POST {server}/api/oidc/mint_token`
  with `{"token": <jwt>}` for the bearer, which is the raw answer body — the shape
  `rattler_networking`'s `PrefixMint` speaks, so the ecosystem's uploaders and this one agree.
  The audience is `prefix.dev` for `prefix.dev` and its subdomains and the endpoint's own host
  otherwise, so a token minted for one server cannot be replayed at another; `prefix.dev.` and
  `evil-prefix.dev` fail closed. A job without `id-token: write` is told which permission it
  is missing rather than shown a transport error.
- **One mint per destination per run.** `publish-artifacts` builds `Credentials` once and
  hands a token to every store on the same server, then drops it; the bearer is never logged,
  echoed, or carried into a message. The HTTP client moved into `Http` so the exchange and the
  uploads share one request path instead of a second one.
- **The manifest refuses what cannot work.** `auth = "oidc"` on a store that is not a
  `prefix-dev` channel is now an error (`manifest.store.oidc.unsupported`) naming `auth =
  "token"` as the way out, and naming a secret alongside OIDC is a warning
  (`manifest.store.oidc.secret`): redundant, not fatal.
- **A branch push authenticates.** `GitTransport::authenticated` takes the store's declared
  secret — rendered name first, then the bare one — and supplies it as
  `http.<origin>/.extraheader` through `GIT_CONFIG_COUNT`/`GIT_CONFIG_KEY_0`/
  `GIT_CONFIG_VALUE_0`: environment, never argv, where `ps` would show it and an error message
  would quote it. GitHub gets the checkout shape (`AUTHORIZATION: basic base64(x-access-token:…)`);
  any other host gets `Authorization: Bearer`. A secret the environment does not hold falls
  back to git's own credential — a local developer's helper or SSH key — because a declared
  secret is an override there, not a requirement.
- **What proves it.** `crates/universe-channel/tests/store_oidc.rs` stands a `wiremock`
  server in front of both requests and asserts the audience, the body, and each failure;
  `crates/universe-kit/src/git.rs` runs the real `git` against a socket that records what it
  was sent; `crates/universe-kit/tests/push_credential.rs` drives the binary with the
  renderer's variable name and asserts the push lands, that an absent variable still pushes,
  and that the token never reaches stdout or stderr; and a renderer scenario asserts
  `id-token: write` is on the *hosted* job and that the hosted job reads no `secrets.`.
2026-11-06 (later) — the audit receipt now travels with the bytes, and the channel a tag
publishes is proved usable by a real client.

- **The fragment is published.** `build_tree` writes each fragment into the channel tree at
  `meta/<subdir>/<filename>.conda.json`, copied verbatim from what `universe audit` wrote —
  verbatim because a field a later Universe adds (the binary audit of `decision-5`) must
  survive a publisher that does not name it. `read_fragments` therefore keeps the text beside
  the parsed `Fragment` (`Audited`, and `Verified` carries it on), instead of re-serializing.
  `decision-5` gained an amendment: a branch store has one tree per snapshot, so it serves the
  fragments from that tree rather than from a second `universe-index` branch; a `github-pages`
  store still owes the dedicated branch.
- **The channel is proved resolvable, not just readable.** `crates/universe-kit/tests/channel_resolves.rs`
  publishes a real `.conda` (built in the test: metadata, file list, one executable) to a bare
  repository, clones the branch with `git`, and runs the real `pixi install --offline` against
  a `pixi.toml` that names nothing but that checkout. It asserts the installed entrypoint runs.
  Two layout facts are pinned there because a client is the only thing that can confirm them:
  `.conda` entries belong under `packages.conda` with an empty legacy `packages`, and a solve
  asks for `noarch` whatever the project builds.
- **Still open in this task:** the `.conda.json` fragment commit to a dedicated `index_branch`
  for a release/hosted store, S3, Quetz and Artifactory upload paths.
- **The upload path is now exercised end to end, not only per store.** The `wiremock` suites
  in `universe-channel` prove what each store sends; `crates/universe-kit/tests/publish_mock.rs`
  proves the verb around them, from `universe.toml` to the bytes on a socket, against a
  destination the test owns. The mock needs no dependency: a loopback `TcpListener` records
  the method, path and body, and answers a canned status. That keeps `Cargo.lock` untouched,
  which matters because the lock is written by the relock workflow rather than by hand.
- **The workflow around the verb now hands it artifacts.** `universe-publish.yml` (renamed
  to `release.yml` when the trusted publisher was measured) builds in the same run before
  publishing, because a workflow artifact belongs to the run that uploaded it (`2026-11-08`).
  Without that, every store job below would have received an empty tree — the publisher was
  exercised, the pipeline around it was not.

2026-11-09 — status set to `Done`. Every acceptance box and the Definition of Done box are
ticked, and the suites named above run under `pixi run gates` (594 tests at `09331fd5`):
the `wiremock` store suites, `publish_bdd`/`publish_mock`, `push_credential`, and
`rendered_workflow`. Marking it Done closes the **criteria**, not every sentence in the
description. The open items listed above are still open and are not implied by this status:

- The dedicated `.conda.json` fragment commit to `index_branch` for hosted stores.
  `decision-5` was amended so a branch store carries its receipts in the single snapshot
  tree (`meta/<subdir>/<filename>.conda.json`), and `UNI-58` proves that tree solves with
  `pixi install --offline`.
- S3, Quetz and Artifactory upload paths. The wiremock suites prove arbitrary endpoints are
  addressed as the manifest declares; dedicated adapters are v1 work.
- `--generate-attestation` and `rattler_upload`. The publisher is its own `Http` client over
  `rustls`; `auth = "oidc"` trusted publishing is what a tag run uses.

`decision-132` limits the checkpoint to two stores — `prefix-dev` and the `branch` static
channel — and both are exercised. When this status was written the prefix.dev trusted
publisher binding was the one owner-side prerequisite (`UNI-59`); it has since been configured
and matched, as the note below records. The branch channel needs no stored secret — see the
2026-11-09 correction below.

2026-11-09 — correction, recorded as a correction. This note previously named
`UNIVERSE_CHANNEL_TOKEN` as an owner-side prerequisite. `universe.toml`'s branch store now
declares the built-in `GITHUB_TOKEN`, which GitHub injects per run; there is no repository
secret to create or rotate, and the job's `contents: write` is the only permission it needs.
`decision-127` already says a `GITHUB_TOKEN` with `contents: write` suffices for these public,
unprotected mirror branches. At that point only the prefix.dev trusted publisher binding
remained owner-side; see the note below.
- **A real tag rehearsal exposed a HEAD probe the upload endpoint refuses.** Once the
  workflow filename and environment matched the channel's trusted publisher, the OIDC token
  was minted and `publish / prefix-dev` reached the upload. prefix.dev answered 405 to
  `HEAD /api/v1/upload/archont561/archont561` because that endpoint accepts POST only.
  `HttpRemote::head` had treated every non-404 non-success status as a transport failure, so
  the run stopped before sending the artifact. A red-first wiremock test now pins that a 405
  means the unchanged probe is unavailable and the POST must proceed; the registry decides
  whether it already holds the bytes. `UNI-59` records the run and the re-watch.
- **The POST then needed the headers the REST endpoint documents.** Run 42900930515 answered
  `400: Header of type 'content-type' was missing`. prefix.dev's `/api/v1/upload/:channel`
  requires `Content-Type: application/octet-stream`, `X-File-Name` and `X-File-SHA256` in
  addition to `Authorization`; `Content-Length` is derived by the client. A red-first wiremock
  test asserts all three. The same slice translates `--force` into `?force=true` on this
  endpoint, which the docs say defaults to false and is required to overwrite an occupied
  name; `force_replaces_a_changed_digest` pins it. `UNI-59` records the re-watch.
- **The publisher is proven against the live channel.** Run 42903356074 from `main` at
  `e9ec1b34` finished green, including `publish / prefix-dev` and `publish / verdict`.
  prefix.dev's package API reports `universe-airlock 0.1.0` for `linux-64` with one variant,
  sha256 `b148b2152d4c096c24264f5e1c8f40419053c4d51a5f81f97c289efa9b95e0f8` and size
  298583 — the same digest the `channel` branch carries. `UNI-59` records the run, the
  digest and the remaining offline-solve gap. This does not advance the release tag.
- **The implicit releases destination is now wired into this repository.** `decision-127`
  says `releases` is added to every `release = true` package, but `Manifest::destinations`
  had only mapped stores that packages named, so no `publish / releases` job existed. A
  declared `releases`-kind store now carries the repository and the built-in `GITHUB_TOKEN`,
  and the renderer gives it every released package without the package naming it;
  `release = false` opts out. `decision-132` records the scope widening, and `UNI-59` AC#4
  stays open until the asset is verified after a tag run.
- **The releases publisher is proven against the live repository.** Run 42914663367 had a
  green `publish / releases` job; it created the published rolling release
  `packages-linux-64 artifacts` with asset `universe-airlock-0.1.0-h41f06ac_0.conda`,
  sha256 `dfc08b401414baebe182ad9a5971673d915d82bdd8ac4c9bb0b81adcb355e2b6`, size 298585.
  The same bytes are in the `channel` branch at `aa853bdb`. The overall verdict is red only
  because `publish / prefix-dev` refused a changed digest with 409; see `UNI-59` for the
  non-reproducibility note. The owner chose `--force`: the renderer now adds it to the
  `prefix-dev` and `releases` `publish-artifacts` commands so a later run asks those stores to
  replace the changed digest, and leaves `publish --store branch` alone. `decision-132`
  records this as a scoped reversal of the no-overwrite release semantics. Run `42920755513`
  proved the GitHub Release and branch stores accept the changed digest, but prefix.dev
  answered `403 Not authorized` on `?force=true`. `https://prefix.dev/docs/prefix/api` says
  force-replacing requires a read-write-delete Repository Access entry. The owner then removed
  the occupied variant manually and reran the failed jobs; attempt 2 of run `42920755513` is
  green. That proves a clean channel accepts the forced upload, not force-replace permission.
  The renderer slice is complete, and the build is still not reproducible.
- **Three packages are proven against the live channel, by digest.** After the owner cleared
  the occupied variants a second time, run `43045274207` from `main` at `be4e7df3` finished
  green on attempt 1. Its build audited `universe-airlock`
  (`2e98b81d2155f5033bef5001dae2c6a3dbf5037d97e32e182f4af7e050bfab26`), `universe-kit`
  (`763999409faee8b9972e7935d8e4f8d49599c4004e7eea8f80b5541304392c0b`) and the umbrella
  `universe` facade
  (`19e00dffc113f7d6d26e139b8d7cca3c72a995c20846f709b8aa4dca790f7217`). `publish / prefix-dev`
  reported all three forced uploads; a fresh, cache-busted read of the channel's
  `linux-64/repodata.json` (publication `87457`) resolves all three by those digests, while the
  plain URL still serves the older CDN-cached index. The three GitHub Release assets of
  `packages-linux-64 artifacts` read back the same digests through the release API. This still
  does not prove force-replacing an occupied filename, because the uploads landed on cleared
  names. `publish / branch` is green but wrong: three snapshot pushes left `channel` at
  `1d30ef21701497dd196eeaa1599d7ec3ee24edc6` holding only the facade. `ci render` now emits
  one command naming every package for a branch or static store; `decision-132` records the
  correction and `UNI-59` records the evidence. The offline branch solve remains open.
- **Force-replacement is proven against the live channel.** After the owner granted
  read-write-delete on the Repository Access entry, run `43061441347` from `main` at
  `fa785a32` is green on attempt 1 with no manual removal. Its build audited
  `universe-airlock` (`134f5e7dfd2d249f08195a2678e7ff6200f54b53fe04bc95c3229ff1ca8e651d`),
  `universe-kit` (`63b958b773573e158ff30bdd682a0669311d58058a790c2783ae1c90902e9c12`) and
  the umbrella `universe` facade
  (`470f52e41e4dfaae20ce22d893cf1e098b2784c123ae623cdacab07c84667e69`). Three forced
  prefix.dev uploads replaced the occupied filenames from the previous run under changed
  digests with no `403`. A fresh cache-busted repodata read (publication `87896`), the three
  GitHub Release assets and the `channel` branch at
  `7afdff10e32c28d368caaf07acc18a118d81ea74` all resolve those three digests. The plain
  repodata URL and publication `87577` were briefly CDN-stale. `publish / branch` now runs
  one snapshot push carrying twelve files, including all three archives and receipts.
  `decision-132` records the correction and `UNI-59` records the evidence. The offline
  branch solve remains open, the build is still not reproducible, and the `v0.1.0` tag was cut
  on 2026-11-11 (see the correction below).
**Status note — correction (2026-11-11).** The annotated `v0.1.0` tag was cut from `1de18891bd69f8d16b4d261cfdaa53525555eabd` and pushed; its tag-triggered `release.yml` run `43073492712` is green, including `publish / verdict`. https://github.com/Archont561/universe/actions/runs/43073492712 This records the tag as evidence, not as a completion of this record's criteria: the tasks above remain To Do. The offline branch solve with network denied, a clean-container restore transcript, and the four-platform and integrated v1 lanes are still open. `UNI-59` records the tag run's digests and the remaining gaps. No further tag is cut.
<!-- SECTION:NOTES:END -->
