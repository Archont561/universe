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

/// A distributable Universe product sharing this workspace.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Product {
    /// Connected owner/producer CLI.
    Kit,
    /// Finite offline snapshot consumer.
    Airlock,
    /// Thin standalone bootstrapper used before Pixi is available.
    Bootstrap,
}

impl Product {
    /// Every product in release/declaration order.
    pub const ALL: [Self; 3] = [Self::Kit, Self::Airlock, Self::Bootstrap];

    /// Human-facing product name.
    #[must_use]
    pub const fn display_name(self) -> &'static str {
        match self {
            Self::Kit => "Universe Kit",
            Self::Airlock => "Universe Airlock",
            Self::Bootstrap => "Universe Airlock Bootstrap",
        }
    }

    /// Executable name without any platform suffix.
    #[must_use]
    pub const fn binary_name(self) -> &'static str {
        match self {
            Self::Kit => "universe-kit",
            Self::Airlock => "universe-airlock",
            Self::Bootstrap => "universe-airlock-bootstrap",
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

/// Render one product version as plain text or the versioned JSON envelope.
#[must_use]
pub fn render_version(product: Product, options: VersionOptions) -> String {
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
                binary: product.binary_name(),
                version: VERSION,
            },
            Vec::new(),
            Vec::new(),
        );
        envelope
            .to_json()
            .expect("serializing the version envelope cannot fail")
    } else {
        format!("{} {VERSION}", product.binary_name())
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
    match parse_version_args(args) {
        Ok(options) => CommandOutput {
            stdout: format!("{}\n", render_version(product, options)),
            stderr: String::new(),
            exit_code: 0,
        },
        Err(error) => CommandOutput {
            stdout: String::new(),
            stderr: format!(
                "error: {error}\nusage: {} [version] [--json]\n",
                product.binary_name()
            ),
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
    let output = execute_version(product, args);
    print!("{}", output.stdout());
    eprint!("{}", output.stderr());
    ExitCode::from(output.exit_code())
}
