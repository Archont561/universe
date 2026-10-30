//! Ecosystem-neutral identity, record, and command-output primitives for Universe.
//!
//! This crate deliberately contains no ecosystem adapter, terminal UI, networking,
//! build, or publishing logic. It owns the vocabulary the rest of the workspace
//! shares: product identity, the catalogue of software/data/model/skill entries,
//! artifact identity and digests, diagnostics, pinned snapshots, and the versioned
//! stdout envelope. Those concerns belong in adapter and binary crates built on
//! top of this stable vocabulary.

pub mod artifact;
pub mod catalog;
pub mod diagnostic;
pub mod digest;
pub mod error;
pub mod schema;
pub mod snapshot;

use std::fmt;
use std::process::ExitCode;

pub use artifact::{Artifact, ArtifactId};
pub use catalog::{Catalog, CatalogEntry};
pub use diagnostic::{Diagnostic, Envelope, Severity};
pub use digest::Sha256Digest;
pub use error::ModelError;
pub use schema::{CATALOG_SCHEMA_VERSION, SNAPSHOT_SCHEMA_VERSION, SchemaVersion};
pub use snapshot::Snapshot;

/// Workspace version shared by every Universe artifact.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Version of the machine-readable stdout envelope.
///
/// Version `2` widened `diagnostics` and `artifacts` from bare strings to the
/// typed [`Diagnostic`] and [`Artifact`] records. The `version` command emits both
/// arrays empty, so its bytes are unchanged, but the contract is not.
pub const STDOUT_SCHEMA_VERSION: SchemaVersion = SchemaVersion::V2;

/// Maximum number of first-class ecosystem adapters maintained in this repo.
pub const FIRST_CLASS_ADAPTER_BUDGET: usize = 5;

/// A Universe edition: the unit that carries a trust posture (`decision-122`).
///
/// The bootstrap is a phase of [`Product::Airlock`] rather than an edition of its own, so it
/// reports this edition while naming its own binary. A third variant would say the workspace
/// ships three products, which is the claim `decision-122` rejects.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Product {
    /// Connected owner/producer CLI.
    Kit,
    /// Finite offline snapshot consumer.
    Airlock,
}

impl Product {
    /// Every edition in release/declaration order.
    pub const ALL: [Self; 2] = [Self::Kit, Self::Airlock];

    /// Human-facing edition name.
    #[must_use]
    pub const fn display_name(self) -> &'static str {
        match self {
            Self::Kit => "Universe Kit",
            Self::Airlock => "Universe Airlock",
        }
    }

    /// Canonical executable name of the edition, without any platform suffix.
    ///
    /// A binary that is not the edition's own executable — the bootstrapper — passes its own
    /// name to [`render_version_as`] instead of this value.
    #[must_use]
    pub const fn binary_name(self) -> &'static str {
        match self {
            Self::Kit => "universe-kit",
            Self::Airlock => "universe-airlock",
        }
    }
}

impl fmt::Display for Product {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.display_name())
    }
}

/// The high-level catalogue entry kinds Universe is designed to describe.
#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum EntryKind {
    /// Packages, applications, libraries, runtimes, and tools.
    Software,
    /// Datasets, geospatial catalogues, fixtures, and archives.
    Data,
    /// Built ML artifacts and geospatial or 3D model outputs.
    Model,
    /// Versioned Agent Skill directories.
    Skill,
}

impl EntryKind {
    /// Every currently recognised entry kind.
    pub const ALL: [Self; 4] = [Self::Software, Self::Data, Self::Model, Self::Skill];

    /// Stable lowercase catalogue slug.
    #[must_use]
    pub const fn slug(self) -> &'static str {
        match self {
            Self::Software => "software",
            Self::Data => "data",
            Self::Model => "model",
            Self::Skill => "skill",
        }
    }
}

/// Parsed options shared by the scaffold version commands.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct VersionOptions {
    json: bool,
}

impl VersionOptions {
    /// Whether to emit the versioned JSON envelope instead of plain text.
    #[must_use]
    pub const fn json(self) -> bool {
        self.json
    }
}

/// An argument error from the intentionally small scaffold CLI surface.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum CliError {
    /// The version token appeared more than once.
    #[error("version may be requested only once")]
    DuplicateVersion,
    /// An unsupported argument was supplied.
    #[error("unknown argument: {0}")]
    UnknownArgument(String),
}

/// Parse no arguments, `version`, `--version`, or `-V`, optionally with `--json`.
///
/// # Errors
///
/// Returns [`CliError::DuplicateVersion`] when a version token repeats, or
/// [`CliError::UnknownArgument`] for every other unsupported token.
pub fn parse_version_args<I, S>(args: I) -> Result<VersionOptions, CliError>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let mut options = VersionOptions::default();
    let mut version_requested = false;

    for argument in args {
        match argument.as_ref() {
            "--json" => options.json = true,
            "version" | "--version" | "-V" => {
                if version_requested {
                    return Err(CliError::DuplicateVersion);
                }
                version_requested = true;
            }
            other => return Err(CliError::UnknownArgument(other.to_owned())),
        }
    }

    Ok(options)
}

/// Render one edition version as plain text or the versioned JSON envelope.
#[must_use]
pub fn render_version(product: Product, options: VersionOptions) -> String {
    render_version_as(product, product.binary_name(), options)
}

/// Render one edition version, naming a binary that is not the edition's own executable.
///
/// The bootstrapper uses this: it is an Airlock phase, so the envelope reports the Airlock
/// edition while the `binary` field names `universe-airlock-bootstrap`.
#[must_use]
pub fn render_version_as(
    product: Product,
    binary: &'static str,
    options: VersionOptions,
) -> String {
    if options.json() {
        #[derive(serde::Serialize)]
        struct ResultValue {
            product: &'static str,
            binary: &'static str,
            version: &'static str,
        }

        let envelope = Envelope::new(
            STDOUT_SCHEMA_VERSION,
            "version",
            true,
            ResultValue {
                product: product.display_name(),
                binary,
                version: VERSION,
            },
            Vec::new(),
            Vec::new(),
        );
        envelope
            .to_json()
            .expect("serializing the version envelope cannot fail")
    } else {
        format!("{binary} {VERSION}")
    }
}

/// Fully materialised output from one scaffold command invocation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CommandOutput {
    stdout: String,
    stderr: String,
    exit_code: u8,
}

impl CommandOutput {
    /// Text destined for stdout.
    #[must_use]
    pub fn stdout(&self) -> &str {
        &self.stdout
    }

    /// Text destined for stderr.
    #[must_use]
    pub fn stderr(&self) -> &str {
        &self.stderr
    }

    /// Process exit code: 0 for success, 2 for a usage/config error.
    #[must_use]
    pub const fn exit_code(&self) -> u8 {
        self.exit_code
    }
}

/// Execute the scaffold version command without touching process streams.
pub fn execute_version<I, S>(product: Product, args: I) -> CommandOutput
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    execute_version_as(product, product.binary_name(), args)
}

/// Execute the version command for a binary that is not the edition's own executable.
pub fn execute_version_as<I, S>(product: Product, binary: &'static str, args: I) -> CommandOutput
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    match parse_version_args(args) {
        Ok(options) => CommandOutput {
            stdout: format!("{}\n", render_version_as(product, binary, options)),
            stderr: String::new(),
            exit_code: 0,
        },
        Err(error) => CommandOutput {
            stdout: String::new(),
            stderr: format!("error: {error}\nusage: {binary} [version] [--json]\n"),
            exit_code: 2,
        },
    }
}

/// Execute and print the scaffold version command, returning its process code.
pub fn run_main<I, S>(product: Product, args: I) -> ExitCode
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    run_main_as(product, product.binary_name(), args)
}

/// Execute and print the version command for a binary that is not the edition's executable.
pub fn run_main_as<I, S>(product: Product, binary: &'static str, args: I) -> ExitCode
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let output = execute_version_as(product, binary, args);
    print!("{}", output.stdout());
    eprint!("{}", output.stderr());
    ExitCode::from(output.exit_code())
}

#[cfg(test)]
mod tests {
    use super::VERSION;

    // The workspace version is the single release identity: `VERSIONS` files, recipes, and
    // the changelog heading are all checked against it, so this crate must not shadow it.
    #[test]
    fn version_is_taken_from_the_package_manifest() {
        assert_eq!(VERSION, env!("CARGO_PKG_VERSION"));
        assert!(!VERSION.is_empty());
    }
}
