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
pub const PUBLISH_FILE: &str = "universe-publish.yml";
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
        Step::uses(universe_assets::CHECKOUT),
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

/// The file names this manifest calls for, in render order.
#[must_use]
pub fn derived_files(manifest: &Manifest) -> Vec<&'static str> {
    let mut files = vec![MAIN_FILE];
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
    match kind {
        StoreKind::Branch | StoreKind::Static => Some("publish"),
        StoreKind::Releases
        | StoreKind::PrefixDev
        | StoreKind::Anaconda
        | StoreKind::Quetz
        | StoreKind::Artifactory
        | StoreKind::S3
        | StoreKind::Filesystem => Some("publish-artifacts"),
        // Nothing implements these yet: no package kind reaches them in v0.1.0, and rendering
        // a command that would fail at run time is worse than rendering none.
        StoreKind::Npmjs
        | StoreKind::GithubPackages
        | StoreKind::Pypi
        | StoreKind::CratesIo
        | StoreKind::Command => None,
    }
}

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
        let mut job = Job::new(&store.name, "publish", vec!["build".to_owned()]).with_timeout(30);
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
            format!("publish --store {} --remote {BRANCH_REMOTE}", store.name)
        } else {
            format!("publish-artifacts --store {}", store.name)
        };
        for package in manifest
            .destinations()
            .into_iter()
            .find(|destination| destination.store.name == store.name)
            .map(|destination| destination.packages)
            .unwrap_or_default()
        {
            job = job.with_step(Step::Run {
                label: format!("Publish {}", package.name),
                command: verb(&format!("{arguments} --package {} --json", package.name)),
            });
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

    let needs: Vec<String> = jobs.iter().map(|job| job.id.clone()).collect();
    jobs.push(Job::new("verdict", "publish", needs).verdict());
    jobs
}

fn airlock_jobs(manifest: &Manifest) -> Vec<Job> {
    let adapters: Vec<String> = manifest
        .lock_adapters()
        .iter()
        .map(|adapter| adapter.as_str().to_owned())
        .collect();
    let mirror = checkout_and_pixi(
        Job::new("mirror", "airlock", Vec::new())
            .with_permissions(&[("contents", "write")])
            .with_matrix("adapter", adapters.clone())
            .with_step(Step::Run {
                label: "Mirror one lock format".to_owned(),
                command: verb("airlock mirror --adapter ${{ matrix.adapter }} --json"),
            }),
    );
    let mut mirror = mirror;
    mirror.fail_fast = false;
    let render = checkout_and_pixi(
        Job::new("render-airlock", "airlock", vec!["mirror".to_owned()])
            .with_permissions(&[("contents", "write")])
            .with_step(Step::Run {
                label: "Render the Airlock restore entrypoints".to_owned(),
                command: verb("airlock render --json"),
            }),
    );
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

/// Write a rendered set into `directory`, creating it when absent.
///
/// # Errors
///
/// Returns [`RenderError::Io`] when the directory or a file cannot be written.
pub fn write(directory: &Path, output: &RenderOutput) -> Result<Vec<PathBuf>, RenderError> {
    std::fs::create_dir_all(directory).map_err(|error| RenderError::Io {
        path: directory.display().to_string(),
        message: error.to_string(),
    })?;
    let mut written = Vec::new();
    for file in &output.files {
        let path = directory.join(&file.path);
        std::fs::write(&path, &file.content).map_err(|error| RenderError::Io {
            path: path.display().to_string(),
            message: error.to_string(),
        })?;
        written.push(path);
    }
    Ok(written)
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
            Ok(_) => diagnostics.push(
                Diagnostic::new(
                    Severity::Error,
                    "ci.file.drift",
                    format!("{} no longer matches the manifest", file.path),
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
    for name in KNOWN_FILES {
        if declared.contains(&name) {
            continue;
        }
        let path = directory.join(name);
        if path.exists() {
            diagnostics.push(
                Diagnostic::new(
                    Severity::Error,
                    "ci.file.undeclared",
                    format!("{name} exists but this manifest calls for no such file"),
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
        AIRLOCK_FILE, ARTIFACT_PATH, MAIN_FILE, PUBLISH_FILE, RenderError, VERSION, derived_files,
        render, sha256, to_identifier,
    };
    use universe_core::manifest::Manifest;

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
        std::fs::write(&target, "# edited by hand\n").expect("edit");
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
        std::fs::write(directory.path().join(PUBLISH_FILE), "stale\n").expect("stale file");
        let diagnostics = super::check(directory.path(), &output);
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].code(), "ci.file.undeclared");
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
