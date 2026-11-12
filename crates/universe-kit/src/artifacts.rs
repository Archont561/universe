//! `universe publish-artifacts`: send built `.conda` files to the stores a manifest declares.
//!
//! `publish` moves a *tree* to a Git branch; this verb sends one artifact at a time to a
//! destination that owns the identity of a file. The two never share a code path, because the
//! failure they must prevent differs: a branch is replaced wholesale and idempotent by
//! construction, while a registry keeps what it was first given, so the publisher has to
//! compare digests before it writes and refuse to overwrite.
//!
//! Order is the same argument as `publish`'s. Fragments are read and proved against the bytes
//! they name, the stores are resolved from the manifest and nothing else, and only then does
//! an artifact leave the machine. A `--dry-run` builds the same plan and performs no mutating
//! request.

use std::collections::BTreeMap;
use std::env;
use std::path::Path;

use serde_json::json;
use universe_channel::store::{
    FilesystemRemote, HttpRemote, Kind, OidcExchange, Policy, ReleaseReport, Releases, Report,
    StoreError, Target, TrustedPublisher, Upload, apply,
};
use universe_core::CommandOutput;
use universe_core::diagnostic::{Diagnostic, Severity};
use universe_core::manifest::{Store, StoreAuth, StoreKind};

use crate::channel::{FRAGMENT_DIRECTORY, Verified, read_fragments, select, verify};
use crate::options::{Options, VerbSpec};
use crate::output::{FINDINGS, USAGE, failure, human, json_success};
use crate::project::Project;

/// The tag prefix a release store uses when the manifest declares none.
///
/// A protocol default, not an owner default: every project files its releases the same way,
/// and the subdir is what distinguishes them.
const DEFAULT_TAG_PREFIX: &str = "packages";

/// Run `universe publish-artifacts`.
pub(crate) fn run(args: &[String]) -> CommandOutput {
    let options = match Options::parse(args, VerbSpec::publish_artifacts()) {
        Ok(options) => options,
        Err(message) => {
            return failure(
                USAGE,
                &[Diagnostic::new(
                    Severity::Error,
                    "publish.usage.invalid",
                    message,
                )],
            );
        }
    };

    let project = match Project::load(&options.root) {
        Ok(project) => project,
        Err(output) => return output,
    };
    if let Err(output) = project.require_valid() {
        return output;
    }

    let fragment_directory = options
        .fragments
        .clone()
        .unwrap_or_else(|| project.root.join(FRAGMENT_DIRECTORY));
    let fragments = match read_fragments(&fragment_directory) {
        Ok(fragments) => select(&fragments, &options.packages),
        Err(diagnostic) => return failure(FINDINGS, &[diagnostic]),
    };
    let verified = match verify(&project.root, &fragments) {
        Ok(verified) => verified,
        Err(diagnostic) => return failure(FINDINGS, &[diagnostic]),
    };
    let uploads = uploads_of(&verified);

    let (stores, warnings) = match stores_for(&project, options.store.as_deref()) {
        Ok(selection) => selection,
        Err(diagnostic) => return failure(USAGE, &[diagnostic]),
    };
    let policy = Policy {
        force: options.force,
        dry_run: options.dry_run,
    };

    let mut credentials = Credentials::ambient();
    let mut published: Vec<serde_json::Value> = Vec::new();
    let mut body = String::new();
    for store in &stores {
        match publish_to(store, &uploads, policy, &mut credentials) {
            Ok(outcome) => {
                body.push_str(&outcome.describe(store, policy));
                published.push(outcome.json(store, policy));
            }
            Err(error) => {
                return failure(FINDINGS, &[error.into_diagnostic(store)]);
            }
        }
    }

    if options.json {
        return json_success(
            "publish-artifacts",
            json!({
                "dry_run": options.dry_run,
                "force": options.force,
                "stores": published,
            }),
            warnings,
            Vec::new(),
        );
    }
    if body.is_empty() {
        body.push_str("no writable store takes an artifact upload\n");
    }
    let mut stdout = body;
    for warning in &warnings {
        stdout.push_str(&format!("warning: {}\n", warning.message()));
    }
    human(stdout)
}

/// One store's publication, before it is rendered.
enum Outcome {
    /// A destination that holds files at paths.
    Uploaded(Report),
    /// A destination that holds releases of assets, one per subdir.
    Released(Vec<ReleaseReport>),
}

impl Outcome {
    fn describe(&self, store: &Store, policy: Policy) -> String {
        let verb = if policy.dry_run {
            "would upload"
        } else {
            "uploaded"
        };
        match self {
            Self::Uploaded(report) => format!(
                "{verb} {} artifact(s), {} byte(s) to '{}' ({} already there)\n",
                report.uploaded.len(),
                report.bytes,
                store.name,
                report.unchanged.len()
            ),
            Self::Released(reports) => {
                let mut text = String::new();
                for report in reports {
                    text.push_str(&format!(
                        "{verb} {} asset(s), {} byte(s) to '{}' release {} ({} already there",
                        report.uploaded.len(),
                        report.bytes,
                        store.name,
                        report.tag,
                        report.unchanged.len()
                    ));
                    if report.pruned.is_empty() {
                        text.push_str(")\n");
                    } else {
                        text.push_str(&format!("; pruned {})\n", report.pruned.join(", ")));
                    }
                }
                text
            }
        }
    }

    fn json(&self, store: &Store, policy: Policy) -> serde_json::Value {
        match self {
            Self::Uploaded(report) => json!({
                "store": store.name,
                "kind": store.kind.as_str(),
                "dry_run": policy.dry_run,
                "uploaded": report.uploaded,
                "unchanged": report.unchanged,
                "bytes": report.bytes,
            }),
            Self::Released(reports) => json!({
                "store": store.name,
                "kind": store.kind.as_str(),
                "dry_run": policy.dry_run,
                "releases": reports
                    .iter()
                    .map(|report| json!({
                        "tag": report.tag,
                        "created": report.created,
                        "uploaded": report.uploaded,
                        "unchanged": report.unchanged,
                        "pruned": report.pruned,
                        "bytes": report.bytes,
                    }))
                    .collect::<Vec<_>>(),
            }),
        }
    }
}

/// The stores this run publishes to, with a warning for every writable store it cannot.
fn stores_for(
    project: &Project,
    named: Option<&str>,
) -> Result<(Vec<Store>, Vec<Diagnostic>), Diagnostic> {
    if let Some(name) = named {
        let Some(store) = project
            .manifest
            .stores
            .iter()
            .find(|candidate| candidate.name == name)
        else {
            return Err(Diagnostic::new(
                Severity::Error,
                "publish.store.unknown",
                format!("no store named '{name}' is declared in universe.toml"),
            ));
        };
        if channel_kind(store.kind).is_none() {
            return Err(Diagnostic::new(
                Severity::Error,
                "publish.store.kind",
                format!(
                    "store '{name}' is a '{}' store, which `publish-artifacts` does not write to; \
                     a branch store is published with `universe publish`",
                    store.kind.as_str()
                ),
            ));
        }
        return Ok((vec![store.clone()], Vec::new()));
    }

    let mut selected = Vec::new();
    let mut warnings = Vec::new();
    for store in &project.manifest.stores {
        if !store.writable {
            continue;
        }
        if channel_kind(store.kind).is_some() {
            selected.push(store.clone());
        } else if !store.kind.is_git_branch() {
            warnings.push(Diagnostic::new(
                Severity::Warning,
                "publish.store.unsupported",
                format!(
                    "store '{}' is a '{}' store, which `publish-artifacts` does not write to yet",
                    store.name,
                    store.kind.as_str()
                ),
            ));
        }
    }
    Ok((selected, warnings))
}

/// Publish `uploads` to one store.
fn publish_to(
    store: &Store,
    uploads: &[Upload],
    policy: Policy,
    credentials: &mut Credentials,
) -> Result<Outcome, PublishError> {
    let Some(kind) = channel_kind(store.kind) else {
        return Err(PublishError::new(format!(
            "store '{}' is not a destination this verb writes to",
            store.name
        )));
    };
    if kind.is_local() {
        let root = store.url.as_deref().map_or_else(
            || {
                Err(PublishError::new(format!(
                    "store '{}' is a filesystem store and declares no url to write into",
                    store.name
                )))
            },
            Ok,
        )?;
        let remote = FilesystemRemote::new(Path::new(root));
        return apply(&remote, uploads, policy)
            .map(Outcome::Uploaded)
            .map_err(PublishError::from);
    }

    let target = target_for(store, kind, credentials)?;
    if kind == Kind::Releases {
        return publish_releases(store, &target, uploads, policy).map(Outcome::Released);
    }
    let remote = HttpRemote::new(target).map_err(PublishError::from)?;
    apply(&remote, uploads, policy)
        .map(Outcome::Uploaded)
        .map_err(PublishError::from)
}

/// Publish to one release per subdir, applying the store's retention.
fn publish_releases(
    store: &Store,
    target: &Target,
    uploads: &[Upload],
    policy: Policy,
) -> Result<Vec<ReleaseReport>, PublishError> {
    let releases = Releases::new(target).map_err(PublishError::from)?;
    let prefix = store.tag_prefix.as_deref().unwrap_or(DEFAULT_TAG_PREFIX);
    let mut reports = Vec::new();
    for subdir in subdirs(uploads) {
        let tag = format!("{prefix}-{subdir}");
        let group: Vec<Upload> = uploads
            .iter()
            .filter(|upload| upload.subdir() == subdir.as_str())
            .cloned()
            .collect();
        reports.push(
            releases
                .publish(
                    &tag,
                    Some(&format!("{tag} artifacts")),
                    &group,
                    policy,
                    store.max_builds_per_version,
                )
                .map_err(PublishError::from)?,
        );
    }
    Ok(reports)
}

/// The subdirs `uploads` mention, in order and without repeats.
fn subdirs(uploads: &[Upload]) -> Vec<String> {
    let mut found: Vec<String> = Vec::new();
    for upload in uploads {
        let subdir = upload.subdir().to_owned();
        if !found.contains(&subdir) {
            found.push(subdir);
        }
    }
    found
}

/// The destination a store's manifest entry describes.
///
/// The token comes from whichever authority the store named: a secret the environment holds, or
/// — for `auth = "oidc"` — an exchange of this job's identity for a token that expires long
/// before anyone could copy it.
fn target_for(
    store: &Store,
    kind: Kind,
    credentials: &mut Credentials,
) -> Result<Target, PublishError> {
    let endpoint = match store.url.as_deref() {
        Some(url) => url.to_owned(),
        None => match kind {
            Kind::PrefixDev => "https://prefix.dev".to_owned(),
            Kind::Releases => "https://api.github.com".to_owned(),
            _ => {
                return Err(PublishError::new(format!(
                    "store '{}' is a '{}' store and declares no url to publish to",
                    store.name,
                    store.kind.as_str()
                )));
            }
        },
    };
    let token = match store.auth {
        StoreAuth::Token => token_for(store)?,
        StoreAuth::Oidc => Some(credentials.exchange(store, kind, &endpoint)?),
    };
    Ok(Target {
        name: store.name.clone(),
        kind,
        endpoint,
        channel: store.channel.clone(),
        token,
    })
}

/// This run's authorization to the destinations that do not take a stored secret.
///
/// Two properties matter more than the code that follows. A token is **minted once per
/// destination**: the exchange is two round trips, and a run that publishes to two stores on
/// one server should not ask twice. And a token is **never printed** — it is not logged, not
/// echoed, and the error paths in this file carry the destination's answer, not the bearer.
#[derive(Debug)]
struct Credentials {
    /// Tokens already exchanged, keyed by the server they are good at.
    minted: BTreeMap<String, String>,
    /// Where an exchange comes from, until it is first needed.
    exchange: Exchange,
}

/// The identity an `auth = "oidc"` store exchanges.
#[derive(Debug)]
enum Exchange {
    /// The CI job this process is running in, read from the environment on first use. Reading
    /// it eagerly would make every publish in a developer's shell fail for the absence of
    /// GitHub variables that no token store needs.
    Ambient,
    /// An exchange the caller prepared, which is what a test supplies.
    Ready(Box<dyn TrustedPublisher>),
}

impl Credentials {
    /// Credentials for this process's own environment.
    fn ambient() -> Self {
        Self {
            minted: BTreeMap::new(),
            exchange: Exchange::Ambient,
        }
    }

    /// The same, over an exchange the caller (or a test) built.
    #[cfg(test)]
    fn ready(exchange: impl TrustedPublisher + 'static) -> Self {
        Self {
            minted: BTreeMap::new(),
            exchange: Exchange::Ready(Box::new(exchange)),
        }
    }

    /// A bearer token good at a store's endpoint.
    fn exchange(
        &mut self,
        store: &Store,
        kind: Kind,
        endpoint: &str,
    ) -> Result<String, PublishError> {
        // The mint endpoint is prefix.dev's convention, and no other kind of destination has
        // been shown to speak it. Guessing would send a job's identity somewhere that cannot
        // verify it, so the refusal names what could be done instead.
        if kind != Kind::PrefixDev {
            return Err(PublishError::new(format!(
                "store '{}' is a '{}' store and asks for `auth = \"oidc\"`, whose token exchange \
                 is only defined for a prefix.dev channel; publish to it with `auth = \"token\"`",
                store.name,
                store.kind.as_str()
            )));
        }
        if let Some(token) = self.minted.get(endpoint) {
            return Ok(token.clone());
        }
        let token = match &self.exchange {
            Exchange::Ready(exchange) => exchange.mint(endpoint)?,
            Exchange::Ambient => {
                let exchange = OidcExchange::from_env()?;
                let token = exchange.mint(endpoint)?;
                // Kept for the next store on the same server, and for the same reason the
                // exchange is built once: the environment does not change mid-run.
                self.exchange = Exchange::Ready(Box::new(exchange));
                token
            }
        };
        self.minted.insert(endpoint.to_owned(), token.clone());
        Ok(token)
    }
}

/// The token a store publishes with, read from the environment variable it names.
///
/// Names only, never values: the manifest says `secrets = ["GITHUB_TOKEN"]` and the value
/// reaches the process from the workflow's environment. Two spellings are accepted, and the
/// order matters: the *rendered* name first — `BRANCH_GITHUB_TOKEN`, which is what a workflow
/// job exports — and then the bare secret, which is what a local shell sets. The repository's
/// branch store names the built-in token, so there is no stored secret to manage.
/// Reading only the bare name was a live mismatch: the renderer exported one and the verb
/// looked up the other, so the first tag run would have found no token at all.
///
/// A store that names a secret the environment does not hold is an error rather than an
/// anonymous upload, because an anonymous upload fails later and less clearly.
fn token_for(store: &Store) -> Result<Option<String>, PublishError> {
    token_from(store, |name| env::var(name).ok())
}

/// The token a store's declared secret holds, when this environment has one.
///
/// The same two spellings as [`token_from`], for the verb that pushes a branch rather than
/// uploading an artifact. A store that declares its secrets somewhere else also names the
/// variable, so a caller that only wants "a token, if there is one" does not need to know the
/// naming rule.
pub(crate) fn declared_token(store: &Store) -> Option<String> {
    token_from(store, |name| env::var(name).ok()).ok().flatten()
}

/// The same lookup, over an injected environment, so the missing-token case is testable
/// without a test writing to the process environment every other test shares.
fn token_from(
    store: &Store,
    lookup: impl Fn(&str) -> Option<String>,
) -> Result<Option<String>, PublishError> {
    let Some(secret) = store.secrets.first() else {
        return Ok(None);
    };
    let rendered = store.secret_environment_variable(secret);
    for name in [rendered.as_str(), secret.as_str()] {
        if let Some(value) = lookup(name).filter(|value| !value.trim().is_empty()) {
            return Ok(Some(value));
        }
    }
    Err(PublishError::new(format!(
        "store '{}' publishes with the token in '{rendered}' (or '{secret}'), which is not set \
         in this environment",
        store.name
    )))
}

/// The artifacts a verified fragment set describes, as uploads.
fn uploads_of(verified: &[Verified]) -> Vec<Upload> {
    verified
        .iter()
        .map(|entry| {
            let fragment = &entry.fragment;
            let name = fragment
                .path
                .rsplit('/')
                .next()
                .unwrap_or(fragment.path.as_str());
            Upload::new(format!("{}/{name}", fragment.subdir), entry.bytes.clone())
        })
        .collect()
}

/// Which store protocol a manifest store kind speaks, when this verb can write to it.
fn channel_kind(kind: StoreKind) -> Option<Kind> {
    match kind {
        StoreKind::PrefixDev => Some(Kind::PrefixDev),
        StoreKind::Anaconda => Some(Kind::Anaconda),
        StoreKind::Quetz => Some(Kind::Quetz),
        StoreKind::Artifactory => Some(Kind::Artifactory),
        StoreKind::S3 => Some(Kind::S3),
        StoreKind::Filesystem => Some(Kind::Filesystem),
        StoreKind::Releases => Some(Kind::Releases),
        _ => None,
    }
}

/// A publish failure, before it is attributed to a store.
#[derive(Debug)]
struct PublishError {
    message: String,
    code: &'static str,
}

impl PublishError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            code: "publish.store.failed",
        }
    }

    fn into_diagnostic(self, store: &Store) -> Diagnostic {
        Diagnostic::new(
            Severity::Error,
            self.code,
            format!("store '{}': {}", store.name, self.message),
        )
    }
}

impl From<StoreError> for PublishError {
    fn from(error: StoreError) -> Self {
        let code = match &error {
            StoreError::Refused { .. } => "publish.store.refused",
            StoreError::Incomplete { .. } => "publish.store.incomplete",
            StoreError::Unsupported { .. } => "publish.store.unsupported",
            StoreError::Io { .. } | StoreError::Transport { .. } | StoreError::Response { .. } => {
                "publish.store.failed"
            }
        };
        Self {
            message: error.to_string(),
            code,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        Credentials, Outcome, PublishError, channel_kind, stores_for, subdirs, target_for,
        token_for, token_from, uploads_of,
    };
    use crate::channel::Verified;
    use crate::project::Project;
    use std::cell::RefCell;
    use std::path::PathBuf;
    use std::rc::Rc;
    use universe_channel::store::TrustedPublisher;
    use universe_channel::store::{Kind, Policy, ReleaseReport, Report, StoreError, Upload};
    use universe_core::manifest::{Manifest, StoreAuth, StoreKind};

    fn store_of(kind: StoreKind) -> universe_core::manifest::Store {
        universe_core::manifest::Store {
            name: "test-store".to_owned(),
            kind,
            writable: true,
            needs_index: false,
            pages: false,
            read_auth: universe_core::manifest::ReadAuth::None,
            auth: universe_core::manifest::StoreAuth::Token,
            verifier: universe_core::manifest::Verifier::File,
            secrets: Vec::new(),
            environment: None,
            channel: Some("acme/acme".to_owned()),
            url: Some("http://127.0.0.1:8080".to_owned()),
            branch: None,
            budgets: universe_core::manifest::StoreBudgets::default(),
            tag_prefix: None,
            max_builds_per_version: None,
        }
    }

    fn project(stores: Vec<universe_core::manifest::Store>) -> Project {
        let manifest = Manifest {
            schema: 1,
            project: universe_core::manifest::Project {
                name: "fixture".to_owned(),
                description: None,
                repository: None,
                platforms: vec!["linux-64".to_owned()],
            },
            build: universe_core::manifest::Build::default(),
            audit: universe_core::manifest::Audit::default(),
            packages: Vec::new(),
            stores,
            release: None,
            airlock: None,
            workflow: universe_core::manifest::Workflow::default(),
        };
        Project {
            root: PathBuf::from("."),
            manifest,
        }
    }

    #[test]
    fn only_the_destinations_this_verb_writes_are_mapped() {
        for (kind, expected) in [
            (StoreKind::PrefixDev, Some(Kind::PrefixDev)),
            (StoreKind::Anaconda, Some(Kind::Anaconda)),
            (StoreKind::Quetz, Some(Kind::Quetz)),
            (StoreKind::Artifactory, Some(Kind::Artifactory)),
            (StoreKind::S3, Some(Kind::S3)),
            (StoreKind::Filesystem, Some(Kind::Filesystem)),
            (StoreKind::Releases, Some(Kind::Releases)),
            (StoreKind::Branch, None),
            (StoreKind::Static, None),
            (StoreKind::Pypi, None),
            (StoreKind::Command, None),
        ] {
            assert_eq!(channel_kind(kind), expected, "{}", kind.as_str());
        }
    }

    #[test]
    fn the_subdirs_of_a_fragment_set_are_listed_once_and_in_order() {
        let uploads = vec![
            Upload::new("osx-arm64/foo-1.0.0-0.conda", Vec::new()),
            Upload::new("linux-64/foo-1.0.0-0.conda", Vec::new()),
            Upload::new("osx-arm64/bar-1.0.0-0.conda", Vec::new()),
        ];

        assert_eq!(subdirs(&uploads), vec!["osx-arm64", "linux-64"]);
    }

    #[test]
    fn an_upload_takes_the_subdir_from_the_fragment_and_the_name_from_its_path() {
        let verified = vec![Verified {
            fragment: universe_channel::Fragment {
                package: "foo".to_owned(),
                version: "1.0.0".to_owned(),
                build: "0".to_owned(),
                build_number: 0,
                subdir: "linux-64".to_owned(),
                digest: String::new(),
                size_bytes: 0,
                depends: Vec::new(),
                license: None,
                license_files: Vec::new(),
                path: "output/foo-1.0.0-0.conda".to_owned(),
            },
            text: "{}".to_owned(),
            bytes: b"bytes".to_vec(),
        }];

        let uploads = uploads_of(&verified);

        assert_eq!(uploads.len(), 1);
        assert_eq!(uploads[0].path, "linux-64/foo-1.0.0-0.conda");
        assert_eq!(uploads[0].bytes, b"bytes");
    }

    #[test]
    fn a_protocol_endpoint_defaults_but_a_server_does_not() {
        let mut prefix = store_of(StoreKind::PrefixDev);
        prefix.url = None;
        assert_eq!(
            target_for(&prefix, Kind::PrefixDev, &mut Credentials::ambient())
                .expect("a default")
                .endpoint,
            "https://prefix.dev"
        );

        let mut releases = store_of(StoreKind::Releases);
        releases.url = None;
        assert_eq!(
            target_for(&releases, Kind::Releases, &mut Credentials::ambient())
                .expect("a default")
                .endpoint,
            "https://api.github.com"
        );

        let mut s3 = store_of(StoreKind::S3);
        s3.url = None;
        assert!(
            target_for(&s3, Kind::S3, &mut Credentials::ambient()).is_err(),
            "an object store has no default endpoint"
        );
    }

    #[test]
    fn a_token_is_read_from_the_variable_the_store_names() {
        let mut store = store_of(StoreKind::PrefixDev);
        store.secrets = vec!["UNIVERSE_TEST_STORE_TOKEN".to_owned()];

        assert!(token_for(&store).is_err(), "an unset variable is an error");
        assert!(
            token_from(&store, |_| Some("  ".to_owned())).is_err(),
            "a blank token is not a token"
        );

        store.secrets.clear();
        assert!(token_for(&store).expect("no token needed").is_none());
    }

    #[test]
    fn the_rendered_variable_name_is_read_before_the_bare_secret() {
        let mut store = store_of(StoreKind::PrefixDev);
        store.secrets = vec!["CONDA_TOKEN".to_owned()];

        // Exactly what a rendered job exports: the store name, then the secret.
        assert_eq!(
            store.secret_environment_variable("CONDA_TOKEN"),
            "TEST_STORE_CONDA_TOKEN"
        );
        let rendered_only = token_from(&store, |name| {
            (name == "TEST_STORE_CONDA_TOKEN").then(|| "from-the-job".to_owned())
        })
        .expect("the rendered name is read");
        assert_eq!(rendered_only.as_deref(), Some("from-the-job"));

        // A local shell sets the bare name, and that still works.
        let bare_only = token_from(&store, |name| {
            (name == "CONDA_TOKEN").then(|| "from-my-shell".to_owned())
        })
        .expect("the bare name is read too");
        assert_eq!(bare_only.as_deref(), Some("from-my-shell"));

        // When both are set the rendered one wins, because that is the one the job chose.
        let both = token_from(&store, |name| Some(format!("{name}-value"))).expect("both are set");
        assert_eq!(both.as_deref(), Some("TEST_STORE_CONDA_TOKEN-value"));
    }

    #[test]
    fn a_missing_token_names_both_spellings_it_looked_for() {
        let mut store = store_of(StoreKind::PrefixDev);
        store.secrets = vec!["CONDA_TOKEN".to_owned()];

        let error = token_from(&store, |_| None).expect_err("nothing is set");
        assert!(
            error.message.contains("TEST_STORE_CONDA_TOKEN"),
            "{}",
            error.message
        );
        assert!(error.message.contains("'CONDA_TOKEN'"), "{}", error.message);
    }

    #[test]
    fn a_named_store_must_exist_and_must_not_be_a_branch() {
        let fixture = project(vec![store_of(StoreKind::Branch)]);

        let error = stores_for(&fixture, Some("missing")).expect_err("an unknown name");
        assert_eq!(error.code(), "publish.store.unknown");

        let error = stores_for(&fixture, Some("test-store")).expect_err("a branch store");
        assert_eq!(error.code(), "publish.store.kind");
    }

    #[test]
    fn every_writable_destination_is_selected_and_the_rest_are_named_as_warnings() {
        let mut read_only = store_of(StoreKind::S3);
        read_only.name = "read-only".to_owned();
        read_only.writable = false;
        let mut pypi = store_of(StoreKind::Pypi);
        pypi.name = "pypi".to_owned();
        let fixture = project(vec![store_of(StoreKind::S3), read_only, pypi]);

        let (selected, warnings) = stores_for(&fixture, None).expect("a selection");

        assert_eq!(
            selected
                .iter()
                .map(|store| store.name.as_str())
                .collect::<Vec<_>>(),
            vec!["test-store"]
        );
        assert_eq!(warnings.len(), 1);
        assert_eq!(warnings[0].code(), "publish.store.unsupported");
    }

    #[test]
    fn a_store_error_keeps_the_code_that_says_what_went_wrong() {
        let cases = [
            (
                StoreError::Refused {
                    path: "foo.conda".to_owned(),
                    published: "a".to_owned(),
                    present: "b".to_owned(),
                },
                "publish.store.refused",
            ),
            (
                StoreError::Incomplete {
                    store: "s".to_owned(),
                    missing: "channel",
                },
                "publish.store.incomplete",
            ),
            (
                StoreError::Unsupported {
                    store: "s".to_owned(),
                    endpoint: "https://x".to_owned(),
                    reason: "no tls",
                },
                "publish.store.unsupported",
            ),
            (
                StoreError::Transport {
                    url: "u".to_owned(),
                    message: "m".to_owned(),
                },
                "publish.store.failed",
            ),
        ];

        for (error, code) in cases {
            assert_eq!(PublishError::from(error).code, code);
        }
    }

    #[test]
    fn an_upload_report_reads_as_what_happened_or_what_would() {
        let store = store_of(StoreKind::S3);
        let report = Report {
            uploaded: vec!["linux-64/foo-1.0.0-0.conda".to_owned()],
            unchanged: vec!["linux-64/bar-1.0.0-0.conda".to_owned()],
            bytes: 12,
            dry_run: false,
        };
        let outcome = Outcome::Uploaded(report);

        let human = outcome.describe(
            &store,
            Policy {
                force: false,
                dry_run: false,
            },
        );
        assert!(
            human.contains("uploaded 1 artifact(s), 12 byte(s)"),
            "{human}"
        );
        assert!(human.contains("(1 already there)"), "{human}");
        let planned = outcome.describe(
            &store,
            Policy {
                force: false,
                dry_run: true,
            },
        );
        assert!(planned.starts_with("would upload"), "{planned}");
        assert_eq!(outcome.json(&store, Policy::default())["kind"], "s3");
    }

    #[test]
    fn a_release_report_names_its_tag_and_what_it_pruned() {
        let store = store_of(StoreKind::Releases);
        let outcome = Outcome::Released(vec![ReleaseReport {
            tag: "packages-linux-64".to_owned(),
            created: true,
            uploaded: vec!["foo-1.0.0-0.conda".to_owned()],
            unchanged: Vec::new(),
            pruned: vec!["foo-1.0.0-0.conda".to_owned()],
            bytes: 3,
            dry_run: false,
        }]);

        let human = outcome.describe(&store, Policy::default());
        assert!(human.contains("release packages-linux-64"), "{human}");
        assert!(human.contains("pruned foo-1.0.0-0.conda"), "{human}");

        let released = Outcome::Released(vec![ReleaseReport {
            tag: "packages-noarch".to_owned(),
            ..ReleaseReport::default()
        }]);
        let human = released.describe(&store, Policy::default());
        assert!(human.contains("(0 already there)"), "{human}");

        let json = outcome.json(&store, Policy::default());
        assert_eq!(json["releases"][0]["tag"], "packages-linux-64");
        assert_eq!(json["store"], "test-store");
    }

    /// An exchange that remembers what it was asked and answers without a network.
    #[derive(Debug, Default, Clone)]
    struct RecordingExchange {
        asked: Rc<RefCell<Vec<String>>>,
    }

    impl TrustedPublisher for RecordingExchange {
        fn mint(&self, server: &str) -> Result<String, StoreError> {
            self.asked.borrow_mut().push(server.to_owned());
            Ok(format!("token-for-{server}"))
        }
    }

    #[test]
    fn an_oidc_store_publishes_with_the_token_its_identity_bought() {
        let mut store = store_of(StoreKind::PrefixDev);
        store.auth = StoreAuth::Oidc;
        store.secrets.clear();
        // An explicit endpoint, because the exchange is asked about the server it will
        // redeem at and a test's guess about the default is not the point.
        store.url = Some("https://prefix.dev".to_owned());
        let exchange = RecordingExchange::default();
        let mut credentials = Credentials::ready(exchange.clone());

        let target = target_for(&store, Kind::PrefixDev, &mut credentials).expect("a destination");

        assert_eq!(
            target.token.as_deref(),
            Some("token-for-https://prefix.dev")
        );
        assert_eq!(exchange.asked.borrow().len(), 1);
    }

    #[test]
    fn one_server_is_asked_once_however_many_stores_publish_to_it() {
        let mut first = store_of(StoreKind::PrefixDev);
        first.name = "first".to_owned();
        first.auth = StoreAuth::Oidc;
        first.secrets.clear();
        let mut second = first.clone();
        second.name = "second".to_owned();
        let exchange = RecordingExchange::default();
        let mut credentials = Credentials::ready(exchange.clone());

        let one = target_for(&first, Kind::PrefixDev, &mut credentials).expect("a destination");
        let two = target_for(&second, Kind::PrefixDev, &mut credentials).expect("a destination");

        // Two round trips per store would be two more than the job can afford, and the second
        // token would be identical anyway.
        assert_eq!(exchange.asked.borrow().len(), 1);
        assert_eq!(one.token, two.token);
    }

    #[test]
    fn an_oidc_store_that_is_not_a_prefix_dev_channel_is_refused_with_a_way_out() {
        let mut store = store_of(StoreKind::Anaconda);
        store.auth = StoreAuth::Oidc;
        let mut credentials = Credentials::ready(RecordingExchange::default());

        let error = target_for(&store, Kind::Anaconda, &mut credentials)
            .expect_err("no exchange is defined for this destination");

        let message = error.message;
        assert!(message.contains("prefix.dev channel"), "{message}");
        assert!(message.contains("auth = \"token\""), "{message}");
    }
}
