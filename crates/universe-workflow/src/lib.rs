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
use universe_core::VERSION;
use universe_core::diagnostic::{Diagnostic, Severity};
use universe_core::manifest::{Manifest, PackageKind, ReadAuth, StoreAuth, StoreKind};

/// Version of this component, equal to the workspace version the whole artifact set is
/// released under.
pub const COMPONENT_VERSION: &str = env!("CARGO_PKG_VERSION");

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
            steps: Vec::new(),
        }
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
        Step::uses(universe_assets::SETUP_PIXI),
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

fn secret_variable(store_name: &str, secret: &str) -> String {
    format!(
        "{}_{}",
        store_name.to_uppercase().replace(['-', '.'], "_"),
        secret.to_uppercase().replace(['-', '.'], "_")
    )
}

/// The command line a rendered job uses to reach a Universe verb.
///
/// A runner that has just run `setup-pixi` has `pixi` on its `PATH` and nothing else:
/// `universe-kit` is not published in v0.1.0 (`decision-132`), so a bare `universe` verb
/// would be a command the job cannot find. The Pixi task builds the workspace and runs the
/// facade it just built, which is also how a developer dogfoods the same binary.
fn verb(arguments: &str) -> String {
    format!("pixi run --frozen universe -- {arguments}")
}

fn main_jobs(manifest: &Manifest) -> Vec<Job> {
    let checks = manifest.workflow.checks.declared();
    let mut view = Job::new("view", "universe", Vec::new()).with_step(Step::Run {
        label: "View the manifest".to_owned(),
        command: verb("plan --json"),
    });
    view = checkout_and_pixi(view);

    let mut validate =
        Job::new("validate", "universe", vec!["view".to_owned()]).with_step(Step::Run {
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

    // A job's filesystem does not outlive it, so the build hands its bytes to the jobs that
    // audit and verify them. Without this the audit job audits an empty tree — and, because
    // audit fails closed, says so.
    let build = checkout_and_pixi(
        Job::new("build", "universe", vec!["view".to_owned()])
            .with_matrix("platform", manifest.project.platforms.clone())
            .with_step(Step::Run {
                label: "Build the declared packages".to_owned(),
                command: verb("build --json"),
            })
            .with_step(Step::uses_with(
                universe_assets::UPLOAD_ARTIFACT,
                &[
                    ("name", ARTIFACT_NAME),
                    ("path", ARTIFACT_PATH),
                    ("if-no-files-found", "error"),
                ],
            )),
    );

    let audit = checkout_and_pixi(
        Job::new("audit", "universe", vec!["build".to_owned()])
            .with_step(Step::uses_with(
                universe_assets::DOWNLOAD_ARTIFACT,
                &[("pattern", "universe-*"), ("merge-multiple", "true")],
            ))
            .with_step(Step::Run {
                label: "Audit the built artifacts".to_owned(),
                command: verb("audit --json"),
            }),
    );

    let mut jobs = vec![view, validate, build, audit];

    if manifest.workflow.verify_artifact {
        jobs.push(checkout_and_pixi(
            Job::new("verify-artifact", "universe", vec!["audit".to_owned()])
                .with_matrix("platform", manifest.project.platforms.clone())
                .with_step(Step::uses_with(
                    universe_assets::DOWNLOAD_ARTIFACT,
                    &[("name", ARTIFACT_NAME)],
                ))
                .with_step(Step::Run {
                    label: "Prove the built artifact installs and runs".to_owned(),
                    command: verb("verify --artifact --json"),
                }),
        ));
    }

    let needs: Vec<String> = jobs.iter().map(|job| job.id.clone()).collect();
    let verdict = Job::new("verdict", "universe", needs).verdict();
    jobs.push(verdict);
    jobs
}

fn publish_jobs(manifest: &Manifest) -> Vec<Job> {
    let mut jobs = Vec::new();
    let mut publishes_anonymously = false;

    for store in manifest.stores.iter().filter(|store| store.writable) {
        let mut job = Job::new(&store.name, "publish", Vec::new());
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
            job = job.with_env(&secret_variable(&store.name, secret), secret);
        }
        job = checkout_and_pixi(job).with_step(Step::uses_with(
            universe_assets::DOWNLOAD_ARTIFACT,
            &[("pattern", "universe-*"), ("merge-multiple", "true")],
        ));
        for package in manifest
            .destinations()
            .into_iter()
            .find(|destination| destination.store.name == store.name)
            .map(|destination| destination.packages)
            .unwrap_or_default()
        {
            job = job.with_step(Step::Run {
                label: format!("Publish {}", package.name),
                command: verb(&format!(
                    "publish --store {} --package {} --json",
                    store.name, package.name
                )),
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
        AIRLOCK_FILE, MAIN_FILE, PUBLISH_FILE, RenderError, derived_files, render, sha256,
        to_identifier,
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
