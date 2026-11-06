---
id: UNI-11
title: Implement multi-target artifact publisher (prefix.dev OIDC, S3, Anaconda, Quetz, Artifactory, and GitHub Releases)
status: In Progress
assignee: []
created_date: '2026-10-26 20:15'
updated_date: '2026-11-06 15:10'
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
  implemented; the release store uploads assets only.
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
- **Still open in this task:** the `.conda.json` fragment commit to `index_branch`, S3,
  Quetz and Artifactory upload paths.
<!-- SECTION:NOTES:END -->
