//! Deterministic manifest-to-workflow renderer for Universe.
//!
//! The renderer owns one transformation: `universe.toml` in, a workflow file set out.
//! It renders what *will* publish; it does not publish. That separation is why the first
//! release workflow can be generated instead of hand-written — a generator that cannot
//! exist before its own output does not apply here, because `cargo` builds this binary
//! inside the Pixi environment and needs no release pipeline to do it.
//!
//! Three properties make the output reviewable rather than merely produced:
//!
//! - **The file set is derived, never chosen.** Stores add the publish file, lock
//!   adapters add the mirror file, and nothing else adds anything. There is no mode flag.
//! - **Rendering is deterministic.** Same manifest, same bytes: no timestamps, no
//!   iteration over a hash map without ordering it first. A no-op re-render produces no
//!   diff, which is what makes `ci check` a meaningful gate instead of noise.
//! - **Privileges are checked before they are written.** A job holding two write scopes is
//!   refused, because the fix is two jobs rather than a wider permission block.
//!
//! The YAML shape lives in the template embedded by `universe-assets`; this module decides
//! *what* each job is, and the template decides *how* a job looks.

pub mod sha256;

use std::fmt;
use std::path::{Path, PathBuf};

use serde_json::{Value, json};
use universe_core::diagnostic::{Diagnostic, Severity};
use universe_core::manifest::{Manifest, PackageKind, ReadAuth, Store, StoreAuth, StoreKind};

/// Version of this component, equal to the workspace version the whole artifact set is
/// released under.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Where rendered files go unless the caller says otherwise.
pub const DEFAULT_OUTPUT_DIR: &str = ".github/workflows";

/// The file that exists for every project.
pub const MAIN_FILE: &str = "universe.yml";
/// The file that exists when the manifest declares a store.
///
/// Its name is the workflow filename the channel's trusted publisher was registered with;
/// GitHub puts that basename in `workflow_ref`, and prefix.dev matches it against the
/// publisher entry. A descriptive name that the registry does not know cannot mint a token.
pub const PUBLISH_FILE: &str = "release.yml";
/// The name the publish file had before the registered publisher was measured. A tag must
/// not start two publish workflows, so `ci render` removes it and `ci check` refuses it.
pub const LEGACY_PUBLISH_FILE: &str = "universe-publish.yml";
/// The file that exists when an Airlock scope declares a lock adapter.
pub const AIRLOCK_FILE: &str = "universe-airlock.yml";
/// Name of the artifact a build uploads and every later job downloads.
///
/// The matrix platform is part of the name, so two platform builds land as two artifacts and
/// a later job can ask for exactly the one it needs.
pub const ARTIFACT_NAME: &str = "universe-${{ matrix.platform }}";

/// Where a job keeps the artifacts it produces or consumes.
pub const ARTIFACT_PATH: &str = "output";

/// The file that exists when a package declares an upstream release source.
pub const WATCH_FILE: &str = "universe-watch.yml";

/// Every file name this renderer can produce, in the order it produces them.
pub const KNOWN_FILES: [&str; 4] = [MAIN_FILE, PUBLISH_FILE, AIRLOCK_FILE, WATCH_FILE];

/// Names the renderer once produced but no longer does. They stay known so a rename
/// removes the old file instead of leaving a second workflow behind.
pub const STALE_FILES: [&str; 1] = [LEGACY_PUBLISH_FILE];

/// One rendered file.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RenderedFile {
    /// File name, relative to the output directory.
    pub path: String,
    /// Complete file contents.
    pub content: String,
}

/// The rendered set plus the two derived views the renderer is required to print.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RenderOutput {
    /// Files, in a stable order.
    pub files: Vec<RenderedFile>,
    /// Checks a repository must require, in the order its settings page expects.
    pub required_checks: Vec<String>,
}

impl RenderOutput {
    /// The store→package mapping that produced the publish file.
    #[must_use]
    pub fn store_mapping(manifest: &Manifest) -> Vec<(String, Vec<String>)> {
        manifest
            .destinations()
            .into_iter()
            .map(|destination| {
                (
                    destination.store.name.clone(),
                    destination
                        .packages
                        .iter()
                        .map(|package| package.name.clone())
                        .collect(),
                )
            })
            .collect()
    }
}

/// Why a manifest could not be rendered.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RenderError {
    /// The template itself failed: a defect in this repository, not in the manifest.
    Template(String),
    /// A job declared more than one write scope.
    MultipleWriteScopes {
        /// Job whose declaration is wrong.
        job: String,
        /// The write scopes it declared.
        scopes: Vec<String>,
    },
    /// A file could not be read or written.
    Io {
        /// Path involved.
        path: String,
        /// Underlying message.
        message: String,
    },
    /// A known filename is occupied by a file this renderer did not write.
    ///
    /// `release.yml` is shared with every consumer's own tag-triggered lane, because prefix.dev
    /// matches a trusted publisher on the workflow basename. Two tools cannot both own it, and
    /// replacing the other one's file is data loss with no undo, so the run stops.
    Foreign {
        /// Path that belongs to somebody else.
        path: String,
    },
}

impl fmt::Display for RenderError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Template(message) => write!(formatter, "template failed: {message}"),
            Self::MultipleWriteScopes { job, scopes } => write!(
                formatter,
                "job '{job}' holds more than one write scope ({}); split it instead",
                scopes.join(", ")
            ),
            Self::Io { path, message } => write!(formatter, "{path}: {message}"),
            Self::Foreign { path } => write!(
                formatter,
                "{path} exists and carries no '{STAMP}' stamp, so it belongs to another tool \\
                 and is not overwritten; move that lane or name this manifest's publish file \\
                 differently"
            ),
        }
    }
}

impl std::error::Error for RenderError {}

/// One step of a rendered job.
#[derive(Clone, Debug, Eq, PartialEq)]
enum Step {
    /// A pinned action, with the inputs it needs.
    Uses {
        reference: String,
        inputs: Vec<(String, String)>,
    },
    /// A shell command.
    Run { label: String, command: String },
}

impl Step {
    /// A pinned action that takes no inputs.
    fn uses(reference: &str) -> Self {
        Self::Uses {
            reference: reference.to_owned(),
            inputs: Vec::new(),
        }
    }

    /// A pinned action with its `with:` block, in declaration order.
    fn uses_with(reference: &str, inputs: &[(&str, &str)]) -> Self {
        Self::Uses {
            reference: reference.to_owned(),
            inputs: inputs
                .iter()
                .map(|(name, value)| ((*name).to_owned(), (*value).to_owned()))
                .collect(),
        }
    }
}

/// One job of a rendered workflow.
#[derive(Clone, Debug, Eq, PartialEq)]
struct Job {
    id: String,
    name: String,
    /// The runner label. Every job in this repository's set runs on the same image; a
    /// per-job override is a manifest facet nothing has asked for yet.
    runs_on: String,
    needs: Vec<String>,
    always: bool,
    permissions: Vec<(String, String)>,
    matrix_key: &'static str,
    matrix: Vec<String>,
    fail_fast: bool,
    env: Vec<(String, String)>,
    /// GitHub environment the job runs in, when its trusted publisher expects one.
    ///
    /// GitHub adds an `environment` claim to the job's OIDC token only when the job declares
    /// an environment. A publisher registered with an environment refuses a token without
    /// it, so the manifest's store facet has to reach this field on exactly that store's job.
    environment: Option<String>,
    /// Ceiling for the whole job, in minutes.
    ///
    /// Without one, a stalled step holds a runner for GitHub's default six hours and the pull
    /// request reads `pending` rather than `failed`. A bound turns a hang into a failure that
    /// reports itself.
    timeout_minutes: u32,
    steps: Vec<Step>,
}

impl Job {
    fn new(id: &str, file_name: &str, needs: Vec<String>) -> Self {
        Self {
            id: to_identifier(id),
            name: format!("{file_name} / {id}"),
            runs_on: "ubuntu-latest".to_owned(),
            needs,
            always: false,
            permissions: Vec::new(),
            matrix_key: "platform",
            matrix: Vec::new(),
            fail_fast: true,
            env: Vec::new(),
            environment: None,
            timeout_minutes: 15,
            steps: Vec::new(),
        }
    }

    fn with_timeout(mut self, minutes: u32) -> Self {
        self.timeout_minutes = minutes;
        self
    }

    fn with_permissions(mut self, permissions: &[(&str, &str)]) -> Self {
        self.permissions = permissions
            .iter()
            .map(|(scope, access)| ((*scope).to_owned(), (*access).to_owned()))
            .collect();
        self
    }

    fn with_owned_permissions(mut self, permissions: Vec<(String, String)>) -> Self {
        self.permissions = permissions;
        self
    }

    fn with_matrix(mut self, key: &'static str, values: Vec<String>) -> Self {
        self.matrix_key = key;
        self.matrix = values;
        self
    }

    fn with_env(mut self, variable: &str, secret: &str) -> Self {
        self.env.push((variable.to_owned(), secret.to_owned()));
        self
    }

    fn with_environment(mut self, environment: &str) -> Self {
        self.environment = Some(environment.to_owned());
        self
    }

    fn with_step(mut self, step: Step) -> Self {
        self.steps.push(step);
        self
    }

    fn verdict(mut self) -> Self {
        self.always = true;
        self.with_step(Step::Run {
            label: "Fail when any required job did not succeed".to_owned(),
            command: "echo \"$RESULTS\" | jq -e 'all(.[]; .result == \"success\")' > /dev/null"
                .to_owned(),
        })
        .with_env("RESULTS", "")
    }

    /// The write scopes this job holds, excluding `id-token`, which is an exchange rather
    /// than a resource.
    fn write_scopes(&self) -> Vec<String> {
        self.permissions
            .iter()
            .filter(|(scope, access)| access == "write" && scope != "id-token")
            .map(|(scope, _)| scope.clone())
            .collect()
    }

    fn to_json(&self, includes_results: bool) -> Value {
        let steps: Vec<Value> = self
            .steps
            .iter()
            .map(|step| match step {
                Step::Uses { reference, inputs } => json!({
                    "uses": reference,
                    "inputs": inputs
                        .iter()
                        .map(|(name, value)| json!({ "name": name, "value": value }))
                        .collect::<Vec<_>>(),
                }),
                Step::Run { label, command } => json!({
                    "uses": Value::Null,
                    "label": label,
                    "run": command,
                }),
            })
            .collect();
        let permissions: Vec<Value> = self
            .permissions
            .iter()
            .map(|(scope, access)| json!({ "scope": scope, "access": access }))
            .collect();
        let env: Vec<Value> = self
            .env
            .iter()
            .map(|(name, secret)| {
                if includes_results && name == "RESULTS" {
                    json!({ "name": name, "secret": Value::Null, "expression": "${{ toJSON(needs) }}" })
                } else {
                    json!({ "name": name, "secret": secret, "expression": Value::Null })
                }
            })
            .collect();
        json!({
            "id": self.id,
            "name": self.name,
            "runs_on": self.runs_on,
            "timeout_minutes": self.timeout_minutes,
            "needs": self.needs,
            "always": self.always,
            "permissions": permissions,
            "matrix_key": self.matrix_key,
            "matrix": self.matrix,
            "fail_fast": if self.fail_fast { "true" } else { "false" },
            "env": env,
            "environment": self.environment.as_deref(),
            "steps": steps,
        })
    }
}

/// A trigger and its filters.
fn trigger(event: &str, filters: &[(&str, Vec<String>)]) -> Value {
    let filters: Vec<Value> = filters
        .iter()
        .map(|(key, values)| {
            let quoted: Vec<String> = values.iter().map(|value| format!("\"{value}\"")).collect();
            json!({ "key": key, "values": quoted })
        })
        .collect();
    json!({ "event": event, "filters": filters })
}

/// Turn a manifest value into a YAML-safe job identifier.
fn to_identifier(text: &str) -> String {
    text.chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() {
                character.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect()
}

/// Put the checkout and the Pixi setup in front of whatever the job already declares.
///
/// Order is the difference between a job that runs and one that does not: a step needs the
/// work tree before it can read the manifest, and needs Pixi before it can run a task. The
/// two are prepended rather than appended so the call site reads in execution order.
fn checkout_and_pixi(job: Job) -> Job {
    let mut steps = vec![
        // The token is not written into `.git/config` for the rest of the job. Every step
        // after a checkout can run project code, and a persisted credential is what lets that
        // code push with the job's identity. A job that must push — the mirror, the lock
        // repair — sets its own token on the step that pushes, which is the narrower grant.
        Step::uses_with(
            universe_assets::CHECKOUT,
            &[("persist-credentials", "false")],
        ),
        // The version is named rather than left to the action: unpinned, every job waits on a
        // GitHub API lookup for the newest Pixi, and that lookup failing fails the job before
        // the repository runs anything. The lockfile is enforced by the `--frozen` the rendered
        // commands already carry, so it is not repeated here.
        Step::uses_with(
            universe_assets::SETUP_PIXI,
            &[
                ("pixi-version", universe_assets::PIXI_VERSION),
                ("cache", "true"),
            ],
        ),
        // Every job compiles the same workspace, so they share one cache rather than each
        // paying the cold build again. `shared-key` is what makes the second job a restore
        // instead of a rebuild; the action's default key already folds in `Cargo.lock` and the
        // toolchain, so a dependency change still misses.
        Step::uses_with(
            universe_assets::RUST_CACHE,
            &[("shared-key", "universe"), ("cache-on-failure", "false")],
        ),
    ];
    steps.extend(job.steps);
    Job { steps, ..job }
}

/// The first line of every file this renderer writes.
///
/// It is the only thing that distinguishes a workflow `ci render` is allowed to delete or
/// replace from one that merely shares a filename with it. `release.yml` is not a name this
/// project can claim: prefix.dev matches a trusted publisher on the workflow basename, so every
/// consumer publishing that way owns a `release.yml` of its own. Content decides ownership, and
/// the stamp is the content.
pub const STAMP: &str = "# @generated by universe";

/// Whether `text` is a file this renderer wrote.
#[must_use]
fn carries_stamp(text: &str) -> bool {
    text.starts_with(STAMP)
}

/// Whether the file at `path` is one this renderer wrote.
///
/// An unreadable file is treated as somebody else's: this renderer writes UTF-8 text, so a file
/// it cannot read as text is not one it produced, and guessing otherwise is how the file that
/// cannot be read becomes the file that gets deleted.
fn is_ours(path: &Path) -> bool {
    std::fs::read_to_string(path).is_ok_and(|text| carries_stamp(&text))
}

/// The file names this manifest calls for, in render order.
#[must_use]
pub fn derived_files(manifest: &Manifest) -> Vec<&'static str> {
    // `universe.yml` is the package lane, and a manifest with no `[[packages]]` has said it has
    // no package lane. Rendering one anyway gives that repository a build job whose upload finds
    // nothing — an always-red required check, plus a `push.tags` trigger competing with whatever
    // already publishes its tags (GitHub issue 122).
    let mut files = Vec::new();
    if !manifest.packages.is_empty() {
        files.push(MAIN_FILE);
    }
    if manifest.stores.iter().any(|store| store.writable) {
        files.push(PUBLISH_FILE);
    }
    if !manifest.lock_adapters().is_empty() {
        files.push(AIRLOCK_FILE);
    }
    files
}

/// Which privilege a store's publication needs.
///
/// One resource write per job (`decision-128` §2). A hosted registry takes
/// `packages: write`; a repository-backed destination takes `contents: write`; a bucket or
/// directory takes neither, because neither is this repository.
const fn store_write_scope(kind: StoreKind) -> Option<&'static str> {
    match kind {
        StoreKind::Releases | StoreKind::Branch => Some("contents"),
        StoreKind::Static
        | StoreKind::PrefixDev
        | StoreKind::Anaconda
        | StoreKind::Quetz
        | StoreKind::Npmjs
        | StoreKind::GithubPackages
        | StoreKind::Pypi
        | StoreKind::CratesIo
        | StoreKind::Artifactory => Some("packages"),
        StoreKind::S3 | StoreKind::Filesystem | StoreKind::Command => None,
    }
}

/// The environment variable a store's named secret is exported as.
///
/// The rule lives on [`Store`] so the renderer and the verbs that read the variable cannot
/// disagree about how it is spelt — they did, and a tag run would have exported a name
/// nothing looked up.
fn secret_variable(store: &Store, secret: &str) -> String {
    store.secret_environment_variable(secret)
}

/// The verb that publishes to a store of `kind`, if one exists.
///
/// The split is the transport's, not the ecosystem's: a branch is replaced wholesale, so one
/// verb pushes a tree to it, while a registry keeps what it was first given, so another sends
/// one file at a time and compares digests first. Rendering the wrong one is not a typo the
/// workflow reports — it fails on a tag, hours after the change that caused it.
const fn publish_verb(kind: StoreKind) -> Option<&'static str> {
    // Which kinds publish is a fact about the producer lane, so it is stated once — in
    // `StoreKind::publishes`, where manifest validation reads it too. A kind that joins the
    // unwritable set is then refused before a manifest can reach this function, and the two
    // cannot disagree about which ones those are.
    if !kind.publishes() {
        return None;
    }
    match kind {
        StoreKind::Branch | StoreKind::Static => Some("publish"),
        // Every remaining writable kind uploads one artifact at a time.
        _ => Some("publish-artifacts"),
    }
}

/// Whether a rendered artifact upload asks its destination to overwrite a changed digest.
///
/// The two kinds this returns are the `v0.1.0` destinations that keep bytes at a filename a
/// rebuild can occupy again. A dispatch of a non-reproducible build then changes the digest
/// without changing the name; the registry and the rolling release both refuse that upload
/// unless `--force` is rendered into their command. This is a deliberate reversal of the
/// no-overwrite release semantics for the pre-release publish lane, not a fix for the
/// reproducibility defect and not a general licence to rewrite every destination. The branch
/// verb already replaces one snapshot tree and takes no overwrite flag.
const fn overwrites_changed_digest(kind: StoreKind) -> bool {
    matches!(kind, StoreKind::PrefixDev | StoreKind::Releases)
}

/// The directory a Pages deployment is generated into, relative to the runner's root.
///
/// It is not `output`: the artifacts the build uploaded already live there, and Pages would then
/// serve the whole build tree — receipts, fragments, and every platform's archives — instead of
/// the channel a solver reads.
pub const PAGES_OUTPUT: &str = "output/pages";

/// The GitHub environment a Pages deployment records itself in when the store names none.
///
/// The environment is not decoration: GitHub derives the deployment URL from it, and a
/// deployment that declares none is refused by the Pages API.
pub const PAGES_ENVIRONMENT: &str = "github-pages";

/// The remote a rendered branch publish pushes to.
///
/// The manifest names the store, not its URL: for a branch store the remote *is* the
/// repository the workflow runs in, so the expression resolves at run time rather than
/// becoming a second place the owner's hosting is written down.
const BRANCH_REMOTE: &str = "${{ github.server_url }}/${{ github.repository }}.git";

/// The command line a rendered job uses to reach a Universe verb.
///
/// A runner that has just run `setup-pixi` has `pixi` on its `PATH` and nothing else:
/// `universe-kit` is not published in v0.1.0 (`decision-132`), so a bare `universe` verb
/// would be a command the job cannot find. The Pixi task builds the workspace and runs the
/// facade it just built, which is also how a developer dogfoods the same binary.
fn verb(arguments: &str) -> String {
    format!("pixi run --frozen universe -- {arguments}")
}

/// The job that produces `output/` and hands it to every later job.
///
/// A job's filesystem does not outlive it, so a build hands its bytes on as a workflow
/// artifact and a consumer downloads them. It lives in one place because two files now need
/// it: the main file audits and verifies what it builds, and the publish file publishes it.
///
/// `workflow` is the file's name in the rendered header, so the same job reads `universe /
/// build` in one file and `publish / build` in the other.
///
/// This is also why the publish file builds instead of downloading the tag's artifacts from a
/// different workflow run: `actions/download-artifact` only sees its own run, and a release
/// that depends on another run's retention is a release that can fail for a reason nobody
/// changed. A rebuild costs minutes on the one run where being self-contained matters.
///
/// `emit_fragments` is the difference between the two files. The main file audits in its own
/// job and keeps the findings there; the publish file has no audit job, and its publishers
/// read `output/fragments` from the artifact they download. Writing the fragments in the
/// build, before the upload, is the only way they reach those jobs — the `v0.1.0-rc1`
/// rehearsal failed both publishes with `channel.fragment.none` because receipts travelled
/// and fragments did not.
fn build_job(manifest: &Manifest, workflow: &str, emit_fragments: bool) -> Job {
    let mut job = Job::new("build", workflow, Vec::new())
        .with_timeout(25)
        .with_matrix("platform", manifest.project.platforms.clone())
        .with_step(Step::Run {
            label: "Build the declared packages".to_owned(),
            command: verb(&format!(
                "build --json --receipts {ARTIFACT_PATH}/.receipts"
            )),
        });
    if emit_fragments {
        job = job.with_step(Step::Run {
            label: "Audit the built artifacts".to_owned(),
            command: verb(&format!(
                "audit --json --emit-fragment {ARTIFACT_PATH}/fragments"
            )),
        });
    }
    checkout_and_pixi(job.with_step(Step::uses_with(
        universe_assets::UPLOAD_ARTIFACT,
        &[
            ("name", ARTIFACT_NAME),
            ("path", ARTIFACT_PATH),
            ("if-no-files-found", "error"),
        ],
    )))
}

fn main_jobs(manifest: &Manifest) -> Vec<Job> {
    let checks = manifest.workflow.checks.declared();
    let mut view = Job::new("view", "universe", Vec::new())
        .with_timeout(10)
        .with_step(Step::Run {
            label: "View the manifest".to_owned(),
            command: verb("plan --json"),
        });
    view = checkout_and_pixi(view);

    // No `needs: [view]`: linting the manifest and formatting the tree do not need the plan,
    // so waiting for it only puts a second job's runtime on the critical path. Every job in
    // the file is independent work; the verdict job is what gathers the results.
    let mut validate = Job::new("validate", "universe", Vec::new())
        .with_timeout(15)
        .with_step(Step::Run {
            label: "Validate the manifest".to_owned(),
            command: verb("lint --json"),
        });
    for (slot, command) in &checks {
        validate = validate.with_step(Step::Run {
            label: format!("Check {slot}"),
            command: (*command).to_owned(),
        });
    }
    validate = checkout_and_pixi(validate);

    let build = build_job(manifest, "universe", false);

    let audit = checkout_and_pixi(
        Job::new("audit", "universe", vec!["build".to_owned()])
            .with_timeout(20)
            .with_step(Step::uses_with(
                universe_assets::DOWNLOAD_ARTIFACT,
                &[
                    ("pattern", "universe-*"),
                    ("merge-multiple", "true"),
                    ("path", ARTIFACT_PATH),
                ],
            ))
            .with_step(Step::Run {
                label: "Audit the built artifacts".to_owned(),
                command: verb(&format!(
                    "audit --json --emit-fragment {ARTIFACT_PATH}/fragments"
                )),
            }),
    );

    let mut jobs = vec![view, validate, build, audit];

    if manifest.workflow.verify_artifact {
        jobs.push(checkout_and_pixi(
            Job::new("verify-artifact", "universe", vec!["audit".to_owned()])
                .with_timeout(20)
                .with_matrix("platform", manifest.project.platforms.clone())
                .with_step(Step::uses_with(
                    universe_assets::DOWNLOAD_ARTIFACT,
                    &[("name", ARTIFACT_NAME), ("path", ARTIFACT_PATH)],
                ))
                .with_step(Step::Run {
                    label: "Prove the built artifact installs and runs".to_owned(),
                    command: verb("verify --artifact --json"),
                }),
        ));
    }

    let needs: Vec<String> = jobs.iter().map(|job| job.id.clone()).collect();
    let verdict = Job::new("verdict", "universe", needs)
        .with_timeout(5)
        .verdict();
    jobs.push(verdict);
    jobs
}

fn publish_jobs(manifest: &Manifest) -> Vec<Job> {
    // The publish file has its own build. `actions/download-artifact` reads the artifacts of
    // *its own* run, so a publish job that downloads `universe-*` while this workflow has no
    // build of its own finds nothing — which is what the first tag would have done, silently:
    // a green pipeline that published no artifact.
    let mut jobs = vec![build_job(manifest, "publish", true)];
    let mut publishes_anonymously = false;

    for store in manifest.stores.iter().filter(|store| store.writable) {
        // A `releases` store is still declared (it carries the repository and token), but
        // `Manifest::destinations` fills its package list from `release = true` rather than
        // from `publish`, so the job below publishes even though no package names the store.
        let mut job = Job::new(&store.name, "publish", vec!["build".to_owned()]).with_timeout(30);
        if let Some(environment) = &store.environment {
            job = job.with_environment(environment);
        }
        if let Some(scope) = store_write_scope(store.kind) {
            job = job.with_permissions(&[(scope, "write")]);
        }
        if store.auth == StoreAuth::Oidc {
            // An OIDC exchange grants no repository content on its own, so it accompanies
            // whatever resource scope the destination already needed.
            let mut permissions = job.permissions.clone();
            permissions.push(("id-token".to_owned(), "write".to_owned()));
            job = job.with_owned_permissions(permissions);
        }
        for secret in &store.secrets {
            job = job.with_env(&secret_variable(store, secret), secret);
        }
        job = checkout_and_pixi(job).with_step(Step::uses_with(
            universe_assets::DOWNLOAD_ARTIFACT,
            &[
                ("pattern", "universe-*"),
                ("merge-multiple", "true"),
                ("path", ARTIFACT_PATH),
            ],
        ));
        let Some(publish) = publish_verb(store.kind) else {
            continue;
        };
        // A branch store is pushed to, so the job has to say which remote and which branch;
        // a registry store is uploaded to, and names nothing beyond itself.
        let arguments = if publish == "publish" {
            // The ref is the destination's, so it is emitted when the store declares one and
            // omitted when it does not: an undeclared ref keeps the tool's default, and this
            // repository declares `universe/conda-channel` because `channel` was its old name.
            match store.branch.as_deref() {
                Some(branch) => format!(
                    "publish --store {} --remote {BRANCH_REMOTE} --branch {branch}",
                    store.name
                ),
                None => format!("publish --store {} --remote {BRANCH_REMOTE}", store.name),
            }
        } else if overwrites_changed_digest(store.kind) {
            format!("publish-artifacts --store {} --force", store.name)
        } else {
            format!("publish-artifacts --store {}", store.name)
        };
        // The release body is a file in the tree, so the job names the path the manifest names
        // and the destination reads what the pull request reviewed.
        let arguments = match store.notes.as_deref() {
            Some(notes) => format!("{arguments} --notes-file {notes}"),
            None => arguments,
        };
        let packages = manifest
            .destinations()
            .into_iter()
            .find(|destination| destination.store.name == store.name)
            .map(|destination| destination.packages)
            .unwrap_or_default();
        if publish == "publish" {
            // `publish` force-pushes one orphan snapshot tree. Three pushes of one
            // package each leave only the last tree on the branch — the widening to
            // three packages made run 43045274207 do exactly that. One push names
            // every package, so the tree the branch serves holds all of them.
            if !packages.is_empty() {
                let filters = packages
                    .iter()
                    .map(|package| format!(" --package {}", package.name))
                    .collect::<String>();
                job = job.with_step(Step::Run {
                    label: "Publish the channel snapshot".to_owned(),
                    command: verb(&format!("{arguments}{filters} --json")),
                });
            }
        } else {
            // An artifact store uploads one file per call; each step is one independent
            // upload whose digest the destination checks on its own.
            for package in packages {
                job = job.with_step(Step::Run {
                    label: format!("Publish {}", package.name),
                    command: verb(&format!("{arguments} --package {} --json", package.name)),
                });
            }
        }
        // A credential-free destination is verified by a separate reader; a credentialed
        // one is verified here, because a second job would present the same token.
        if store.read_auth == ReadAuth::Token {
            job = job.with_step(Step::Run {
                label: format!("Verify {}", store.name),
                command: verb(&format!("verify --published --store {} --json", store.name)),
            });
        } else {
            publishes_anonymously = true;
        }
        jobs.push(job);
    }

    if publishes_anonymously && manifest.workflow.verify_published {
        let needs: Vec<String> = jobs.iter().map(|job| job.id.clone()).collect();
        jobs.push(checkout_and_pixi(
            Job::new("verify-index", "publish", needs).with_step(Step::Run {
                label: "Re-read every anonymously readable destination".to_owned(),
                command: verb("verify --index --json"),
            }),
        ));
    }

    // The Pages deployment comes after the stores: what it serves is the index of what they
    // accepted, so a deployment that ran first would publish a tree describing artifacts the
    // release had not yet published.
    jobs.extend(pages_jobs(manifest));

    let needs: Vec<String> = jobs.iter().map(|job| job.id.clone()).collect();
    jobs.push(Job::new("verdict", "publish", needs).verdict());
    jobs
}

/// The GitHub Pages deployment of the channel index, when a store asks for one.
///
/// Two jobs, because the actions need different privileges and `decision-128` allows one write
/// scope per job: `pages` renders the tree and uploads it as the Pages artifact, and
/// `pages-deploy` publishes what was uploaded. The split is also what the actions themselves
/// document, and it means a rendering failure never reaches a deployment.
///
/// The tree is *generated here* rather than committed: a second copy of the channel in the
/// repository would be a copy that can drift from the one the stores hold, and the point of the
/// site is to serve exactly what a release published.
fn pages_jobs(manifest: &Manifest) -> Vec<Job> {
    let mut jobs = Vec::new();
    for store in manifest.stores.iter().filter(|store| store.pages) {
        let url = store.url.as_deref().unwrap_or_default();
        // `configure-pages` before the index is generated, not after: its base-path output is
        // what a project page served under a subpath has to be told, and a failure there belongs
        // before the work rather than after it.
        let generate = checkout_and_pixi(
            Job::new(&store.name, "publish", vec!["build".to_owned()])
                .with_timeout(20)
                .with_permissions(&[("pages", "write"), ("id-token", "write")])
                .with_step(Step::uses(universe_assets::CONFIGURE_PAGES))
                .with_step(Step::uses_with(
                    universe_assets::DOWNLOAD_ARTIFACT,
                    &[
                        ("pattern", "universe-*"),
                        ("merge-multiple", "true"),
                        ("path", ARTIFACT_PATH),
                    ],
                ))
                .with_step(Step::Run {
                    label: "Render the channel the site serves".to_owned(),
                    command: verb(&format!(
                        "index --output {PAGES_OUTPUT} --base-url {url} --json"
                    )),
                })
                .with_step(Step::uses_with(
                    universe_assets::UPLOAD_PAGES_ARTIFACT,
                    &[("path", PAGES_OUTPUT)],
                )),
        );
        let environment = store
            .environment
            .as_deref()
            .unwrap_or(PAGES_ENVIRONMENT)
            .to_owned();
        let deploy = Job::new(
            &format!("{}-deploy", store.name),
            "publish",
            vec![store.name.clone()],
        )
        .with_timeout(10)
        .with_environment(&environment)
        // A Pages deployment authenticates with the run's own identity, so it holds an
        // `id-token: write` exchange and the `pages` resource write — and no repository
        // content, which is why it needs no checkout.
        .with_permissions(&[("pages", "write"), ("id-token", "write")])
        .with_step(Step::uses(universe_assets::DEPLOY_PAGES));
        jobs.push(generate);
        jobs.push(deploy);
    }
    jobs
}

fn airlock_jobs(manifest: &Manifest) -> Vec<Job> {
    let adapters: Vec<String> = manifest
        .lock_adapters()
        .iter()
        .map(|adapter| adapter.as_str().to_owned())
        .collect();
    // Both jobs push to the branch store, and neither inherits a credential: the checkout is
    // rendered with `persist-credentials: false`, so the token each push needs is the one the
    // store declares, exported on the job.
    let pushing = manifest
        .stores
        .iter()
        .find(|store| store.writable && store.kind.is_git_branch());
    let credentialised = |mut job: Job| {
        if let Some(store) = pushing {
            for secret in &store.secrets {
                job = job.with_env(&secret_variable(store, secret), secret);
            }
        }
        job
    };
    let mirror = checkout_and_pixi(credentialised(
        Job::new("mirror", "airlock", Vec::new())
            .with_permissions(&[("contents", "write")])
            .with_matrix("adapter", adapters.clone())
            .with_step(Step::Run {
                label: "Mirror one lock format".to_owned(),
                command: verb("airlock mirror --adapter ${{ matrix.adapter }} --json"),
            }),
    ));
    let mut mirror = mirror;
    mirror.fail_fast = false;
    let render = checkout_and_pixi(credentialised(
        Job::new("render-airlock", "airlock", vec!["mirror".to_owned()])
            .with_permissions(&[("contents", "write")])
            .with_step(Step::Run {
                label: "Render the Airlock restore entrypoints".to_owned(),
                command: verb("airlock render --json"),
            }),
    ));
    let verdict = Job::new(
        "verdict",
        "airlock",
        vec!["mirror".to_owned(), "render-airlock".to_owned()],
    )
    .verdict();
    vec![mirror, render, verdict]
}

fn triggers_for(file: &str, manifest: &Manifest) -> Vec<Value> {
    let tag_pattern = manifest
        .release
        .as_ref()
        .map_or_else(|| "v*".to_owned(), |release| release.tag_pattern.clone());
    match file {
        MAIN_FILE => vec![
            trigger("pull_request", &[]),
            trigger(
                "push",
                &[
                    ("branches", vec!["main".to_owned()]),
                    ("tags", vec![tag_pattern]),
                ],
            ),
            trigger("workflow_dispatch", &[]),
        ],
        PUBLISH_FILE => vec![
            trigger("push", &[("tags", vec![tag_pattern])]),
            trigger("workflow_dispatch", &[]),
        ],
        AIRLOCK_FILE => vec![
            trigger(
                "push",
                &[
                    ("branches", vec!["main".to_owned()]),
                    (
                        "paths",
                        vec![
                            "universe.toml".to_owned(),
                            "pixi.lock".to_owned(),
                            "Cargo.lock".to_owned(),
                            "bun.lock".to_owned(),
                        ],
                    ),
                ],
            ),
            trigger("workflow_dispatch", &[]),
        ],
        _ => vec![trigger("workflow_dispatch", &[])],
    }
}

/// Render the workflow set a manifest calls for.
///
/// # Errors
///
/// Returns [`RenderError::MultipleWriteScopes`] when a job would hold two resource write
/// scopes, and [`RenderError::Template`] when the template cannot be rendered.
pub fn render(manifest: &Manifest, manifest_text: &str) -> Result<RenderOutput, RenderError> {
    let mut environment = minijinja::Environment::new();
    // Only the *variable* delimiters move. `${{ … }}` is a GitHub expression that must
    // survive rendering verbatim, so it cannot also be the template's own syntax; the
    // block tags stay at `{% %}` because nothing in a workflow collides with them.
    let syntax = minijinja::syntax::SyntaxConfig::builder()
        .variable_delimiters("<<", ">>")
        .build()
        .map_err(|error| RenderError::Template(error.to_string()))?;
    environment.set_syntax(syntax);
    environment.set_auto_escape_callback(|_| minijinja::AutoEscape::None);
    environment
        .add_template("workflow.yml.j2", universe_assets::WORKFLOW_TEMPLATE)
        .map_err(|error| RenderError::Template(error.to_string()))?;
    let template = environment
        .get_template("workflow.yml.j2")
        .map_err(|error| RenderError::Template(error.to_string()))?;

    let manifest_digest = sha256::sha256_hex(manifest_text.as_bytes());
    let mut files = Vec::new();
    let mut required_checks = Vec::new();

    for file in derived_files(manifest) {
        let jobs = match file {
            MAIN_FILE => main_jobs(manifest),
            PUBLISH_FILE => publish_jobs(manifest),
            AIRLOCK_FILE => airlock_jobs(manifest),
            other => return Err(RenderError::Template(format!("no job plan for {other}"))),
        };

        for job in &jobs {
            let scopes = job.write_scopes();
            if scopes.len() > 1 {
                return Err(RenderError::MultipleWriteScopes {
                    job: job.name.clone(),
                    scopes,
                });
            }
        }

        let triggers = triggers_for(file, manifest);
        let triggers_pull_request = triggers.iter().any(|trigger| {
            trigger
                .get("event")
                .and_then(Value::as_str)
                .is_some_and(|event| event == "pull_request")
        });
        if triggers_pull_request {
            required_checks.extend(jobs.iter().map(|job| job.name.clone()));
        }

        let mut rendered = template
            .render(minijinja::value::Serde(json!({
                "generator": VERSION,
                "manifest_digest": manifest_digest,
                "name": file.trim_end_matches(".yml").replace('-', " "),
                "triggers": triggers,
                "jobs": jobs
                    .iter()
                    .map(|job| job.to_json(job.always))
                    .collect::<Vec<_>>(),
            })))
            .map_err(|error| RenderError::Template(error.to_string()))?;
        // The template's last line has no newline of its own; a text file that does not
        // end with one shows up as a spurious final-line diff in every hand-edit.
        if !rendered.ends_with('\n') {
            rendered.push('\n');
        }

        files.push(RenderedFile {
            path: file.to_owned(),
            content: rendered,
        });
    }

    Ok(RenderOutput {
        files,
        required_checks,
    })
}

/// What one render did to a directory.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Written {
    /// The files this run wrote, in render order.
    pub written: Vec<PathBuf>,
    /// Files at known names that were left alone because they carry no stamp of this renderer.
    ///
    /// Not an error: another tool's workflow at a filename this renderer also uses is the
    /// normal case for `release.yml`. The caller says so, because silence here is what made the
    /// deletion surprising in the first place.
    pub untouched: Vec<PathBuf>,
}

/// Write a rendered set into `directory`, creating it when absent.
///
/// # Errors
///
/// Returns [`RenderError::Io`] when the directory or a file cannot be written, and
/// [`RenderError::Foreign`] when a file this renderer did not write occupies a name the
/// manifest renders. The check runs before anything is written, so a refusal leaves the tree
/// exactly as it was.
pub fn write(directory: &Path, output: &RenderOutput) -> Result<Written, RenderError> {
    std::fs::create_dir_all(directory).map_err(|error| RenderError::Io {
        path: directory.display().to_string(),
        message: error.to_string(),
    })?;
    // Refuse before writing anything: a run that stops halfway leaves the caller with a tree
    // that is neither the old render nor the new one.
    for file in &output.files {
        let path = directory.join(&file.path);
        if path.exists() && !is_ours(&path) {
            return Err(RenderError::Foreign {
                path: path.display().to_string(),
            });
        }
    }
    let mut written = Vec::new();
    for file in &output.files {
        let path = directory.join(&file.path);
        std::fs::write(&path, &file.content).map_err(|error| RenderError::Io {
            path: path.display().to_string(),
            message: error.to_string(),
        })?;
        written.push(path);
    }
    // A rename is not complete until the old name is gone: two tag-triggered workflows
    // would both publish, and only one of them can match a trusted publisher entry. Only a
    // file carrying the stamp is a rename, though — an unstamped one is another project's
    // workflow that happens to share the name, and removing it is not this tool's business.
    let declared: Vec<&str> = output.files.iter().map(|file| file.path.as_str()).collect();
    let mut untouched = Vec::new();
    for name in KNOWN_FILES.iter().chain(STALE_FILES.iter()) {
        if declared.contains(name) {
            continue;
        }
        let path = directory.join(name);
        if !path.exists() {
            continue;
        }
        if !is_ours(&path) {
            untouched.push(path);
            continue;
        }
        std::fs::remove_file(&path).map_err(|error| RenderError::Io {
            path: path.display().to_string(),
            message: error.to_string(),
        })?;
    }
    Ok(Written { written, untouched })
}

/// Compare a rendered set against the files on disk.
///
/// A missing file and a drifted file are both failures, and so is a known file that this
/// manifest no longer calls for: the last one is a capability that was removed without
/// regenerating, which would leave a workflow that publishes to nothing.
#[must_use]
pub fn check(directory: &Path, output: &RenderOutput) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    for file in &output.files {
        let path = directory.join(&file.path);
        match std::fs::read_to_string(&path) {
            Ok(existing) if existing == file.content => {}
            // A stamped file at a rendered name is ours and drifted; an unstamped one is
            // somebody else's and would be refused by `write`, so the remedy is to move it.
            Ok(existing) if carries_stamp(&existing) => diagnostics.push(
                Diagnostic::new(
                    Severity::Error,
                    "ci.file.drift",
                    format!("{} no longer matches the manifest", file.path),
                )
                .with_path(path.display().to_string()),
            ),
            Ok(_) => diagnostics.push(
                Diagnostic::new(
                    Severity::Error,
                    "ci.file.foreign",
                    format!(
                        "{} exists, carries no '{STAMP}' stamp, and belongs to another tool; \
                         `ci render` refuses to overwrite it, so move that lane or reconcile \
                         the two",
                        file.path
                    ),
                )
                .with_path(path.display().to_string()),
            ),
            Err(_) => diagnostics.push(
                Diagnostic::new(
                    Severity::Error,
                    "ci.file.missing",
                    format!("{} was never rendered", file.path),
                )
                .with_path(path.display().to_string()),
            ),
        }
    }
    let declared: Vec<&str> = output.files.iter().map(|file| file.path.as_str()).collect();
    for name in KNOWN_FILES.iter().chain(STALE_FILES.iter()) {
        if declared.contains(name) {
            continue;
        }
        let path = directory.join(name);
        // Only a file this renderer wrote is a leftover: it is the thing a rename leaves
        // behind, or a capability the manifest dropped without regenerating. A foreign file
        // at the same name is another project's workflow and says nothing about this one.
        if path.exists() && is_ours(&path) {
            diagnostics.push(
                Diagnostic::new(
                    Severity::Error,
                    "ci.file.undeclared",
                    format!(
                        "{name} was rendered by this tool but this manifest calls for no such \
                         file; run `universe ci render` to remove it"
                    ),
                )
                .with_path(path.display().to_string()),
            );
        }
    }
    diagnostics
}

/// Whether a package's kind has a documented static surface, which is what decides
/// whether a `static` store may be listed for it.
#[must_use]
pub fn has_static_surface(kind: PackageKind) -> bool {
    StoreKind::Static.accepts(kind)
}

#[cfg(test)]
mod tests {
    use super::{
        AIRLOCK_FILE, ARTIFACT_PATH, LEGACY_PUBLISH_FILE, MAIN_FILE, PUBLISH_FILE, RenderError,
        VERSION, derived_files, publish_verb, render, sha256, to_identifier,
    };
    use universe_core::manifest::{Manifest, Store, StoreKind};

    const MANIFEST: &str = r#"
schema = 1

[project]
name = "example"
platforms = ["linux-64", "noarch"]

[[packages]]
name = "example-airlock"
kind = "conda"
command = ["cargo", "build"]
outputs = ["output/**/*.conda"]
smoke = ["example-airlock --version"]
publish = ["conda-prefix", "conda-static"]

[[stores]]
name = "conda-prefix"
kind = "prefix-dev"
channel = "someone/channel"
writable = true
auth = "oidc"
verifier = "registry"

[[stores]]
name = "conda-static"
kind = "static"
writable = true
needs_index = true
verifier = "file"

[workflow.checks]
format = "cargo fmt --check"
"#;

    /// A project whose only store is the implicit GitHub Releases destination.
    const RELEASES_MANIFEST: &str = r#"
schema = 1

[project]
name = "example"
platforms = ["linux-64"]
repository = "https://github.com/someone/example"

[[packages]]
name = "example-airlock"
kind = "conda"
command = ["cargo", "build"]
outputs = ["output/**/*.conda"]
smoke = ["example-airlock --version"]
publish = []

[[stores]]
name = "releases"
kind = "releases"
channel = "someone/example"
writable = true
auth = "token"
secrets = ["GITHUB_TOKEN"]
verifier = "registry"
"#;

    /// The three `v0.1.0` destinations, spelled as this repository spells them.
    const THREE_STORES: &str = r#"
schema = 1

[project]
name = "example"
platforms = ["linux-64"]
repository = "https://github.com/someone/example"

[[packages]]
name = "example-airlock"
kind = "conda"
command = ["cargo", "build"]
outputs = ["output/**/*.conda"]
smoke = ["example-airlock --version"]
publish = ["prefix-dev", "branch"]
release = true

[[stores]]
name = "prefix-dev"
kind = "prefix-dev"
channel = "someone/channel"
writable = true
auth = "oidc"
verifier = "registry"

[[stores]]
name = "branch"
kind = "branch"
writable = true
auth = "token"
secrets = ["GITHUB_TOKEN"]
verifier = "file"

[[stores]]
name = "releases"
kind = "releases"
channel = "someone/example"
writable = true
auth = "token"
secrets = ["GITHUB_TOKEN"]
verifier = "registry"
"#;

    /// `THREE_STORES` with the release body declared as a file, as this repository declares it.
    const NOTED_RELEASE: &str = r#"
schema = 1

[project]
name = "example"
platforms = ["linux-64"]
repository = "https://github.com/someone/example"

[[packages]]
name = "example-airlock"
kind = "conda"
command = ["cargo", "build"]
outputs = ["output/**/*.conda"]
smoke = ["example-airlock --version"]
publish = ["branch"]
release = true

[[stores]]
name = "branch"
kind = "branch"
writable = true
auth = "token"
secrets = ["GITHUB_TOKEN"]
verifier = "file"

[[stores]]
name = "releases"
kind = "releases"
channel = "someone/example"
writable = true
auth = "token"
secrets = ["GITHUB_TOKEN"]
verifier = "registry"
notes = "docs/releases/0.1.0.md"
"#;

    /// Three packages reaching one branch store. A snapshot push replaces the whole tree,
    /// so three separate `publish` steps would leave only the last package on the channel —
    /// exactly what run 43045274207 did to `refs/heads/channel`.
    const MANY_PACKAGES: &str = r#"
schema = 1

[project]
name = "example"
platforms = ["linux-64"]
repository = "https://github.com/someone/example"

[[packages]]
name = "example-airlock"
kind = "conda"
command = ["cargo", "build"]
outputs = ["output/**/*.conda"]
smoke = ["example-airlock --version"]
publish = ["prefix-dev", "branch"]
release = true

[[packages]]
name = "example-kit"
kind = "conda"
command = ["cargo", "build"]
outputs = ["output/**/*.conda"]
smoke = ["example-kit --version"]
publish = ["prefix-dev", "branch"]
release = true

[[packages]]
name = "example-facade"
kind = "conda"
command = ["cargo", "build"]
outputs = ["output/**/*.conda"]
smoke = ["example-facade --version"]
publish = ["prefix-dev", "branch"]
release = true

[[stores]]
name = "prefix-dev"
kind = "prefix-dev"
channel = "someone/channel"
writable = true
auth = "oidc"
verifier = "registry"

[[stores]]
name = "branch"
kind = "branch"
writable = true
needs_index = true
auth = "token"
secrets = ["GITHUB_TOKEN"]
verifier = "file"

[[stores]]
name = "releases"
kind = "releases"
channel = "someone/example"
writable = true
auth = "token"
secrets = ["GITHUB_TOKEN"]
verifier = "registry"
"#;

    /// The same destinations, with the branch store naming its ref the way this repository does.
    fn named_ref_manifest() -> Manifest {
        Manifest::from_toml(&THREE_STORES.replace(
            "name = \"branch\"\nkind = \"branch\"",
            "name = \"branch\"\nkind = \"branch\"\nbranch = \"universe/conda-channel\"",
        ))
        .expect("the manifest parses")
    }

    /// The three `v0.1.0` destinations plus a GitHub Pages store, spelled as a project that
    /// serves its channel from Pages would spell them.
    ///
    /// A Pages store is `static` and not writable: nothing uploads to it. The tree it serves is
    /// generated from what the other stores accepted, which is why it carries no packages of its
    /// own and why its `url` is what a consumer's solver points at.
    const PAGES_STORE: &str = r#"
schema = 1

[project]
name = "example"
platforms = ["linux-64"]
repository = "https://github.com/someone/example"

[[packages]]
name = "example-airlock"
kind = "conda"
command = ["cargo", "build"]
outputs = ["output/**/*.conda"]
smoke = ["example-airlock --version"]
publish = ["prefix-dev", "branch"]
release = true

[[stores]]
name = "prefix-dev"
kind = "prefix-dev"
channel = "someone/channel"
writable = true
auth = "oidc"
verifier = "registry"

[[stores]]
name = "branch"
kind = "branch"
writable = true
auth = "token"
secrets = ["GITHUB_TOKEN"]
verifier = "file"

[[stores]]
name = "releases"
kind = "releases"
channel = "someone/example"
writable = true
auth = "token"
secrets = ["GITHUB_TOKEN"]
verifier = "registry"

[[stores]]
name = "pages"
kind = "static"
writable = false
needs_index = true
url = "https://someone.github.io/example"
pages = true
"#;

    fn manifest() -> Manifest {
        Manifest::from_toml(MANIFEST).expect("manifest parses")
    }

    // The renderer stamps its own version into every file it writes, so a version that came
    // from anywhere but the workspace would label the release with a number no other artifact
    // carries. `env!` resolves this crate's inherited manifest; the core's constant resolves
    // the same literal, and the two agreeing is what says the inheritance holds.
    #[test]
    fn the_renderer_reports_the_version_the_rest_of_the_train_carries() {
        assert_eq!(VERSION, env!("CARGO_PKG_VERSION"));
        assert_eq!(VERSION, universe_core::VERSION);
    }

    /// The same project with nothing published anywhere: no stores, no smoke commands.
    fn lean() -> (Manifest, String) {
        let text = MANIFEST
            .replace("smoke = [\"example-airlock --version\"]\n", "")
            .replace(
                "publish = [\"conda-prefix\", \"conda-static\"]",
                "release = false\npublish = []",
            )
            .replace("writable = true\n", "writable = false\n");
        let manifest = Manifest::from_toml(&text).expect("manifest parses");
        (manifest, text)
    }

    #[test]
    fn every_pixi_setup_names_the_version_it_installs() {
        // An unpinned setup asks the GitHub API for the newest Pixi on every run, so the job
        // can fail before it reads the repository at all — which is what happened twice on
        // `main`. A generated workflow that names the version cannot.
        let rendered = render(&manifest(), MANIFEST).expect("renders").files;
        let mut setups = 0;
        for file in &rendered {
            let lines: Vec<&str> = file.content.lines().collect();
            for (index, line) in lines.iter().enumerate() {
                if !line
                    .trim_start()
                    .starts_with("- uses: prefix-dev/setup-pixi@")
                {
                    continue;
                }
                setups += 1;
                // The inputs live in the step's own `with:` block, so the block has to be read
                // until the next step starts: a `pixi-version` belonging to a later step would
                // otherwise satisfy this.
                let block: String = lines[index + 1..]
                    .iter()
                    .take_while(|next| !next.trim_start().starts_with("- "))
                    .copied()
                    .collect::<Vec<_>>()
                    .join("\n");
                assert!(
                    block.contains("pixi-version:"),
                    "a setup step installs whatever Pixi is newest: {line}\n{block}"
                );
                assert!(
                    block.contains(universe_assets::PIXI_VERSION),
                    "the setup step names a version the repository does not build with: {block}"
                );
            }
        }
        assert!(
            setups > 0,
            "no job sets Pixi up, so this test asserted nothing"
        );
    }

    #[test]
    fn every_job_is_bounded_so_a_hang_reports_instead_of_holding_a_runner() {
        // GitHub's default ceiling is six hours, and a job that sits there reads `pending`
        // rather than `failed` — the difference between a broken build and a silence.
        let rendered = render(&manifest(), MANIFEST).expect("renders").files;
        let mut bounded = 0;
        for file in &rendered {
            for line in file.content.lines() {
                let trimmed = line.trim();
                if let Some(value) = trimmed.strip_prefix("timeout-minutes:") {
                    let minutes: u32 = value.trim().parse().expect("a number of minutes");
                    assert!(
                        (1..=60).contains(&minutes),
                        "a bound of {minutes} minutes is not a useful one: {file:?}"
                    );
                    bounded += 1;
                }
            }
        }
        // One per job in the main file, plus one per publish job.
        assert!(bounded >= 5, "only {bounded} jobs carry a bound");
    }

    #[test]
    fn the_shared_compile_is_cached_rather_than_paid_for_by_every_job() {
        // Four jobs in `universe.yml` each compiled this workspace from cold, which was 71s,
        // 55s, 68s and 63s on 2026-11-08 — the same build, four times.
        let rendered = render(&manifest(), MANIFEST).expect("renders").files;
        let main = &rendered[0].content;
        // The invariant is per job that sets Pixi up — that is the job that is about to
        // compile. The verdict job reads its predecessors' results and has neither.
        let caches = main
            .lines()
            .filter(|line| {
                line.trim_start()
                    .starts_with("- uses: Swatinem/rust-cache@")
            })
            .count();
        let setups = main
            .lines()
            .filter(|line| {
                line.trim_start()
                    .starts_with("- uses: prefix-dev/setup-pixi@")
            })
            .count();
        assert!(
            setups > 0,
            "no job sets Pixi up, so this test asserted nothing"
        );
        assert_eq!(
            caches, setups,
            "every job that installs the toolchain goes on to compile; {caches} of {setups}              restore the cache"
        );
        assert!(
            main.contains("shared-key: universe"),
            "a per-job key would cache four separate builds and share nothing"
        );
    }

    #[test]
    fn checking_the_manifest_does_not_wait_for_the_plan() {
        // `validate` used to need `view`, so the critical path was the sum of two jobs instead
        // of the slower of them, and a formatting error could not be reported until the plan
        // had been rendered.
        let rendered = render(&manifest(), MANIFEST).expect("renders");
        let text = &rendered.files[0].content;
        let block = text
            .split("  validate:")
            .nth(1)
            .and_then(|rest| rest.split("  build:").next())
            .expect("a validate job");
        assert!(
            !block.contains("needs:"),
            "validate should run beside view, not after it:\n{block}"
        );
    }

    /// A manifest with no package lane: an Airlock-only consumer.
    const AIRLOCK_ONLY: &str = r#"
schema = 1

[project]
name = "consumer"
platforms = ["linux-64"]

[airlock.developer]
adapters = ["pixi"]

[airlock.developer.restore]
remote = "https://example.invalid/universe.git"
branch = "universe/bootstrap"
commit = "0123456789abcdef0123456789abcdef01234567"
"#;

    #[test]
    fn a_manifest_with_no_packages_renders_no_package_lane() {
        // GitHub issue 122. `universe.yml` is the package lane, and a build job over zero
        // declared packages uploads nothing with `if-no-files-found: error` — an always-red
        // required check in a repository that never asked for the lane.
        let manifest = Manifest::from_toml(AIRLOCK_ONLY).expect("parses");
        assert_eq!(
            derived_files(&manifest),
            [AIRLOCK_FILE],
            "a consumer that declares no packages was given a producer lane"
        );
    }

    #[test]
    fn a_package_lane_that_publishes_nothing_still_renders() {
        // Declaring packages is what asks for the lane. Build-and-test without publication is
        // a real configuration and must not lose its workflow to the rule above.
        let (lean, _) = lean();
        assert_eq!(derived_files(&lean), [MAIN_FILE]);
    }

    #[test]
    fn dropping_the_package_lane_removes_the_file_it_rendered() {
        // The file set follows the manifest in both directions: a workflow rendered by an
        // earlier manifest is removed when the packages that called for it go away, and only
        // because it carries the stamp this renderer writes.
        let directory = tempfile::tempdir().expect("temp dir");
        let main = render(&manifest(), MANIFEST)
            .expect("renders")
            .files
            .into_iter()
            .find(|file| file.path == MAIN_FILE)
            .expect("the main file");
        std::fs::write(directory.path().join(MAIN_FILE), &main.content).expect("leftover");

        let airlock_only = Manifest::from_toml(AIRLOCK_ONLY).expect("parses");
        let output = render(&airlock_only, AIRLOCK_ONLY).expect("renders");
        super::write(directory.path(), &output).expect("writes");

        assert!(
            !directory.path().join(MAIN_FILE).exists(),
            "the package lane was dropped and its file stayed"
        );
    }

    #[test]
    fn the_file_set_is_derived_from_the_manifest() {
        let text = format!("{MANIFEST}\n[airlock.developer]\nadapters = [\"pixi\"]\n");
        let with_airlock = Manifest::from_toml(&text).expect("parses");
        assert_eq!(
            derived_files(&with_airlock),
            [MAIN_FILE, PUBLISH_FILE, AIRLOCK_FILE]
        );

        // Without stores or adapters only the main file exists.
        let (lean, _) = lean();
        assert_eq!(derived_files(&lean), [MAIN_FILE]);
    }

    #[test]
    fn rendering_is_byte_identical_across_runs() {
        let manifest = manifest();
        let first = render(&manifest, MANIFEST).expect("renders");
        let second = render(&manifest, MANIFEST).expect("renders");
        assert_eq!(first, second);
    }

    #[test]
    fn every_rendered_file_ends_with_a_newline() {
        let output = render(&manifest(), MANIFEST).expect("renders");
        for file in &output.files {
            assert!(
                file.content.ends_with('\n'),
                "{} does not end with a newline, which every editor and diff tool expects",
                file.path
            );
        }
    }

    #[test]
    fn every_rendered_file_records_the_manifest_it_came_from() {
        let manifest = manifest();
        let output = render(&manifest, MANIFEST).expect("renders");
        let digest = sha256::sha256_hex(MANIFEST.as_bytes());
        for file in &output.files {
            assert!(
                file.content.contains(&format!(
                    "# @generated by universe {}",
                    universe_core::VERSION
                )),
                "{} lacks the generator header",
                file.path
            );
            assert!(file.content.contains(&digest));
            assert!(file.content.contains("Source of truth: universe.toml"));
        }
        assert!(
            output.files[0]
                .content
                .contains(&format!("sha256:{digest}"))
        );
    }

    #[test]
    fn the_generated_yaml_keeps_github_expressions_untouched() {
        let output = render(&manifest(), MANIFEST).expect("renders");
        let main = &output.files[0].content;
        // `<<`/`>>` delimiters exist so these survive verbatim.
        assert!(main.contains("${{ github.workflow }}"));
        assert!(main.contains("${{ github.ref }}"));
        assert!(main.contains("${{ toJSON(needs) }}"));
    }

    #[test]
    fn the_publish_file_gets_one_job_per_writable_store() {
        let output = render(&manifest(), MANIFEST).expect("renders");
        let publish = output
            .files
            .iter()
            .find(|file| file.path == PUBLISH_FILE)
            .expect("publish file");
        assert!(publish.content.contains("name: publish / conda-prefix"));
        assert!(publish.content.contains("name: publish / conda-static"));
        // Declared order is publish order, per store.
        let prefix = publish
            .content
            .find("--store conda-prefix")
            .expect("prefix job");
        let static_store = publish
            .content
            .find("--store conda-static")
            .expect("static job");
        assert!(prefix < static_store);
    }

    #[test]
    fn an_oidc_store_gets_a_token_exchange_and_no_secret() {
        let output = render(&manifest(), MANIFEST).expect("renders");
        let publish = &output
            .files
            .iter()
            .find(|file| file.path == PUBLISH_FILE)
            .expect("publish file")
            .content;
        assert!(publish.contains("id-token: write"));
        assert!(
            !publish.contains("secrets.CONDA_PREFIX"),
            "oidc holds no secret"
        );
    }

    #[test]
    fn the_publish_file_builds_what_it_publishes() {
        // The bug this pins: `download-artifact` reads only its own run's artifacts, so a
        // publish job that downloads `universe-*` while the file has no build of its own
        // finds nothing. The first tag would have published an empty directory and reported
        // success — the failure mode that looks like a release.
        let output = render(&manifest(), MANIFEST).expect("renders");
        let publish = &output
            .files
            .iter()
            .find(|file| file.path == PUBLISH_FILE)
            .expect("publish file")
            .content;

        assert!(
            publish.contains("name: publish / build"),
            "the publish file has no build job to produce `output/`"
        );
        // The build must upload, and the upload must be the artifact the publishers ask for
        // by name; a build that keeps its bytes on its own runner publishes nothing.
        let build = publish
            .split("name: publish / conda-prefix")
            .next()
            .expect("the build job precedes the stores");
        assert!(build.contains("actions/upload-artifact"));
        assert!(build.contains("name: universe-${{ matrix.platform }}"));

        // Every job that downloads the artifact has to wait for the build.
        for store in ["conda-prefix", "conda-static"] {
            let job = publish
                .split(&format!("name: publish / {store}"))
                .nth(1)
                .expect("the store's job");
            let job = job.split("name: publish /").next().expect("the job's body");
            assert!(
                job.contains("needs: [build]"),
                "`{store}` downloads the artifact without waiting for the build"
            );
        }
        // And the verdict counts it, so a failed build cannot read as a successful release.
        let verdict = publish
            .split("name: publish / verdict")
            .nth(1)
            .expect("the verdict job");
        assert!(verdict.contains("build"));
    }

    #[test]
    fn the_publish_build_emits_fragments_before_it_uploads_them() {
        // The `v0.1.0-rc1` rehearsal failed with `channel.fragment.none` in both publish
        // jobs. `universe.yml` audits in a separate job and keeps its findings there; the
        // publish file never audited at all, so its artifact held receipts but no
        // fragments. A publisher reads `output/fragments` from the artifact it downloads,
        // so the publish build has to write them before it uploads `output/`.
        let rendered = render(&manifest(), MANIFEST).expect("renders");
        let publish = &rendered
            .files
            .iter()
            .find(|file| file.path == PUBLISH_FILE)
            .expect("publish file")
            .content;
        let build = publish
            .split("name: publish / conda-prefix")
            .next()
            .expect("the build job precedes the stores");

        let audit = format!("audit --json --emit-fragment {ARTIFACT_PATH}/fragments");
        let audit_at = build.find(&audit).unwrap_or_else(|| {
            panic!("the publish build emits no fragments into the uploaded tree:\n{build}")
        });
        let upload_at = build
            .find("actions/upload-artifact@")
            .expect("the publish build uploads an artifact");
        assert!(
            audit_at < upload_at,
            "fragments are written after the upload, so no publisher can read them:\n{build}"
        );
        assert!(
            build.contains(&format!("path: {ARTIFACT_PATH}")),
            "the uploaded directory is not the one the audit wrote into:\n{build}"
        );

        // This is a publish-file defect. The validation workflow keeps its own audit job
        // rather than auditing twice inside the build.
        let main = &rendered.files[0].content;
        assert!(main.contains("name: universe / audit"));
    }

    #[test]
    fn a_store_environment_is_declared_on_the_one_job_that_publishes_there() {
        // prefix.dev matches the OIDC token's `environment` claim when its trusted publisher
        // was declared with one. GitHub adds that claim only for a job that declares the
        // environment, so the name travels from the manifest to exactly one job — declaring
        // it on every job would put an unrelated environment on the branch publisher too.
        let text = MANIFEST.replace(
            "auth = \"oidc\"",
            "auth = \"oidc\"\nenvironment = \"prefix\"",
        );
        let manifest = Manifest::from_toml(&text).expect("a store may name an environment");
        let rendered = render(&manifest, &text).expect("renders");
        let publish = &rendered
            .files
            .iter()
            .find(|file| file.path == PUBLISH_FILE)
            .expect("publish file")
            .content;

        let hosted = publish
            .split("name: publish / conda-prefix")
            .nth(1)
            .expect("the hosted store's job")
            .split("name: publish /")
            .next()
            .expect("the hosted job's body");
        assert!(
            hosted.contains("environment: prefix"),
            "the hosted publish job declares no environment:\n{hosted}"
        );

        let build = publish
            .split("name: publish / conda-prefix")
            .next()
            .expect("the build job precedes the stores");
        let static_store = publish
            .split("name: publish / conda-static")
            .nth(1)
            .expect("the static store's job")
            .split("name: publish /")
            .next()
            .expect("the static job's body");
        assert!(
            !build.contains("environment:") && !static_store.contains("environment:"),
            "an unrelated job declares the hosted store's environment:\n{publish}"
        );
    }

    #[test]
    fn an_implicit_releases_store_gets_a_publish_job_with_its_package() {
        // decision-127 makes GitHub Releases the one destination a package reaches without
        // naming it. A job with no publish step would look like a destination and publish
        // nothing, so the rendered YAML has to carry both the job and its package command.
        let manifest = Manifest::from_toml(RELEASES_MANIFEST).expect("the manifest parses");
        let rendered = render(&manifest, RELEASES_MANIFEST).expect("renders");
        let publish = &rendered
            .files
            .iter()
            .find(|file| file.path == PUBLISH_FILE)
            .expect("publish file")
            .content;
        let releases = publish
            .split("name: publish / releases")
            .nth(1)
            .expect("the releases job")
            .split("name: publish /")
            .next()
            .expect("the releases job's body");
        assert!(
            releases.contains(
                "publish-artifacts --store releases --force --package example-airlock --json"
            ),
            "the releases job publishes no package:\n{releases}"
        );
        assert!(
            releases.contains("contents: write"),
            "a GitHub Release needs contents: write:\n{releases}"
        );
        assert!(
            releases.contains("RELEASES_GITHUB_TOKEN"),
            "the job does not export the store's token:\n{releases}"
        );
    }

    #[test]
    fn artifact_stores_ask_to_overwrite_a_changed_digest() {
        // A rebuilt `.conda` under an occupied name carries a different sha256. prefix.dev
        // and GitHub Releases both refuse a changed digest unless the job passes `--force`;
        // the branch store pushes a snapshot and does not take that flag.
        let manifest = manifest();
        let rendered = render(&manifest, MANIFEST).expect("renders");
        let publish = &rendered
            .files
            .iter()
            .find(|file| file.path == PUBLISH_FILE)
            .expect("publish file")
            .content;
        assert!(
            publish.contains(
                "publish-artifacts --store conda-prefix --force --package example-airlock --json"
            ),
            "the hosted job does not ask to overwrite a changed digest:\n{publish}"
        );
        let static_job = publish
            .split("name: publish / conda-static")
            .nth(1)
            .expect("the static store's job")
            .split("name: publish /")
            .next()
            .expect("the static job's body");
        assert!(
            !static_job.contains("--force"),
            "a snapshot push must not be given an overwrite flag:\n{static_job}"
        );
    }

    #[test]
    fn the_three_release_destinations_force_only_the_two_artifact_stores() {
        // The owner chose `--force` for `prefix-dev` and `releases` after a rebuilt
        // artifact occupied the same name with a different digest. The branch store's
        // `publish` verb is a whole-snapshot push and does not accept that flag.
        let manifest = Manifest::from_toml(THREE_STORES).expect("the manifest parses");
        let rendered = render(&manifest, THREE_STORES).expect("renders");
        let publish = &rendered
            .files
            .iter()
            .find(|file| file.path == PUBLISH_FILE)
            .expect("publish file")
            .content;
        let job = |name: &str| {
            publish
                .split(&format!("name: publish / {name}"))
                .nth(1)
                .unwrap_or_else(|| panic!("no {name} job:\n{publish}"))
                .split("name: publish /")
                .next()
                .expect("a job body")
        };
        assert!(
            job("prefix-dev").contains(
                "publish-artifacts --store prefix-dev --force --package example-airlock --json"
            ),
            "the hosted channel does not overwrite a changed digest:\n{publish}"
        );
        assert!(
            job("releases").contains(
                "publish-artifacts --store releases --force --package example-airlock --json"
            ),
            "the GitHub Release does not overwrite a changed digest:\n{publish}"
        );
        let branch = job("branch");
        assert!(
            branch.contains("publish --store branch --remote"),
            "the branch store does not push its snapshot:\n{branch}"
        );
        assert!(
            !branch.contains("--force"),
            "a branch push was given an artifact-store overwrite flag:\n{branch}"
        );
    }

    #[test]
    fn the_release_job_names_the_file_its_body_is_read_from() {
        // The body is a file in the tree, so the job that publishes it names the path the
        // manifest names. A job that rendered the default would publish the tag's artifacts
        // with no description at all.
        let manifest = Manifest::from_toml(NOTED_RELEASE).expect("the manifest parses");
        let rendered = render(&manifest, NOTED_RELEASE).expect("renders");
        let publish = &rendered
            .files
            .iter()
            .find(|file| file.path == PUBLISH_FILE)
            .expect("publish file")
            .content;
        let releases = publish
            .split("name: publish / releases")
            .nth(1)
            .unwrap_or_else(|| panic!("no releases job:\n{publish}"))
            .split("name: publish /")
            .next()
            .expect("a job body");
        assert!(
            releases.contains("--notes-file docs/releases/0.1.0.md"),
            "the releases job publishes no body:\n{releases}"
        );
        // The path is repository-relative in the manifest, so the job must have checked the
        // tree out at the root it resolves against.
        let branch = publish
            .split("name: publish / branch")
            .nth(1)
            .expect("the branch job")
            .split("name: publish /")
            .next()
            .expect("a job body");
        assert!(
            !branch.contains("--notes-file"),
            "a branch push was handed release prose:\n{branch}"
        );
    }

    #[test]
    fn a_snapshot_store_pushes_every_package_in_one_command() {
        // A branch store is one orphan tree. Run 43045274207 rendered three `publish`
        // steps, one per package; each force-push replaced the whole tree, so `channel`
        // ended up holding only the last package (the `universe` facade). An artifact
        // store is different: its steps upload one file each and stay per package.
        let manifest = Manifest::from_toml(MANY_PACKAGES).expect("the manifest parses");
        let rendered = render(&manifest, MANY_PACKAGES).expect("renders");
        let publish = &rendered
            .files
            .iter()
            .find(|file| file.path == PUBLISH_FILE)
            .expect("publish file")
            .content;
        let branch = publish
            .split("name: publish / branch")
            .nth(1)
            .expect("the branch job")
            .split("name: publish /")
            .next()
            .expect("the branch job's body");
        let commands: Vec<&str> = branch
            .lines()
            .filter(|line| line.contains("publish --store branch"))
            .collect();
        assert_eq!(
            commands.len(),
            1,
            "a snapshot store must push one tree, not one per package:\n{branch}"
        );
        for package in ["example-airlock", "example-kit", "example-facade"] {
            assert!(
                commands[0].contains(&format!("--package {package}")),
                "the one snapshot omits {package}:\n{}",
                commands[0]
            );
        }
        assert!(
            !branch.contains("--force"),
            "a snapshot push was given an artifact-store overwrite flag:\n{branch}"
        );
        // The artifact stores still upload one file per package: the fix is scoped to
        // the verb that replaces a whole tree.
        for store in ["prefix-dev", "releases"] {
            let body = publish
                .split(&format!("name: publish / {store}"))
                .nth(1)
                .unwrap_or_else(|| panic!("no {store} job"))
                .split("name: publish /")
                .next()
                .expect("a job body");
            for package in ["example-airlock", "example-kit", "example-facade"] {
                assert!(
                    body.contains(&format!(
                        "publish-artifacts --store {store} --force --package {package} --json"
                    )),
                    "{store} no longer uploads {package}:\n{body}"
                );
            }
        }
    }

    /// The body of one job in one rendered file, from its `name:` line to the next job's.
    fn job_body<'a>(file: &'a str, id: &str) -> &'a str {
        file.split(&format!("name: publish / {id}\n"))
            .nth(1)
            .unwrap_or_else(|| panic!("no job named {id}"))
            .split("name: publish /")
            .next()
            .expect("a job body")
    }

    /// The rendered publish file, and nothing else.
    fn publish_file(text: &str) -> String {
        let manifest = Manifest::from_toml(text).expect("the manifest parses");
        render(&manifest, text)
            .expect("renders")
            .files
            .into_iter()
            .find(|file| file.path == PUBLISH_FILE)
            .expect("publish file")
            .content
    }

    #[test]
    fn a_branch_store_publishes_to_the_ref_the_manifest_names() {
        // The ref is where the bytes land. A renderer that omitted the flag would publish to the
        // tool's default, which for this repository is a different ref: the release would go
        // green and the channel a consumer is told about would never change.
        let manifest = named_ref_manifest();
        let publish = render(&manifest, THREE_STORES)
            .expect("renders")
            .files
            .into_iter()
            .find(|file| file.path == PUBLISH_FILE)
            .expect("publish file")
            .content;

        assert!(
            publish.contains(
                "publish --store branch --remote ${{ github.server_url }}/${{ github.repository }}.git --branch universe/conda-channel"
            ),
            "the branch store does not name its ref:\n{publish}"
        );
    }

    #[test]
    fn a_branch_store_that_names_no_ref_leaves_the_choice_to_the_tool() {
        // Absent is not the same as empty: an undeclared ref keeps the rendered command exactly
        // as it was, so a project that never declared one is unaffected by this field existing.
        let manifest = Manifest::from_toml(THREE_STORES).expect("the manifest parses");
        let publish = render(&manifest, THREE_STORES)
            .expect("renders")
            .files
            .into_iter()
            .find(|file| file.path == PUBLISH_FILE)
            .expect("publish file")
            .content;

        assert!(
            publish.contains("publish --store branch --remote"),
            "{publish}"
        );
        assert!(!publish.contains("--branch"), "{publish}");
    }

    #[test]
    fn a_pages_store_renders_a_deployment_and_no_publish_job() {
        // Nothing uploads to a Pages site: the deployment publishes a tree that is *generated*
        // from what the other stores accepted. Rendering a `publish` step for it would send the
        // channel to a verb with no implementation behind that store kind.
        let publish = publish_file(PAGES_STORE);

        assert!(
            publish.contains("publish-artifacts --store prefix-dev"),
            "the other stores stopped publishing:\n{publish}"
        );
        assert!(
            !publish.contains("--store pages"),
            "a Pages store was rendered as a byte destination:\n{publish}"
        );
        assert!(
            publish.contains("name: publish / pages"),
            "no Pages job was rendered:\n{publish}"
        );
        assert!(
            publish.contains("name: publish / pages-deploy"),
            "no Pages deployment was rendered:\n{publish}"
        );
    }

    #[test]
    fn the_pages_job_generates_the_index_from_the_build_it_depends_on() {
        // The tree the site serves is `universe index`'s output, generated in the job from the
        // artifacts the build uploaded — not a directory committed to the repository, which
        // would be a second copy of the channel that can drift from the one the stores hold.
        let publish = publish_file(PAGES_STORE);
        let pages = job_body(&publish, "pages");

        assert!(
            pages.contains("needs: [build]"),
            "the Pages job does not wait for the build:\n{pages}"
        );
        assert!(
            pages.contains("download-artifact"),
            "the Pages job does not download the built artifacts:\n{pages}"
        );
        assert!(
            pages.contains(
                "index --output output/pages --base-url https://someone.github.io/example"
            ),
            "the Pages job does not generate the index at the store's own URL:\n{pages}"
        );
        assert!(
            pages.contains("actions/upload-pages-artifact@"),
            "the Pages job does not upload a Pages artifact:\n{pages}"
        );
        assert!(
            pages.contains("configure-pages@"),
            "the Pages job does not configure Pages, so a repository with no site fails at deploy time instead of here:\n{pages}"
        );
        assert!(
            !pages.contains("publish"),
            "the Pages job runs a publish verb:\n{pages}"
        );
    }

    #[test]
    fn the_deployment_is_its_own_job_with_the_privileges_it_needs() {
        // `pages: write` and `id-token: write` belong to the deployment, not to the job that
        // renders a directory. A second write scope on one job is what `MultipleWriteScopes`
        // refuses, so the split is also what keeps the renderer's own rule satisfied.
        let publish = publish_file(PAGES_STORE);
        let deploy = job_body(&publish, "pages-deploy");

        assert!(
            deploy.contains("needs: [pages]"),
            "the deployment does not wait for the artifact:\n{deploy}"
        );
        assert!(
            deploy.contains("pages: write") && deploy.contains("id-token: write"),
            "the deployment lacks the scopes `deploy-pages` needs:\n{deploy}"
        );
        assert!(
            deploy.contains("deploy-pages@"),
            "the deployment runs no deployment action:\n{deploy}"
        );
        assert!(
            deploy.contains("environment: github-pages"),
            "the deployment declares no environment, so the deployment URL is not recorded:\n{deploy}"
        );
        assert!(
            !deploy.contains("pixi run"),
            "the deployment job pays for a checkout and a toolchain it does not use:\n{deploy}"
        );
    }

    #[test]
    fn a_project_without_a_pages_store_renders_no_pages_job() {
        let publish = publish_file(THREE_STORES);
        assert!(
            !publish.contains("pages"),
            "a project that declares no Pages store got a Pages job:\n{publish}"
        );
    }

    #[test]
    fn the_verdict_waits_for_the_deployment() {
        // A verdict that did not need the deployment would report the release green while the
        // site was still the previous one.
        let publish = publish_file(PAGES_STORE);
        let verdict = publish
            .split("name: publish / verdict")
            .nth(1)
            .expect("the verdict job");
        assert!(
            verdict.contains("pages-deploy") || publish.contains("needs: [pages-deploy]"),
            "the verdict does not wait for the deployment:\n{publish}"
        );
    }

    #[test]
    fn a_token_store_declares_its_secret_and_verifies_itself() {
        let text = MANIFEST.replace(
            "auth = \"oidc\"\nverifier = \"registry\"",
            "auth = \"token\"\nread_auth = \"token\"\nsecrets = [\"conda-token\"]\nverifier = \"registry\"",
        );
        let manifest = Manifest::from_toml(&text).expect("parses");
        let output = render(&manifest, &text).expect("renders");
        let publish = &output
            .files
            .iter()
            .find(|file| file.path == PUBLISH_FILE)
            .expect("publish file")
            .content;
        assert!(publish.contains("CONDA_PREFIX_CONDA_TOKEN: ${{ secrets.conda-token }}"));
        // A credentialed destination verifies inside its own job...
        assert!(
            publish
                .contains("pixi run --frozen universe -- verify --published --store conda-prefix")
        );
        // ...while the anonymous one is re-read by `verify-index`.
        assert!(publish.contains("name: publish / verify-index"));
    }

    #[test]
    fn an_anonymous_destination_gets_a_separate_reader() {
        let output = render(&manifest(), MANIFEST).expect("renders");
        let publish = &output
            .files
            .iter()
            .find(|file| file.path == PUBLISH_FILE)
            .expect("publish file")
            .content;
        assert!(publish.contains("name: publish / verify-index"));
    }

    #[test]
    fn the_verification_ranges_follow_the_manifest() {
        let text = MANIFEST.replace(
            "[workflow.checks]",
            "[workflow]\nverify_artifact = false\nverify_published = false\n\n[workflow.checks]",
        );
        let manifest = Manifest::from_toml(&text).expect("parses");
        let output = render(&manifest, &text).expect("renders");
        assert!(
            !output.files[0]
                .content
                .contains("name: universe / verify-artifact")
        );
        assert!(
            !output
                .files
                .iter()
                .find(|file| file.path == PUBLISH_FILE)
                .expect("publish file")
                .content
                .contains("verify-index")
        );
    }

    #[test]
    fn declared_checks_become_steps_in_declaration_order() {
        let text = MANIFEST.replace(
            "format = \"cargo fmt --check\"",
            "typecheck = \"cargo check\"\nformat = \"cargo fmt --check\"",
        );
        let manifest = Manifest::from_toml(&text).expect("parses");
        let main = &render(&manifest, &text).expect("renders").files[0].content;
        let format = main.find("Check format").expect("format step");
        let typecheck = main.find("Check typecheck").expect("typecheck step");
        assert!(format < typecheck, "declaration order is not preserved");
    }

    #[test]
    fn required_checks_are_the_pull_request_jobs_in_order() {
        let output = render(&manifest(), MANIFEST).expect("renders");
        assert_eq!(
            output.required_checks,
            [
                "universe / view",
                "universe / validate",
                "universe / build",
                "universe / audit",
                "universe / verify-artifact",
                "universe / verdict",
            ]
        );
    }

    #[test]
    fn a_publish_file_contributes_no_required_checks() {
        let output = render(&manifest(), MANIFEST).expect("renders");
        // It does not run on a pull request, so requiring it would wait forever.
        assert!(
            !output
                .required_checks
                .iter()
                .any(|check| check.starts_with("publish /"))
        );
    }

    #[test]
    fn the_derived_store_mapping_is_printable() {
        let mapping = super::RenderOutput::store_mapping(&manifest());
        assert_eq!(
            mapping,
            [
                (
                    "conda-prefix".to_owned(),
                    vec!["example-airlock".to_owned()]
                ),
                (
                    "conda-static".to_owned(),
                    vec!["example-airlock".to_owned()]
                ),
            ]
        );
    }

    #[test]
    fn a_job_holding_two_write_scopes_is_refused() {
        // The publish file's jobs are derived, so the refusal is proved on the check that
        // guards them rather than by hand-building a manifest the validator accepts.
        let job = super::Job::new("publish", "publish", Vec::new())
            .with_permissions(&[("contents", "write"), ("packages", "write")]);
        assert_eq!(job.write_scopes(), ["contents", "packages"]);
        let single = super::Job::new("publish", "publish", Vec::new())
            .with_permissions(&[("packages", "write"), ("id-token", "write")]);
        assert_eq!(
            single.write_scopes(),
            ["packages"],
            "id-token is an exchange, not a resource write"
        );
    }

    #[test]
    fn rendering_errors_explain_themselves() {
        let privilege = RenderError::MultipleWriteScopes {
            job: "publish / x".to_owned(),
            scopes: vec!["contents".to_owned(), "packages".to_owned()],
        };
        assert!(privilege.to_string().contains("split it instead"));
        assert!(
            RenderError::Template("boom".to_owned())
                .to_string()
                .contains("template failed")
        );
        assert!(
            RenderError::Io {
                path: "a".to_owned(),
                message: "denied".to_owned()
            }
            .to_string()
            .contains("denied")
        );
    }

    #[test]
    fn job_identifiers_survive_awkward_store_names() {
        assert_eq!(to_identifier("conda-prefix"), "conda-prefix");
        assert_eq!(to_identifier("My Store.Name"), "my-store-name");
    }

    #[test]
    fn writing_then_checking_finds_no_drift() {
        let directory = tempfile::tempdir().expect("temp dir");
        let output = render(&manifest(), MANIFEST).expect("renders");
        super::write(directory.path(), &output).expect("writes");
        assert!(super::check(directory.path(), &output).is_empty());
    }

    #[test]
    fn drift_is_named_by_file() {
        let directory = tempfile::tempdir().expect("temp dir");
        let output = render(&manifest(), MANIFEST).expect("renders");
        super::write(directory.path(), &output).expect("writes");
        let target = directory.path().join(MAIN_FILE);
        // A hand edit to a generated file keeps its stamp — the stamp is the first line and
        // an edit is to the body. That is what makes drift recognizable as *our* drift.
        let edited =
            "# @generated by universe 0.1.0 — manifest sha256:edited\nname: edited by hand\n";
        std::fs::write(&target, edited).expect("edit");
        let diagnostics = super::check(directory.path(), &output);
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].code(), "ci.file.drift");
        assert!(diagnostics[0].path().expect("path").ends_with(MAIN_FILE));
    }

    #[test]
    fn a_missing_file_is_reported_as_missing() {
        let directory = tempfile::tempdir().expect("empty dir");
        let output = render(&manifest(), MANIFEST).expect("renders");
        let diagnostics = super::check(directory.path(), &output);
        let codes: Vec<&str> = diagnostics
            .iter()
            .map(universe_core::diagnostic::Diagnostic::code)
            .collect();
        assert_eq!(codes, ["ci.file.missing", "ci.file.missing"]);
    }

    #[test]
    fn a_file_the_manifest_no_longer_calls_for_is_refused() {
        let directory = tempfile::tempdir().expect("temp dir");
        let (manifest, text) = lean();
        let output = render(&manifest, &text).expect("renders");
        super::write(directory.path(), &output).expect("writes");
        // A file this renderer wrote and the manifest stopped calling for: stamped, which is
        // what makes it ours to remove.
        std::fs::write(
            directory.path().join(PUBLISH_FILE),
            "# @generated by universe 0.1.0 — manifest sha256:leftover\n",
        )
        .expect("stale file");
        let diagnostics = super::check(directory.path(), &output);
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].code(), "ci.file.undeclared");
    }

    #[test]
    fn a_legacy_publish_file_is_refused_by_check() {
        // The channel's trusted publisher names `release.yml`. Leaving the old file behind
        // would give one tag two tag-triggered workflows, so `ci check` must refuse it.
        let directory = tempfile::tempdir().expect("temp dir");
        let output = render(&manifest(), MANIFEST).expect("renders");
        super::write(directory.path(), &output).expect("writes");
        let legacy = directory.path().join(LEGACY_PUBLISH_FILE);
        std::fs::write(
            &legacy,
            "# @generated by universe 0.1.0 — manifest sha256:legacy\n",
        )
        .expect("legacy file");
        let diagnostics = super::check(directory.path(), &output);
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].code(), "ci.file.undeclared");
        assert!(
            diagnostics[0]
                .path()
                .expect("path")
                .ends_with(LEGACY_PUBLISH_FILE)
        );
    }

    #[test]
    fn writing_the_workflow_set_removes_the_legacy_publish_file() {
        // `ci render` has to be authoritative over the generated set: a rename that
        // leaves the old file on disk is not a rename, it is a second workflow.
        let directory = tempfile::tempdir().expect("temp dir");
        let output = render(&manifest(), MANIFEST).expect("renders");
        let legacy = directory.path().join(LEGACY_PUBLISH_FILE);
        // The old name as it was on disk when it was ours: carrying the stamp.
        std::fs::write(
            &legacy,
            "# @generated by universe 0.1.0 — manifest sha256:legacy\n",
        )
        .expect("legacy file");
        super::write(directory.path(), &output).expect("writes");
        assert!(
            !legacy.exists(),
            "ci render left a second tag-triggered workflow behind"
        );
    }

    /// One job's block from a rendered file: the lines after `  <id>:` up to the next job.
    fn job_block<'a>(file: &'a str, id: &str) -> &'a str {
        let start = file
            .find(&format!("\n  {id}:\n"))
            .unwrap_or_else(|| panic!("no {id} job:\n{file}"))
            + 1;
        let rest = &file[start..];
        let mut end = rest.len();
        let mut offset = 0;
        for line in rest.split_inclusive('\n').skip(1) {
            offset += line.len();
            let is_job = line.starts_with("  ")
                && !line.starts_with("   ")
                && line.trim_end().ends_with(':');
            if is_job {
                end = offset;
                break;
            }
        }
        &rest[..end]
    }

    /// Names of files `ci render` reports it left alone.
    fn untouched(report: &super::Written) -> Vec<String> {
        report
            .untouched
            .iter()
            .filter_map(|path| path.file_name())
            .map(|name| name.to_string_lossy().into_owned())
            .collect()
    }

    #[test]
    fn a_workflow_this_renderer_did_not_write_is_never_deleted() {
        // The issue's reproducer: an Airlock-only consumer whose own npm lane owns
        // `release.yml`. The filename is shared because prefix.dev matches the trusted
        // publisher on the basename, so ownership has to be decided by content.
        let directory = tempfile::tempdir().expect("temp dir");
        let (manifest, text) = lean();
        let output = render(&manifest, &text).expect("renders");
        let foreign = directory.path().join(PUBLISH_FILE);
        let theirs = "# An unrelated npm publishing lane\nname: publish\n";
        std::fs::write(&foreign, theirs).expect("foreign file");

        let report = super::write(directory.path(), &output).expect("writes");

        assert_eq!(
            std::fs::read_to_string(&foreign).expect("still there"),
            theirs,
            "ci render deleted a workflow it did not write"
        );
        assert_eq!(untouched(&report), [PUBLISH_FILE]);
    }

    #[test]
    fn a_workflow_this_renderer_did_not_write_is_never_overwritten() {
        // The other direction of the same question: the manifest does render that name, and
        // the file there belongs to somebody else. Replacing it is the same silent loss.
        let directory = tempfile::tempdir().expect("temp dir");
        let output = render(&manifest(), MANIFEST).expect("renders");
        let foreign = directory.path().join(PUBLISH_FILE);
        let theirs = "# An unrelated npm publishing lane\n";
        std::fs::write(&foreign, theirs).expect("foreign file");

        let error = super::write(directory.path(), &output).expect_err("refused");

        assert!(
            error.to_string().contains(PUBLISH_FILE),
            "the refusal does not name the file: {error}"
        );
        assert_eq!(
            std::fs::read_to_string(&foreign).expect("still there"),
            theirs,
            "ci render overwrote a workflow it did not write"
        );
    }

    #[test]
    fn check_separates_a_file_we_own_from_a_file_we_do_not() {
        // A stamped leftover is drift to regenerate. A foreign file at a name this manifest
        // no longer renders is somebody else's business: `ci check` must not report it, or
        // every Airlock-only consumer fails on that consumer's own release lane.
        let directory = tempfile::tempdir().expect("temp dir");
        let (manifest, text) = lean();
        let output = render(&manifest, &text).expect("renders");
        std::fs::write(
            directory.path().join(PUBLISH_FILE),
            "# An unrelated npm publishing lane\n",
        )
        .expect("foreign file");
        super::write(directory.path(), &output).expect("writes");

        let diagnostics = super::check(directory.path(), &output);

        let codes: Vec<&str> = diagnostics
            .iter()
            .map(universe_core::diagnostic::Diagnostic::code)
            .collect();
        assert!(
            codes.iter().all(|code| *code == "ci.file.missing"),
            "a foreign file at a name we no longer render was reported: {codes:?}"
        );
    }

    #[test]
    fn check_refuses_a_foreign_file_but_calls_it_foreign() {
        // `ci render` would refuse to write there, so `ci check` must refuse too — and say
        // something different from drift, because the remedy is different: move the other
        // lane, not regenerate this one.
        let directory = tempfile::tempdir().expect("temp dir");
        let output = render(&manifest(), MANIFEST).expect("renders");
        super::write(directory.path(), &output).expect("writes");
        std::fs::write(
            directory.path().join(PUBLISH_FILE),
            "# An unrelated npm publishing lane\n",
        )
        .expect("foreign file");

        let diagnostics = super::check(directory.path(), &output);

        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].code(), "ci.file.foreign");
        assert!(diagnostics[0].path().expect("path").ends_with(PUBLISH_FILE));
    }

    #[test]
    fn no_rendered_checkout_leaves_the_token_in_the_work_tree() {
        // `persist-credentials` defaults to true, which writes the job's token into
        // `.git/config` for every step that follows — including steps that run the project's
        // own build scripts. A job that must push supplies its own token on the step that
        // pushes; the checkout does not hand one to everything.
        let output = render(&manifest(), MANIFEST).expect("renders");
        for file in &output.files {
            let checkouts = file.content.matches(universe_assets::CHECKOUT).count();
            assert!(checkouts > 0, "{} has no checkout to check", file.path);
            assert_eq!(
                file.content.matches("persist-credentials: false").count(),
                checkouts,
                "{} has a checkout that keeps the token: {checkouts} checkout(s)",
                file.path
            );
        }
    }

    #[test]
    fn a_job_that_pushes_carries_its_own_token() {
        // `persist-credentials: false` takes away the implicit credential, so every job that
        // pushes has to name its own. The mirror and the entrypoint render both push to the
        // branch store, and both read the token the store declares.
        let text = format!("{MANIFEST}\n[airlock.developer]\nadapters = [\"pixi\"]\n").replace(
            "kind = \"static\"\nwritable = true",
            "kind = \"static\"\nwritable = true\nauth = \"token\"\nsecrets = [\"GITHUB_TOKEN\"]",
        );
        let manifest = Manifest::from_toml(&text).expect("parses");
        let rendered = render(&manifest, &text).expect("renders");
        let airlock = &rendered
            .files
            .iter()
            .find(|file| file.path == AIRLOCK_FILE)
            .expect("the airlock file")
            .content;
        let job = |name: &str| job_block(airlock, name);
        for pushing in ["mirror", "render-airlock"] {
            let body = job(pushing);
            assert!(
                body.contains("secrets.GITHUB_TOKEN"),
                "the {pushing} job pushes without a credential of its own:\n{body}"
            );
        }
    }

    #[test]
    fn every_store_kind_either_publishes_or_is_refused_before_rendering() {
        // GitHub issue 121 in one assertion: no kind reaches the job builder without a verb.
        // The two tables are the manifest's `publishes` and this crate's `publish_verb`, and a
        // kind added to one without the other is the bug the issue describes.
        for kind in StoreKind::ALL {
            assert_eq!(
                publish_verb(kind).is_some(),
                kind.publishes(),
                "{} disagrees about whether it publishes",
                kind.as_str()
            );
            assert!(
                !matches!(publish_verb(kind), Some("")),
                "{} has an empty verb",
                kind.as_str()
            );
        }
    }

    #[test]
    fn a_writable_store_with_no_verb_renders_no_publish_job() {
        // Defence in depth: validation refuses this manifest, but a job emitted without a
        // publish step would pass its verdict, so the guard sits before the job is assembled
        // rather than after. Construction here is the closest the unit test gets to the bug.
        let mut manifest = Manifest::from_toml(THREE_STORES).expect("parses");
        manifest.stores.push(Store {
            name: "registry-with-no-verb".to_owned(),
            kind: StoreKind::Pypi,
            writable: true,
            needs_index: false,
            read_auth: universe_core::manifest::ReadAuth::None,
            auth: universe_core::manifest::StoreAuth::Token,
            verifier: universe_core::manifest::Verifier::Registry,
            secrets: Vec::new(),
            environment: None,
            channel: Some("someone/example".to_owned()),
            url: None,
            notes: None,
            branch: None,
            pages: false,
            budgets: universe_core::manifest::StoreBudgets::default(),
            tag_prefix: None,
            max_builds_per_version: None,
        });
        let rendered = render(&manifest, THREE_STORES).expect("renders");
        let publish = &rendered
            .files
            .iter()
            .find(|file| file.path == PUBLISH_FILE)
            .expect("publish file")
            .content;
        assert!(
            !publish.contains("publish / registry-with-no-verb"),
            "a store with no verb still got a job:\n{publish}"
        );
    }

    #[test]
    fn the_static_surface_is_where_decision_127_puts_it() {
        assert!(super::has_static_surface(
            universe_core::manifest::PackageKind::Conda
        ));
        assert!(!super::has_static_surface(
            universe_core::manifest::PackageKind::Npm
        ));
    }
}
