//! The project manifest: one `universe.toml` that says what is built and where it goes.
//!
//! The manifest is split into facets rather than one target list, because a single
//! package can publish to two destinations and a single destination can receive two
//! kinds of package. `[[packages]]` describes what is built; `[[stores]]` describes
//! where bytes go and what the publisher must prove about them. A package names the
//! stores it publishes to **by name**, never by kind, so a store is an act with its own
//! credentials rather than a lookup ([`decision-127`]).
//!
//! Everything here is data. The renderer in `universe-workflow` derives jobs from these
//! records, which is why the validation below is strict about the shape of the mapping
//! and silent about how a job is written.
//!
//! Two rules the code enforces rather than documents:
//!
//! - **Kinds are compatible or the manifest is wrong.** A `conda` package may not name
//!   an `npmjs` store, and that is decided here rather than at publish time.
//! - **`releases` is the only implicit destination.** Every package is released as a
//!   GitHub Release asset unless it says `release = false`; every other destination
//!   must be named, because naming it is what makes it reviewable.
//!
//! [`decision-127`]: https://github.com/Archont561/universe/blob/main/backlog/decisions/decision-127%20-%20Manifest-facets-packages-stores-and-airlock-scopes.md

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::diagnostic::{Diagnostic, Severity};

/// Version of the `universe.toml` shape this module reads.
pub const MANIFEST_SCHEMA_VERSION: u32 = 1;

/// Platforms a Conda-shaped store understands, and the neutral platform vocabulary.
///
/// A manifest naming a platform outside this set is refused rather than carried
/// forward, because the renderer would turn it into a matrix entry no runner has.
pub const KNOWN_PLATFORMS: [&str; 6] = [
    "linux-64",
    "linux-aarch64",
    "win-64",
    "osx-64",
    "osx-arm64",
    "noarch",
];

/// The whole project manifest.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    /// Shape version. A file written for a newer shape is refused, not guessed at.
    pub schema: u32,
    /// Who this repository is.
    pub project: Project,
    /// How the declared commands are invoked.
    #[serde(default)]
    pub build: Build,
    /// What the artifact audit enforces.
    #[serde(default)]
    pub audit: Audit,
    /// What is built.
    #[serde(default)]
    pub packages: Vec<Package>,
    /// Where bytes go.
    #[serde(default)]
    pub stores: Vec<Store>,
    /// Release scoping, when this project cuts releases.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub release: Option<Release>,
    /// Airlock scopes, split because they are two provenances rather than one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub airlock: Option<Airlock>,
    /// What the renderer may emit.
    #[serde(default)]
    pub workflow: Workflow,
}

/// Repository identity and the platform set the project targets.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Project {
    /// Slug used in generated files and in the catalog.
    pub name: String,
    /// One-line description.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Repository URL, when the project is hosted.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub repository: Option<String>,
    /// Platforms this project builds for; non-empty.
    #[serde(default)]
    pub platforms: Vec<String>,
}

/// How declared build commands are invoked.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Build {
    /// Channels consulted in order by `plan`, `build`, and `test`.
    #[serde(default = "default_channels")]
    pub channels: Vec<String>,
    /// Whether an already-published build is skipped.
    #[serde(default)]
    pub skip_existing: SkipExisting,
}

impl Default for Build {
    fn default() -> Self {
        Self {
            channels: default_channels(),
            skip_existing: SkipExisting::default(),
        }
    }
}

fn default_channels() -> Vec<String> {
    vec!["conda-forge".to_owned()]
}

/// What to do when a build already exists upstream.
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum SkipExisting {
    /// Skip anything already present in any configured channel.
    #[default]
    All,
    /// Skip only what this repository already published.
    Local,
    /// Never skip.
    None,
}

/// Artifact audit rules. Every flag defaults to the strict value.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Audit {
    /// Refuse an artifact whose `about.json` names a licence file it does not bundle.
    #[serde(default = "default_true")]
    pub enforce_license_file: bool,
    /// Refuse an artifact whose licence is not a valid SPDX expression.
    #[serde(default = "default_true")]
    pub require_valid_spdx: bool,
    /// Refuse an artifact whose binaries embed an absolute build root.
    #[serde(default = "default_true")]
    pub forbid_build_path_leak: bool,
    /// Refuse an artifact whose binaries do not match the target architecture.
    #[serde(default = "default_true")]
    pub verify_arch_headers: bool,
}

impl Default for Audit {
    fn default() -> Self {
        Self {
            enforce_license_file: true,
            require_valid_spdx: true,
            forbid_build_path_leak: true,
            verify_arch_headers: true,
        }
    }
}

const fn default_true() -> bool {
    true
}

/// One thing the project builds.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Package {
    /// Package name as its ecosystem spells it.
    pub name: String,
    /// Which protocol this package speaks.
    pub kind: PackageKind,
    /// External command that builds it. Empty means the artifact is supplied by other means.
    #[serde(default)]
    pub command: Vec<String>,
    /// Environment the command runs with. Values are literals, never secret references.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub env: BTreeMap<String, String>,
    /// Platforms this package is built for; empty means every project platform.
    #[serde(default)]
    pub platforms: Vec<String>,
    /// Packages that must be built before this one.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub depends_on: Vec<String>,
    /// Globs, relative to the declared output directory, naming what the build produced.
    #[serde(default)]
    pub outputs: Vec<String>,
    /// Commands that prove an installed artifact works.
    ///
    /// Mandatory when the package publishes anywhere: a package with no smoke command
    /// is a package whose installability nobody checked.
    #[serde(default)]
    pub smoke: Vec<String>,
    /// Store names this package publishes to, in publish order.
    #[serde(default)]
    pub publish: Vec<String>,
    /// Whether the implicit GitHub Release destination applies.
    #[serde(default = "default_true")]
    pub release: bool,
    /// Authored discovery metadata, so the catalog works without a network lookup.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub catalog: Option<CatalogFacet>,
}

/// The package protocols Universe speaks.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum PackageKind {
    /// A Conda package (`.conda`).
    Conda,
    /// A Python wheel.
    Pypi,
    /// An npm tarball.
    Npm,
    /// A crate archive.
    Cargo,
    /// An R source or binary package.
    R,
}

impl PackageKind {
    /// Stable lowercase spelling, as the manifest and diagnostics write it.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Conda => "conda",
            Self::Pypi => "pypi",
            Self::Npm => "npm",
            Self::Cargo => "cargo",
            Self::R => "r",
        }
    }
}

/// Discovery metadata authored in the manifest rather than fetched at render time.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CatalogFacet {
    /// Human title shown in the catalog.
    pub title: String,
    /// One-line summary.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    /// SPDX licence expression.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub license: Option<String>,
}

/// One place bytes can go.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Store {
    /// Name used by `[[packages]] publish` lists. Two stores of one kind stay distinct.
    pub name: String,
    /// Which destination protocol this is.
    pub kind: StoreKind,
    /// Whether the release lane publishes here.
    #[serde(default = "default_true")]
    pub writable: bool,
    /// Whether an index must be generated for this destination.
    #[serde(default)]
    pub needs_index: bool,
    /// What a *consumer* needs to read it.
    #[serde(default)]
    pub read_auth: ReadAuth,
    /// How the *publisher* authenticates.
    #[serde(default)]
    pub auth: StoreAuth,
    /// Which job proves the bytes arrived: this one, or a separate reader.
    #[serde(default)]
    pub verifier: Verifier,
    /// Secret *names*, never values; the renderer emits the reference.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub secrets: Vec<String>,
    /// Channel or owner path, for stores addressed that way.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub channel: Option<String>,
    /// Endpoint, for stores addressed by URL.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
}

/// Destination protocols.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum StoreKind {
    /// A static channel tree Universe generates an index for.
    Static,
    /// GitHub Releases assets.
    Releases,
    /// A git branch holding package files.
    Branch,
    /// A hosted Conda channel on prefix.dev.
    PrefixDev,
    /// Anaconda.org.
    Anaconda,
    /// A self-hosted Quetz instance.
    Quetz,
    /// JFrog Artifactory.
    Artifactory,
    /// An S3 bucket.
    S3,
    /// A local directory.
    Filesystem,
    /// npmjs.
    Npmjs,
    /// GitHub Packages.
    GithubPackages,
    /// PyPI.
    Pypi,
    /// crates.io.
    CratesIo,
    /// A declared external command, the escape hatch.
    Command,
}

impl StoreKind {
    /// Stable lowercase spelling.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Static => "static",
            Self::Releases => "releases",
            Self::Branch => "branch",
            Self::PrefixDev => "prefix-dev",
            Self::Anaconda => "anaconda",
            Self::Quetz => "quetz",
            Self::Artifactory => "artifactory",
            Self::S3 => "s3",
            Self::Filesystem => "filesystem",
            Self::Npmjs => "npmjs",
            Self::GithubPackages => "github-packages",
            Self::Pypi => "pypi",
            Self::CratesIo => "crates-io",
            Self::Command => "command",
        }
    }

    /// Whether this destination generates an index Universe has to produce.
    #[must_use]
    pub const fn generates_index(self) -> bool {
        matches!(self, Self::Static | Self::Branch | Self::PrefixDev)
    }

    /// Whether this destination accepts a package of `kind`.
    ///
    /// This is the compatibility half of `decision-127` rule 3: the manifest is wrong,
    /// rather than the publish failing halfway through an upload.
    #[must_use]
    pub const fn accepts(self, kind: PackageKind) -> bool {
        match kind {
            PackageKind::Conda => matches!(
                self,
                Self::Static
                    | Self::PrefixDev
                    | Self::Anaconda
                    | Self::Quetz
                    | Self::Artifactory
                    | Self::S3
                    | Self::Filesystem
                    | Self::Branch
                    | Self::Releases
            ),
            PackageKind::Pypi => matches!(
                self,
                Self::Pypi
                    | Self::Static
                    | Self::Artifactory
                    | Self::S3
                    | Self::Filesystem
                    | Self::Branch
                    | Self::Releases
            ),
            // No hosted-store path for a wrapper crate yet, so `npm` is not accepted here.
            PackageKind::Npm => matches!(self, Self::Npmjs | Self::GithubPackages | Self::Releases),
            PackageKind::Cargo => {
                matches!(self, Self::CratesIo | Self::Npmjs | Self::Releases)
            }
            PackageKind::R => matches!(
                self,
                Self::Static
                    | Self::Artifactory
                    | Self::S3
                    | Self::Filesystem
                    | Self::Branch
                    | Self::Releases
            ),
        }
    }
}

/// What a consumer needs to read a store.
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ReadAuth {
    /// Anonymous reads.
    #[default]
    None,
    /// A token is required to read.
    Token,
}

/// How the publisher authenticates.
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum StoreAuth {
    /// A long-lived secret named in `secrets`.
    #[default]
    Token,
    /// A short-lived token exchanged from the workflow identity.
    Oidc,
}

/// Which job proves the bytes arrived.
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Verifier {
    /// Read the artifact back from the file store.
    #[default]
    File,
    /// Read the registry's own record back.
    Registry,
}

/// Release scoping.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Release {
    /// Glob matching the tags that cut a release.
    #[serde(default = "default_tag_pattern")]
    pub tag_pattern: String,
    /// Changelog path, when the project keeps one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub changelog: Option<String>,
    /// Named scopes that narrow the destination set.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub scope: Vec<ReleaseScope>,
}

fn default_tag_pattern() -> String {
    "v*".to_owned()
}

/// A named release scope.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ReleaseScope {
    /// Scope name.
    pub name: String,
    /// Store names this scope publishes to.
    #[serde(default)]
    pub stores: Vec<String>,
}

/// Airlock scopes. Two provenances, deliberately not mergeable.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Airlock {
    /// Restoring my own toolchain, from my own pins.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub developer: Option<AirlockDeveloper>,
    /// Consuming a catalog someone else published.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub delivery: Option<AirlockDelivery>,
}

/// The developer scope.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AirlockDeveloper {
    /// Lock formats this scope can replay.
    #[serde(default)]
    pub adapters: Vec<LockAdapter>,
    /// Branch prefix the mirror transport writes to.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mirror: Option<String>,
}

/// The delivery scope.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AirlockDelivery {
    /// Lock formats this scope can replay.
    #[serde(default)]
    pub adapters: Vec<LockAdapter>,
    /// `github:<owner>/<repo>@<commit>`: a pinned catalog, never a tag or branch head.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub catalog: Option<String>,
}

/// A lock format Airlock can replay.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum LockAdapter {
    /// `pixi.lock`.
    Pixi,
    /// `bun.lock`.
    Bun,
    /// `Cargo.lock`.
    Cargo,
}

/// The bounded surface the renderer may emit.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Workflow {
    /// The closed static-check vocabulary: three slots, one command each.
    #[serde(default)]
    pub checks: Checks,
    /// Whether the build-lane verification range is active.
    #[serde(default = "default_true")]
    pub verify_artifact: bool,
    /// Whether the published-lane verification range is active.
    #[serde(default = "default_true")]
    pub verify_published: bool,
}

impl Default for Workflow {
    // The derived `Default` would switch both verification ranges *off*, which is the
    // permissive direction a missing `[workflow]` block must never take.
    fn default() -> Self {
        Self {
            checks: Checks::default(),
            verify_artifact: true,
            verify_published: true,
        }
    }
}

/// The three static-check slots. An unknown slot is a manifest error.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Checks {
    /// Formatting command.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub format: Option<String>,
    /// Lint command.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lint: Option<String>,
    /// Type-check command.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub typecheck: Option<String>,
}

impl Checks {
    /// The declared slots, in declaration order, as `(slot, command)`.
    ///
    /// The renderer emits one step per entry, so the order here is the order in
    /// the rendered `validate` job.
    #[must_use]
    pub fn declared(&self) -> Vec<(&'static str, &str)> {
        [
            ("format", self.format.as_deref()),
            ("lint", self.lint.as_deref()),
            ("typecheck", self.typecheck.as_deref()),
        ]
        .into_iter()
        .filter_map(|(slot, command)| command.map(|command| (slot, command)))
        .collect()
    }
}

/// One row of the derived store→package mapping.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Destination<'a> {
    /// The store.
    pub store: &'a Store,
    /// Packages that named it, in declaration order.
    pub packages: Vec<&'a Package>,
}

impl Manifest {
    /// Read a manifest from TOML.
    ///
    /// Three things can go wrong and they are reported differently: the file is not
    /// TOML, the file does not match the shape, or the shape is valid but the mapping
    /// it declares is not. The last one collects every finding, because fixing a
    /// manifest one error per run is a worse experience than reading a list.
    ///
    /// # Errors
    ///
    /// Returns every error-level diagnostic found, including a parse or shape failure
    /// as its single entry.
    pub fn from_toml(text: &str) -> Result<Self, Vec<Diagnostic>> {
        let document = match text.parse::<toml_edit::DocumentMut>() {
            Ok(document) => document,
            Err(error) => {
                return Err(vec![Diagnostic::new(
                    Severity::Error,
                    "manifest.toml.invalid",
                    format!("manifest is not valid TOML: {error}"),
                )]);
            }
        };
        let value = item_to_json(document.as_item()).map_err(|message| {
            vec![Diagnostic::new(
                Severity::Error,
                "manifest.toml.unsupported_value",
                message,
            )]
        })?;
        let manifest: Self = serde_json::from_value(value).map_err(|error| {
            vec![
                Diagnostic::new(
                    Severity::Error,
                    "manifest.shape.invalid",
                    format!("manifest does not match schema {MANIFEST_SCHEMA_VERSION}: {error}"),
                )
                .with_path(error_path(&error)),
            ]
        })?;
        let diagnostics = manifest.validate();
        if diagnostics.iter().any(Diagnostic::is_error) {
            return Err(diagnostics);
        }
        Ok(manifest)
    }

    /// Every problem with this manifest, in declaration order.
    ///
    /// Findings are collected rather than short-circuited so one run reports the whole
    /// list; the severities are part of the contract, because a dead destination is a
    /// warning while a package naming it wrongly is an error.
    #[must_use]
    pub fn validate(&self) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();
        self.validate_project(&mut diagnostics);
        self.validate_stores(&mut diagnostics);
        self.validate_packages(&mut diagnostics);
        self.validate_release(&mut diagnostics);
        self.validate_workflow(&mut diagnostics);
        diagnostics
    }

    fn validate_workflow(&self, diagnostics: &mut Vec<Diagnostic>) {
        for (index, (slot, command)) in self.workflow.checks.declared().iter().enumerate() {
            if command.trim().is_empty() {
                diagnostics.push(
                    Diagnostic::new(
                        Severity::Error,
                        "manifest.workflow.check.empty",
                        format!("workflow check '{slot}' is declared with no command"),
                    )
                    .with_path(format!("workflow.checks[{index}]")),
                );
            }
        }
    }

    fn validate_project(&self, diagnostics: &mut Vec<Diagnostic>) {
        if self.schema != MANIFEST_SCHEMA_VERSION {
            diagnostics.push(Diagnostic::new(
                Severity::Error,
                "manifest.schema.unsupported",
                format!(
                    "manifest schema {} is not the supported schema {MANIFEST_SCHEMA_VERSION}",
                    self.schema
                ),
            ));
        }
        if self.project.name.trim().is_empty() {
            diagnostics.push(
                Diagnostic::new(
                    Severity::Error,
                    "manifest.project.name.empty",
                    "project name must not be empty",
                )
                .with_path("project.name"),
            );
        }
        match &self.project.repository {
            Some(repository) if !repository.starts_with("https://") => {
                diagnostics.push(
                    Diagnostic::new(
                        Severity::Error,
                        "manifest.project.repository.invalid",
                        "project repository must be an https:// URL",
                    )
                    .with_path("project.repository"),
                );
            }
            _ => {}
        }
        if self.project.platforms.is_empty() {
            diagnostics.push(
                Diagnostic::new(
                    Severity::Error,
                    "manifest.project.platforms.empty",
                    "project must declare at least one platform",
                )
                .with_path("project.platforms"),
            );
        }
        for (index, platform) in self.project.platforms.iter().enumerate() {
            if !KNOWN_PLATFORMS.contains(&platform.as_str()) {
                diagnostics.push(
                    Diagnostic::new(
                        Severity::Error,
                        "manifest.project.platform.unknown",
                        format!(
                            "platform '{platform}' is not one of {}",
                            KNOWN_PLATFORMS.join(", ")
                        ),
                    )
                    .with_path(format!("project.platforms[{index}]")),
                );
            }
        }
    }

    fn validate_stores(&self, diagnostics: &mut Vec<Diagnostic>) {
        for (index, store) in self.stores.iter().enumerate() {
            let path = format!("stores[{index}]");
            if store.name.trim().is_empty() {
                diagnostics.push(
                    Diagnostic::new(
                        Severity::Error,
                        "manifest.store.name.empty",
                        "store name must not be empty",
                    )
                    .with_path(format!("{path}.name")),
                );
            }
            let duplicate = self
                .stores
                .iter()
                .take(index)
                .any(|earlier| earlier.name == store.name && !store.name.trim().is_empty());
            if duplicate {
                diagnostics.push(
                    Diagnostic::new(
                        Severity::Error,
                        "manifest.store.name.duplicate",
                        format!("store '{}' is declared twice", store.name),
                    )
                    .with_path(format!("{path}.name")),
                );
            }
            if matches!(
                store.kind,
                StoreKind::PrefixDev
                    | StoreKind::Anaconda
                    | StoreKind::Quetz
                    | StoreKind::Artifactory
            ) && store
                .channel
                .as_deref()
                .unwrap_or_default()
                .trim()
                .is_empty()
            {
                diagnostics.push(
                    Diagnostic::new(
                        Severity::Error,
                        "manifest.store.channel.empty",
                        format!(
                            "store '{}' of kind '{}' must declare a channel",
                            store.name,
                            store.kind.as_str()
                        ),
                    )
                    .with_path(format!("{path}.channel")),
                );
            }
            for (secret_index, secret) in store.secrets.iter().enumerate() {
                if secret.trim().is_empty() {
                    diagnostics.push(
                        Diagnostic::new(
                            Severity::Error,
                            "manifest.store.secret.empty",
                            "a declared secret name must not be empty",
                        )
                        .with_path(format!("{path}.secrets[{secret_index}]")),
                    );
                }
            }
            if store.writable && store.auth == StoreAuth::Token && store.secrets.is_empty() {
                diagnostics.push(
                    Diagnostic::new(
                        Severity::Warning,
                        "manifest.store.secret.missing",
                        format!(
                            "store '{}' publishes with a token but names no secret",
                            store.name
                        ),
                    )
                    .with_path(format!("{path}.secrets")),
                );
            }
            if !store.writable
                && self
                    .packages
                    .iter()
                    .any(|package| package.publish.iter().any(|name| name == &store.name))
            {
                diagnostics.push(
                    Diagnostic::new(
                        Severity::Error,
                        "manifest.package.publish.store_read_only",
                        format!(
                            "store '{}' is declared read-only but a package publishes to it",
                            store.name
                        ),
                    )
                    .with_path(format!("{path}.writable")),
                );
            }
            let named = self
                .packages
                .iter()
                .any(|package| package.publish.iter().any(|name| name == &store.name));
            if store.writable && !named && !store.name.trim().is_empty() {
                diagnostics.push(
                    Diagnostic::new(
                        Severity::Warning,
                        "manifest.store.dead",
                        format!(
                            "store '{}' is writable but no package publishes to it",
                            store.name
                        ),
                    )
                    .with_path(format!("{path}.name")),
                );
            }
        }
    }

    fn validate_packages(&self, diagnostics: &mut Vec<Diagnostic>) {
        for (index, package) in self.packages.iter().enumerate() {
            let path = format!("packages[{index}]");
            if package.name.trim().is_empty() {
                diagnostics.push(
                    Diagnostic::new(
                        Severity::Error,
                        "manifest.package.name.empty",
                        "package name must not be empty",
                    )
                    .with_path(format!("{path}.name")),
                );
            }
            if self
                .packages
                .iter()
                .take(index)
                .any(|earlier| earlier.name == package.name && !package.name.trim().is_empty())
            {
                diagnostics.push(
                    Diagnostic::new(
                        Severity::Error,
                        "manifest.package.name.duplicate",
                        format!("package '{}' is declared twice", package.name),
                    )
                    .with_path(format!("{path}.name")),
                );
            }
            for (platform_index, platform) in package.platforms.iter().enumerate() {
                if !KNOWN_PLATFORMS.contains(&platform.as_str()) {
                    diagnostics.push(
                        Diagnostic::new(
                            Severity::Error,
                            "manifest.package.platform.unknown",
                            format!("platform '{platform}' is not a known platform"),
                        )
                        .with_path(format!("{path}.platforms[{platform_index}]")),
                    );
                }
            }
            if package.command.is_empty() && package.outputs.is_empty() {
                diagnostics.push(
                    Diagnostic::new(
                        Severity::Warning,
                        "manifest.package.command.missing",
                        format!(
                            "package '{}' declares no command and no outputs, so nothing builds it",
                            package.name
                        ),
                    )
                    .with_path(format!("{path}.command")),
                );
            }
            let publishes_somewhere = package.release || !package.publish.is_empty();
            if publishes_somewhere && package.smoke.is_empty() {
                diagnostics.push(
                    Diagnostic::new(
                        Severity::Error,
                        "manifest.package.smoke.missing",
                        format!(
                            "package '{}' publishes somewhere but declares no smoke command",
                            package.name
                        ),
                    )
                    .with_path(format!("{path}.smoke")),
                );
            }
            for (publish_index, name) in package.publish.iter().enumerate() {
                match self.stores.iter().find(|store| &store.name == name) {
                    None => diagnostics.push(
                        Diagnostic::new(
                            Severity::Error,
                            "manifest.package.publish.store_unknown",
                            format!(
                                "package '{}' publishes to unknown store '{name}'",
                                package.name
                            ),
                        )
                        .with_path(format!("{path}.publish[{publish_index}]")),
                    ),
                    Some(store) if !store.kind.accepts(package.kind) => {
                        diagnostics.push(
                            Diagnostic::new(
                                Severity::Error,
                                "manifest.package.publish.incompatible",
                                format!(
                                    "a '{}' package cannot publish to store '{}' of kind '{}'",
                                    package.kind.as_str(),
                                    store.name,
                                    store.kind.as_str()
                                ),
                            )
                            .with_path(format!("{path}.publish[{publish_index}]")),
                        );
                    }
                    Some(_) => {}
                }
            }
            let duplicate_publish = has_duplicates(&package.publish);
            if duplicate_publish {
                diagnostics.push(
                    Diagnostic::new(
                        Severity::Error,
                        "manifest.package.publish.duplicate",
                        format!("package '{}' names a store twice", package.name),
                    )
                    .with_path(format!("{path}.publish")),
                );
            }
            for (dependency_index, dependency) in package.depends_on.iter().enumerate() {
                if !self
                    .packages
                    .iter()
                    .any(|candidate| &candidate.name == dependency)
                {
                    diagnostics.push(
                        Diagnostic::new(
                            Severity::Error,
                            "manifest.package.dependency.unknown",
                            format!(
                                "package '{}' depends on unknown package '{dependency}'",
                                package.name
                            ),
                        )
                        .with_path(format!("{path}.depends_on[{dependency_index}]")),
                    );
                }
            }
        }
        diagnostics.extend(self.cycle_diagnostics());
    }

    fn cycle_diagnostics(&self) -> Vec<Diagnostic> {
        // A dependency cycle cannot be ordered, so it is refused here rather than
        // discovered by the planner halfway through a release.
        let mut diagnostics = Vec::new();
        for (index, package) in self.packages.iter().enumerate() {
            let mut stack = vec![package.name.as_str()];
            let mut visited = Vec::new();
            while let Some(current) = stack.pop() {
                if visited.contains(&current) {
                    continue;
                }
                visited.push(current);
                if let Some(node) = self.packages.iter().find(|node| node.name == current) {
                    for dependency in &node.depends_on {
                        if dependency == &package.name {
                            diagnostics.push(
                                Diagnostic::new(
                                    Severity::Error,
                                    "manifest.package.dependency.cycle",
                                    format!(
                                        "package '{}' is part of a dependency cycle",
                                        package.name
                                    ),
                                )
                                .with_path(format!("packages[{index}].depends_on")),
                            );
                            stack.clear();
                            break;
                        }
                        stack.push(dependency.as_str());
                    }
                }
            }
        }
        diagnostics
    }

    fn validate_release(&self, diagnostics: &mut Vec<Diagnostic>) {
        let Some(release) = &self.release else {
            return;
        };
        if release.tag_pattern.trim().is_empty() {
            diagnostics.push(
                Diagnostic::new(
                    Severity::Error,
                    "manifest.release.tag_pattern.empty",
                    "release tag pattern must not be empty",
                )
                .with_path("release.tag_pattern"),
            );
        }
        for (scope_index, scope) in release.scope.iter().enumerate() {
            for (store_index, name) in scope.stores.iter().enumerate() {
                if !self.stores.iter().any(|store| &store.name == name) {
                    diagnostics.push(
                        Diagnostic::new(
                            Severity::Error,
                            "manifest.release.scope.store_unknown",
                            format!(
                                "release scope '{}' names unknown store '{name}'",
                                scope.name
                            ),
                        )
                        .with_path(format!(
                            "release.scope[{scope_index}].stores[{store_index}]"
                        )),
                    );
                }
            }
        }
    }

    /// The store→package mapping this manifest declares.
    ///
    /// Written as a derived view rather than inferred by the renderer, because a mapping
    /// nobody prints is a mapping nobody checks.
    #[must_use]
    pub fn destinations(&self) -> Vec<Destination<'_>> {
        self.stores
            .iter()
            .map(|store| Destination {
                store,
                packages: self
                    .packages
                    .iter()
                    .filter(|package| package.publish.iter().any(|name| name == &store.name))
                    .collect(),
            })
            .collect()
    }

    /// Packages that reach GitHub Releases implicitly.
    #[must_use]
    pub fn released_packages(&self) -> Vec<&Package> {
        self.packages
            .iter()
            .filter(|package| package.release)
            .collect()
    }

    /// Platforms this project builds, once package overrides are applied.
    #[must_use]
    pub fn platforms_for<'a>(&'a self, package: &'a Package) -> Vec<&'a str> {
        if package.platforms.is_empty() {
            self.project.platforms.iter().map(String::as_str).collect()
        } else {
            package.platforms.iter().map(String::as_str).collect()
        }
    }

    /// Every lock adapter any Airlock scope declares.
    #[must_use]
    pub fn lock_adapters(&self) -> Vec<LockAdapter> {
        let mut adapters = Vec::new();
        if let Some(airlock) = &self.airlock {
            for adapter in airlock
                .developer
                .iter()
                .flat_map(|scope| scope.adapters.iter())
                .chain(
                    airlock
                        .delivery
                        .iter()
                        .flat_map(|scope| scope.adapters.iter()),
                )
            {
                if !adapters.contains(adapter) {
                    adapters.push(*adapter);
                }
            }
        }
        adapters
    }

    /// Serialize back to JSON, which is how the renderer receives a parsed manifest.
    ///
    /// # Errors
    ///
    /// Returns the serializer error when the record cannot be represented as JSON.
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }
}

impl LockAdapter {
    /// Stable lowercase spelling.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Pixi => "pixi",
            Self::Bun => "bun",
            Self::Cargo => "cargo",
        }
    }
}

/// Version of the pinned release manifest shape (`universe-release.json`).
pub const RELEASE_MANIFEST_SCHEMA_VERSION: u32 = 1;

/// The pinned release manifest: what one release published, by digest.
///
/// It is written after publication and read before any restore, so it names bytes and
/// digests rather than jobs and steps. A manifest that described the pipeline would be
/// useless to the consumer who has to prove it received what the pipeline claimed.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ReleaseManifest {
    /// Shape version.
    pub schema: u32,
    /// Version this release publishes, from the release root.
    pub version: String,
    /// Commit the release was cut from.
    pub commit: String,
    /// Packages in the release.
    pub packages: Vec<ReleasedPackage>,
}

/// One package in a pinned release.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ReleasedPackage {
    /// Package name.
    pub name: String,
    /// Protocol this package speaks.
    pub kind: PackageKind,
    /// Package version.
    pub version: String,
    /// Payloads it published.
    pub artifacts: Vec<ReleasedArtifact>,
}

/// One published payload.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ReleasedArtifact {
    /// File name as published, used to construct the static path.
    pub filename: String,
    /// SHA-256 of the payload.
    pub sha256: crate::digest::Sha256Digest,
    /// Size in bytes.
    pub size: u64,
    /// Store names that received these bytes.
    #[serde(default)]
    pub stores: Vec<String>,
}

impl ReleaseManifest {
    /// Problems with this release manifest, in field order.
    #[must_use]
    pub fn validate(&self) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();
        if self.schema != RELEASE_MANIFEST_SCHEMA_VERSION {
            diagnostics.push(Diagnostic::new(
                Severity::Error,
                "release.schema.unsupported",
                format!(
                    "release schema {} is not the supported schema {RELEASE_MANIFEST_SCHEMA_VERSION}",
                    self.schema
                ),
            ));
        }
        for (field, value) in [("version", &self.version), ("commit", &self.commit)] {
            if value.trim().is_empty() {
                diagnostics.push(
                    Diagnostic::new(
                        Severity::Error,
                        "release.field.empty",
                        format!("release {field} must not be empty"),
                    )
                    .with_path(field.to_owned()),
                );
            }
        }
        if self.packages.is_empty() {
            diagnostics.push(Diagnostic::new(
                Severity::Error,
                "release.packages.empty",
                "a release manifest must name at least one package",
            ));
        }
        for (package_index, package) in self.packages.iter().enumerate() {
            let path = format!("packages[{package_index}]");
            for (field, value) in [("name", &package.name), ("version", &package.version)] {
                if value.trim().is_empty() {
                    diagnostics.push(
                        Diagnostic::new(
                            Severity::Error,
                            "release.field.empty",
                            format!("released package {field} must not be empty"),
                        )
                        .with_path(format!("{path}.{field}")),
                    );
                }
            }
            if package.artifacts.is_empty() {
                diagnostics.push(
                    Diagnostic::new(
                        Severity::Error,
                        "release.artifacts.empty",
                        format!("released package '{}' names no artifact", package.name),
                    )
                    .with_path(format!("{path}.artifacts")),
                );
            }
            for (artifact_index, artifact) in package.artifacts.iter().enumerate() {
                if artifact.filename.trim().is_empty() {
                    diagnostics.push(
                        Diagnostic::new(
                            Severity::Error,
                            "release.artifact.filename.empty",
                            "a released artifact must name its file",
                        )
                        .with_path(format!("{path}.artifacts[{artifact_index}].filename")),
                    );
                }
                if artifact.size == 0 {
                    diagnostics.push(
                        Diagnostic::new(
                            Severity::Error,
                            "release.artifact.size.zero",
                            format!(
                                "released artifact '{}' declares a zero size",
                                artifact.filename
                            ),
                        )
                        .with_path(format!("{path}.artifacts[{artifact_index}].size")),
                    );
                }
                if artifact.stores.is_empty() {
                    diagnostics.push(
                        Diagnostic::new(
                            Severity::Error,
                            "release.artifact.stores.empty",
                            format!(
                                "released artifact '{}' names no destination",
                                artifact.filename
                            ),
                        )
                        .with_path(format!("{path}.artifacts[{artifact_index}].stores")),
                    );
                }
            }
        }
        diagnostics
    }

    /// Read a release manifest from JSON.
    ///
    /// # Errors
    ///
    /// Returns the shape error when the document does not match the schema.
    pub fn from_json(text: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(text)
    }

    /// Serialize to compact JSON.
    ///
    /// # Errors
    ///
    /// Returns the serializer error when the record cannot be represented as JSON.
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }
}

fn has_duplicates(values: &[String]) -> bool {
    values
        .iter()
        .enumerate()
        .any(|(index, value)| values.iter().skip(index + 1).any(|later| later == value))
}

fn error_path(error: &serde_json::Error) -> String {
    // serde's message names the offending field but not its path; the line it reports is
    // a line in the intermediate JSON, which would be worse than saying nothing.
    let message = error.to_string();
    message
        .split("unknown field `")
        .nth(1)
        .and_then(|rest| rest.split('`').next())
        .map_or_else(String::new, str::to_owned)
}

fn item_to_json(item: &toml_edit::Item) -> Result<serde_json::Value, String> {
    match item {
        toml_edit::Item::None => Ok(serde_json::Value::Null),
        toml_edit::Item::Value(value) => value_to_json(value),
        toml_edit::Item::Table(table) => {
            let mut object = serde_json::Map::new();
            for (key, value) in table {
                object.insert(key.to_owned(), item_to_json(value)?);
            }
            Ok(serde_json::Value::Object(object))
        }
        toml_edit::Item::ArrayOfTables(tables) => {
            let mut array = Vec::with_capacity(tables.len());
            for table in tables {
                let mut object = serde_json::Map::new();
                for (key, value) in table {
                    object.insert(key.to_owned(), item_to_json(value)?);
                }
                array.push(serde_json::Value::Object(object));
            }
            Ok(serde_json::Value::Array(array))
        }
    }
}

fn value_to_json(value: &toml_edit::Value) -> Result<serde_json::Value, String> {
    match value {
        toml_edit::Value::String(text) => Ok(serde_json::Value::String(text.value().clone())),
        toml_edit::Value::Integer(number) => Ok(serde_json::Value::from(*number.value())),
        toml_edit::Value::Float(number) => serde_json::Number::from_f64(*number.value())
            .map(serde_json::Value::Number)
            .ok_or_else(|| format!("float {} has no JSON representation", number.value())),
        toml_edit::Value::Boolean(flag) => Ok(serde_json::Value::Bool(*flag.value())),
        toml_edit::Value::Datetime(datetime) => {
            Ok(serde_json::Value::String(datetime.value().to_string()))
        }
        toml_edit::Value::Array(array) => {
            let mut values = Vec::with_capacity(array.len());
            for item in array {
                values.push(value_to_json(item)?);
            }
            Ok(serde_json::Value::Array(values))
        }
        toml_edit::Value::InlineTable(table) => {
            let mut object = serde_json::Map::new();
            for (key, value) in table {
                object.insert(key.to_owned(), value_to_json(value)?);
            }
            Ok(serde_json::Value::Object(object))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        KNOWN_PLATFORMS, LockAdapter, MANIFEST_SCHEMA_VERSION, Manifest, PackageKind,
        RELEASE_MANIFEST_SCHEMA_VERSION, ReadAuth, ReleaseManifest, SkipExisting, StoreAuth,
        StoreKind, Verifier, has_duplicates,
    };
    use crate::diagnostic::Diagnostic;

    /// Codes of every diagnostic, in the order they were produced.
    fn codes(diagnostics: &[Diagnostic]) -> Vec<&str> {
        diagnostics.iter().map(Diagnostic::code).collect()
    }

    const MINIMAL: &str = r#"
schema = 1

[project]
name = "example"
platforms = ["linux-64"]

[[packages]]
name = "example-airlock"
kind = "conda"
command = ["cargo", "build"]
outputs = ["output/**/*.conda"]
smoke = ["example-airlock --version"]
publish = ["conda-prefix"]

[[stores]]
name = "conda-prefix"
kind = "prefix-dev"
channel = "someone/channel"
writable = true
auth = "oidc"
verifier = "registry"
"#;

    fn parsed(text: &str) -> Manifest {
        Manifest::from_toml(text).expect("manifest parses")
    }

    fn problems(text: &str) -> Vec<Diagnostic> {
        Manifest::from_toml(text).expect_err("manifest is refused")
    }

    /// Findings for a manifest that is accepted: warnings are not a refusal.
    fn findings(text: &str) -> Vec<Diagnostic> {
        parsed(text).validate()
    }

    #[test]
    fn a_minimal_manifest_parses_with_its_defaults() {
        let manifest = parsed(MINIMAL);
        assert_eq!(manifest.schema, MANIFEST_SCHEMA_VERSION);
        assert_eq!(manifest.project.name, "example");
        assert_eq!(manifest.project.platforms, ["linux-64"]);
        // Absent facets take their strict default rather than a permissive one.
        assert_eq!(manifest.build.channels, ["conda-forge"]);
        assert_eq!(manifest.build.skip_existing, SkipExisting::All);
        assert!(manifest.audit.enforce_license_file);
        assert!(manifest.workflow.verify_artifact);
        assert!(manifest.workflow.verify_published);
        assert_eq!(manifest.packages[0].kind, PackageKind::Conda);
        assert!(
            manifest.packages[0].release,
            "releases is the implicit default"
        );
        assert_eq!(manifest.stores[0].auth, StoreAuth::Oidc);
        assert_eq!(manifest.stores[0].verifier, Verifier::Registry);
        assert_eq!(manifest.stores[0].read_auth, ReadAuth::None);
    }

    #[test]
    fn text_that_is_not_toml_names_itself_as_the_problem() {
        let diagnostics = problems("schema = [unclosed");
        assert_eq!(codes(&diagnostics), ["manifest.toml.invalid"]);
    }

    #[test]
    fn an_unknown_field_is_refused_and_named() {
        let text = MINIMAL.replace("name = \"example\"", "name = \"example\"\nnme = \"typo\"");
        let diagnostics = problems(&text);
        assert_eq!(codes(&diagnostics), ["manifest.shape.invalid"]);
        assert_eq!(diagnostics[0].path(), Some("nme"));
    }

    #[test]
    fn a_newer_schema_is_refused_rather_than_guessed_at() {
        let text = MINIMAL.replace("schema = 1", "schema = 99");
        assert_eq!(codes(&problems(&text)), ["manifest.schema.unsupported"]);
    }

    #[test]
    fn project_fields_are_each_checked() {
        let text = MINIMAL
            .replace("name = \"example\"", "name = \" \"")
            .replace(
                "platforms = [\"linux-64\"]",
                "platforms = [\"linux-64\", \"linux-32\"]\nrepository = \"git@example.com:x/y\"",
            );
        let diagnostics = problems(&text);
        assert_eq!(
            codes(&diagnostics),
            [
                "manifest.project.name.empty",
                "manifest.project.repository.invalid",
                "manifest.project.platform.unknown",
            ]
        );
        assert_eq!(diagnostics[2].path(), Some("project.platforms[1]"));
    }

    #[test]
    fn a_project_without_platforms_is_refused() {
        let text = MINIMAL.replace("platforms = [\"linux-64\"]", "");
        assert_eq!(
            codes(&problems(&text)),
            ["manifest.project.platforms.empty"]
        );
    }

    #[test]
    fn every_known_platform_is_accepted() {
        let platforms = KNOWN_PLATFORMS
            .iter()
            .map(|platform| format!("\"{platform}\""))
            .collect::<Vec<_>>()
            .join(", ");
        let text = MINIMAL.replace("\"linux-64\"]", &format!("\"linux-64\", {platforms}]"));
        assert!(Manifest::from_toml(&text).is_ok());
    }

    #[test]
    fn a_store_that_publishes_with_a_token_must_name_a_secret() {
        let text = MINIMAL.replace("auth = \"oidc\"", "auth = \"token\"");
        let diagnostics = findings(&text);
        assert_eq!(codes(&diagnostics), ["manifest.store.secret.missing"]);
        assert!(!diagnostics[0].is_error(), "a missing secret is a warning");
    }

    #[test]
    fn store_identity_and_addresses_are_checked() {
        let text = MINIMAL.replace(
            "channel = \"someone/channel\"",
            "channel = \" \"\nsecrets = [\" \"]",
        );
        let diagnostics = problems(&text);
        assert_eq!(
            codes(&diagnostics),
            [
                "manifest.store.channel.empty",
                "manifest.store.secret.empty",
            ]
        );
    }

    #[test]
    fn two_stores_may_not_share_a_name() {
        let store = MINIMAL.rsplit_once("[[stores]]").expect("store block").1;
        let text = format!("{MINIMAL}\n[[stores]]{store}");
        assert_eq!(codes(&problems(&text)), ["manifest.store.name.duplicate"]);
    }

    #[test]
    fn an_unnamed_store_is_refused() {
        let text = MINIMAL.replace("name = \"conda-prefix\"", "name = \"\"");
        let diagnostics = problems(&text);
        assert!(
            codes(&diagnostics).contains(&"manifest.store.name.empty"),
            "{:?}",
            codes(&diagnostics)
        );
    }

    #[test]
    fn a_writable_store_nobody_publishes_to_is_a_dead_destination() {
        let text = MINIMAL.replace("publish = [\"conda-prefix\"]", "");
        let diagnostics = findings(&text);
        assert!(
            codes(&diagnostics).contains(&"manifest.store.dead"),
            "{:?}",
            codes(&diagnostics)
        );
        let dead = diagnostics
            .iter()
            .find(|diagnostic| diagnostic.code() == "manifest.store.dead")
            .expect("dead store");
        assert!(!dead.is_error(), "a dead destination is a warning");
    }

    #[test]
    fn a_read_only_store_may_not_receive_a_package() {
        let text = MINIMAL.replace("writable = true", "writable = false");
        assert_eq!(
            codes(&problems(&text)),
            ["manifest.package.publish.store_read_only"]
        );
    }

    #[test]
    fn package_identity_is_checked() {
        let text = MINIMAL
            .replace(
                "kind = \"conda\"",
                "kind = \"conda\"\nplatforms = [\"beos-68k\"]",
            )
            .replace("name = \"example-airlock\"", "name = \" \"");
        let diagnostics = problems(&text);
        assert_eq!(
            codes(&diagnostics),
            [
                "manifest.package.name.empty",
                "manifest.package.platform.unknown",
            ]
        );
    }

    #[test]
    fn two_packages_may_not_share_a_name() {
        let package = MINIMAL
            .split_once("[[packages]]")
            .and_then(|(_, rest)| rest.split_once("[[stores]]"))
            .expect("package block")
            .0;
        let text = format!("{MINIMAL}\n[[packages]]{package}");
        assert!(
            codes(&problems(&text)).contains(&"manifest.package.name.duplicate"),
            "{}",
            codes(&problems(&text)).join(", ")
        );
    }

    #[test]
    fn a_package_that_builds_nothing_is_a_warning() {
        let text = MINIMAL
            .replace("command = [\"cargo\", \"build\"]\n", "")
            .replace("outputs = [\"output/**/*.conda\"]\n", "");
        let diagnostics = findings(&text);
        assert_eq!(codes(&diagnostics), ["manifest.package.command.missing"]);
        assert!(!diagnostics[0].is_error());
    }

    #[test]
    fn a_publishing_package_must_say_how_to_prove_it_installs() {
        let text = MINIMAL.replace("smoke = [\"example-airlock --version\"]\n", "");
        assert_eq!(codes(&problems(&text)), ["manifest.package.smoke.missing"]);
    }

    #[test]
    fn a_package_that_publishes_nowhere_needs_no_smoke_command() {
        let text = MINIMAL
            .replace("smoke = [\"example-airlock --version\"]\n", "")
            .replace("publish = [\"conda-prefix\"]", "release = false");
        // The store then has no publisher, which is the only remaining finding.
        assert_eq!(codes(&findings(&text)), ["manifest.store.dead"]);
    }

    #[test]
    fn naming_a_store_that_does_not_exist_is_refused() {
        let text = MINIMAL.replace(
            "publish = [\"conda-prefix\"]",
            "publish = [\"conda-static\"]",
        );
        diagnostics_contain(&problems(&text), "manifest.package.publish.store_unknown");
    }

    #[test]
    fn kind_compatibility_is_decided_by_the_manifest() {
        // A Conda package cannot be uploaded to npm.
        let text = MINIMAL
            .replace("kind = \"prefix-dev\"", "kind = \"npmjs\"")
            .replace("channel = \"someone/channel\"\n", "");
        let diagnostics = problems(&text);
        diagnostics_contain(&diagnostics, "manifest.package.publish.incompatible");
    }

    #[test]
    fn a_store_named_twice_by_one_package_is_refused() {
        let text = MINIMAL.replace(
            "publish = [\"conda-prefix\"]",
            "publish = [\"conda-prefix\", \"conda-prefix\"]",
        );
        diagnostics_contain(&problems(&text), "manifest.package.publish.duplicate");
    }

    #[test]
    fn an_unknown_dependency_is_refused() {
        let text = MINIMAL.replace(
            "command = [\"cargo\", \"build\"]",
            "command = [\"cargo\", \"build\"]\ndepends_on = [\"ghost\"]",
        );
        assert_eq!(
            codes(&problems(&text)),
            ["manifest.package.dependency.unknown"]
        );
    }

    #[test]
    fn a_dependency_cycle_is_refused_once_per_member() {
        let text = r#"
schema = 1

[project]
name = "example"
platforms = ["linux-64"]

[[packages]]
name = "first"
kind = "conda"
command = ["cargo", "build"]
smoke = ["first --version"]
publish = ["conda-prefix"]
depends_on = ["second"]

[[packages]]
name = "second"
kind = "conda"
command = ["cargo", "build"]
smoke = ["second --version"]
publish = ["conda-prefix"]
depends_on = ["first"]

[[stores]]
name = "conda-prefix"
kind = "prefix-dev"
channel = "someone/channel"
"#;
        let diagnostics = problems(text);
        let cycles = diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.code() == "manifest.package.dependency.cycle")
            .count();
        assert_eq!(cycles, 2, "one finding per member of the cycle");
    }

    #[test]
    fn release_scoping_names_existing_stores() {
        let text = format!(
            "{MINIMAL}\n[release]\ntag_pattern = \" \"\n[[release.scope]]\nname = \"stable\"\nstores = [\"ghost\"]\n"
        );
        let diagnostics = problems(&text);
        assert_eq!(
            codes(&diagnostics),
            [
                "manifest.release.tag_pattern.empty",
                "manifest.release.scope.store_unknown",
            ]
        );
    }

    #[test]
    fn a_declared_check_slot_must_hold_a_command() {
        let text = format!("{MINIMAL}\n[workflow.checks]\nformat = \"just fmt\"\nlint = \" \"\n");
        assert_eq!(codes(&problems(&text)), ["manifest.workflow.check.empty"]);
    }

    #[test]
    fn an_unknown_check_slot_is_refused_by_the_shape() {
        let text = format!("{MINIMAL}\n[workflow.checks]\ntest = \"just test\"\n");
        assert_eq!(codes(&problems(&text)), ["manifest.shape.invalid"]);
    }

    #[test]
    fn declared_checks_keep_their_declaration_order() {
        let text = format!(
            "{MINIMAL}\n[workflow.checks]\ntypecheck = \"just typecheck\"\nformat = \"just fmt\"\n"
        );
        let manifest = parsed(&text);
        assert_eq!(
            manifest.workflow.checks.declared(),
            [("format", "just fmt"), ("typecheck", "just typecheck")]
        );
    }

    #[test]
    fn the_derived_mapping_names_the_packages_each_store_receives() {
        let manifest = parsed(MINIMAL);
        let destinations = manifest.destinations();
        assert_eq!(destinations.len(), 1);
        assert_eq!(destinations[0].store.name, "conda-prefix");
        assert_eq!(destinations[0].packages.len(), 1);
        assert_eq!(destinations[0].packages[0].name, "example-airlock");
        assert_eq!(manifest.released_packages().len(), 1);
    }

    #[test]
    fn an_opted_out_package_is_not_released() {
        let text = MINIMAL
            .replace("smoke = [\"example-airlock --version\"]\n", "")
            .replace(
                "publish = [\"conda-prefix\"]",
                "release = false\npublish = []",
            );
        let manifest = parsed(&text);
        assert!(manifest.released_packages().is_empty());
    }

    #[test]
    fn package_platforms_default_to_the_project_platforms() {
        let manifest = parsed(MINIMAL);
        assert_eq!(manifest.platforms_for(&manifest.packages[0]), ["linux-64"]);

        let text = MINIMAL.replace(
            "kind = \"conda\"",
            "kind = \"conda\"\nplatforms = [\"win-64\"]",
        );
        let manifest = parsed(&text);
        assert_eq!(manifest.platforms_for(&manifest.packages[0]), ["win-64"]);
    }

    #[test]
    fn airlock_adapters_are_collected_once_each() {
        let text = format!(
            "{MINIMAL}\n[airlock.developer]\nadapters = [\"pixi\", \"bun\"]\nmirror = \"mirror\"\n\n[airlock.delivery]\nadapters = [\"pixi\", \"cargo\"]\ncatalog = \"github:someone/repo@0123456\"\n"
        );
        let manifest = parsed(&text);
        assert_eq!(
            manifest.lock_adapters(),
            [LockAdapter::Pixi, LockAdapter::Bun, LockAdapter::Cargo]
        );
        assert_eq!(LockAdapter::Cargo.as_str(), "cargo");
        assert_eq!(
            manifest
                .airlock
                .expect("airlock")
                .developer
                .expect("developer")
                .mirror
                .as_deref(),
            Some("mirror")
        );
    }

    #[test]
    fn a_manifest_without_airlock_declares_no_adapters() {
        assert!(parsed(MINIMAL).lock_adapters().is_empty());
    }

    #[test]
    fn the_manifest_round_trips_through_json() {
        let manifest = parsed(MINIMAL);
        let json = manifest.to_json().expect("serializes");
        let restored: Manifest = serde_json::from_str(&json).expect("deserializes");
        assert_eq!(restored, manifest);
    }

    #[test]
    fn toml_scalars_that_json_cannot_hold_are_reported() {
        let text = format!("{MINIMAL}\n[extra]\nvalue = nan\n");
        // `extra` is also an unknown field, but the value conversion runs first.
        assert_eq!(codes(&problems(&text)), ["manifest.toml.unsupported_value"]);
    }

    #[test]
    fn toml_datetimes_survive_the_conversion() {
        let text = format!("{MINIMAL}\n[build.extra]\nstamp = 1979-05-27\n");
        // The conversion succeeds; the strict shape then refuses the unknown facet.
        assert_eq!(codes(&problems(&text)), ["manifest.shape.invalid"]);
    }

    #[test]
    fn duplicates_are_detected_positionally() {
        assert!(has_duplicates(&["a".to_owned(), "a".to_owned()]));
        assert!(!has_duplicates(&["a".to_owned(), "b".to_owned()]));
        assert!(!has_duplicates(&[]));
    }

    #[test]
    fn store_kinds_report_whether_they_are_indexed_and_what_they_accept() {
        assert!(StoreKind::Static.generates_index());
        assert!(StoreKind::PrefixDev.generates_index());
        assert!(StoreKind::Branch.generates_index());
        assert!(!StoreKind::Releases.generates_index());
        assert!(!StoreKind::Npmjs.generates_index());

        assert!(StoreKind::Static.accepts(PackageKind::Conda));
        assert!(StoreKind::Static.accepts(PackageKind::Pypi));
        assert!(StoreKind::Static.accepts(PackageKind::R));
        assert!(!StoreKind::Static.accepts(PackageKind::Npm));
        assert!(!StoreKind::Static.accepts(PackageKind::Cargo));
        assert!(StoreKind::Npmjs.accepts(PackageKind::Cargo));
        assert!(!StoreKind::Npmjs.accepts(PackageKind::Pypi));
        assert!(StoreKind::CratesIo.accepts(PackageKind::Cargo));
        assert!(!StoreKind::CratesIo.accepts(PackageKind::Conda));
        assert!(StoreKind::Releases.accepts(PackageKind::Npm));
        assert!(StoreKind::Anaconda.accepts(PackageKind::Conda));
        assert!(!StoreKind::Anaconda.accepts(PackageKind::Pypi));
        assert!(StoreKind::Quetz.accepts(PackageKind::Conda));
        assert!(StoreKind::Artifactory.accepts(PackageKind::Conda));
        assert!(StoreKind::Artifactory.accepts(PackageKind::Pypi));
        assert!(StoreKind::S3.accepts(PackageKind::Conda));
        assert!(StoreKind::Filesystem.accepts(PackageKind::Conda));
        assert!(StoreKind::Branch.accepts(PackageKind::Conda));
        assert!(!StoreKind::Pypi.accepts(PackageKind::Conda));
        assert!(StoreKind::Pypi.accepts(PackageKind::Pypi));
        assert!(!StoreKind::GithubPackages.accepts(PackageKind::Conda));
        assert!(StoreKind::GithubPackages.accepts(PackageKind::Npm));
        assert!(!StoreKind::Command.accepts(PackageKind::Conda));
    }

    #[test]
    fn store_kind_spellings_are_stable() {
        assert_eq!(StoreKind::PrefixDev.as_str(), "prefix-dev");
        assert_eq!(StoreKind::Static.as_str(), "static");
        assert_eq!(StoreKind::GithubPackages.as_str(), "github-packages");
        assert_eq!(StoreKind::CratesIo.as_str(), "crates-io");
        assert_eq!(PackageKind::R.as_str(), "r");
        assert_eq!(PackageKind::Npm.as_str(), "npm");
    }

    const RELEASE: &str = r#"{
        "schema": 1,
        "version": "0.1.0",
        "commit": "0123456789abcdef0123456789abcdef01234567",
        "packages": [
            {
                "name": "universe-airlock",
                "kind": "conda",
                "version": "0.1.0",
                "artifacts": [
                    {
                        "filename": "universe-airlock-0.1.0-h1b2c3d4_0.conda",
                        "sha256": "0000000000000000000000000000000000000000000000000000000000000000",
                        "size": 1024,
                        "stores": ["conda-prefix"]
                    }
                ]
            }
        ]
    }"#;

    #[test]
    fn a_release_manifest_pins_bytes_rather_than_jobs() {
        let manifest = ReleaseManifest::from_json(RELEASE).expect("parses");
        assert_eq!(manifest.schema, RELEASE_MANIFEST_SCHEMA_VERSION);
        assert!(manifest.validate().is_empty());
        assert_eq!(
            manifest.to_json().expect("serializes"),
            RELEASE.replace(['\n', ' '], "")
        );
    }

    #[test]
    fn a_release_manifest_that_proves_nothing_is_refused() {
        let text = RELEASE
            .replace("\"schema\": 1", "\"schema\": 7")
            .replace("\"version\": \"0.1.0\"", "\"version\": \" \"")
            .replace("\"size\": 1024", "\"size\": 0")
            .replace("\"stores\": [\"conda-prefix\"]", "\"stores\": []");
        let manifest = ReleaseManifest::from_json(&text).expect("parses");
        assert_eq!(
            codes(&manifest.validate()),
            [
                "release.schema.unsupported",
                // Both the release version and the package version were blanked.
                "release.field.empty",
                "release.field.empty",
                "release.artifact.size.zero",
                "release.artifact.stores.empty",
            ]
        );
    }

    #[test]
    fn an_empty_release_manifest_is_refused() {
        let text = r#"{"schema":1,"version":"0.1.0","commit":"abc","packages":[]}"#;
        let manifest = ReleaseManifest::from_json(text).expect("parses");
        assert_eq!(codes(&manifest.validate()), ["release.packages.empty"]);
    }

    #[test]
    fn released_packages_and_artifacts_are_each_checked() {
        let text = r#"{"schema":1,"version":"0.1.0","commit":"abc","packages":[
            {"name":" ","kind":"conda","version":" ","artifacts":[]},
            {"name":"a","kind":"conda","version":"1","artifacts":[
                {"filename":" ","sha256":"0000000000000000000000000000000000000000000000000000000000000000","size":1,"stores":["s"]}
            ]}
        ]}"#;
        let manifest = ReleaseManifest::from_json(text).expect("parses");
        let diagnostics = manifest.validate();
        let codes = codes(&diagnostics);
        assert!(codes.contains(&"release.field.empty"));
        assert!(codes.contains(&"release.artifacts.empty"));
        assert!(codes.contains(&"release.artifact.filename.empty"));
    }

    #[test]
    fn an_unknown_release_field_is_refused() {
        let text = RELEASE.replace("\"commit\"", "\"comit\"");
        assert!(ReleaseManifest::from_json(&text).is_err());
    }

    fn diagnostics_contain(diagnostics: &[Diagnostic], code: &str) {
        assert!(
            diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code() == code),
            "expected {code} in {:?}",
            codes(diagnostics)
        );
    }
}
