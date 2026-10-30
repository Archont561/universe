//! What a `.conda` archive says about itself.
//!
//! The archive is a zip of two zstd-compressed tars: `info-*.tar.zst` holds the
//! metadata a solver reads, `pkg-*.tar.zst` holds the payload. This module reads
//! the first one and the file's own digest, which is everything an audit needs
//! before it looks at a single binary.
//!
//! Nothing here shells out to `tar`, `zstd`, or `sha256sum`: the archive is
//! opened as a file and streamed, so the auditor behaves the same on a host with
//! no Conda tooling at all.

use std::fs::File;
use std::path::Path;

use rattler_digest::Sha256;
use serde::Deserialize;
use universe_core::artifact::{Artifact, ArtifactId};
use universe_core::digest::Sha256Digest;

/// A failure while reading an archive.
#[derive(Debug, thiserror::Error)]
pub enum PackageError {
    /// The archive could not be read.
    #[error("cannot read {path}: {source}")]
    Io {
        /// Path that failed.
        path: String,
        /// Underlying error.
        #[source]
        source: std::io::Error,
    },

    /// The archive is not a `.conda`, or its info section cannot be opened.
    #[error("cannot open the archive's info section: {0}")]
    Unreadable(String),

    /// The info section carries no `info/index.json`.
    #[error("the archive has no info/index.json")]
    MissingIndex,

    /// `info/index.json` is not the record its schema describes.
    #[error("info/index.json is not a Conda index record: {0}")]
    InvalidIndex(#[source] serde_json::Error),

    /// The index names an identity the neutral vocabulary rejects.
    #[error("info/index.json names an invalid artifact identity: {0}")]
    InvalidIdentity(String),
}

/// The fields of `info/index.json` an audit acts on.
///
/// Unknown fields are accepted, unlike every record in `universe-core`. That
/// asymmetry is deliberate: this is a *foreign* record, produced by `conda-build`
/// or `rattler-build`, and a real index carries `platform`, `arch`,
/// `license_family`, `timestamp`, and whatever the next builder adds. Rejecting
/// those would make the auditor fail on correct archives; the fields below are the
/// ones an audit acts on, and the rest are read past.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct PackageIndex {
    /// Package name, as the channel spells it.
    pub name: String,
    /// Package version, unnormalised.
    pub version: String,
    /// Build string, which with name and version identifies one artifact.
    pub build: String,
    /// Monotonic build counter within a version.
    pub build_number: u64,
    /// Subdirectory the artifact belongs to.
    pub subdir: String,
    /// SPDX licence expression, when the recipe declared one.
    #[serde(default)]
    pub license: Option<String>,
    /// Runtime dependencies, as Conda specifiers.
    #[serde(default)]
    pub depends: Vec<String>,
}

/// An opened archive: the neutral artifact record plus the Conda index.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CondaPackage {
    artifact: Artifact,
    index: PackageIndex,
}

impl CondaPackage {
    /// The neutral artifact record: identity, digest, and size of the bytes.
    #[must_use]
    pub fn artifact(&self) -> &Artifact {
        &self.artifact
    }

    /// The `info/index.json` record.
    #[must_use]
    pub fn index(&self) -> &PackageIndex {
        &self.index
    }
}

/// Read an archive's identity, size, digest, and index record.
///
/// # Errors
///
/// Returns [`PackageError`] when the file cannot be read, is not a `.conda`, has
/// no `info/index.json`, or carries a record that does not match its schema.
pub fn open_package(path: impl AsRef<Path>) -> Result<CondaPackage, PackageError> {
    let path = path.as_ref();
    let file = File::open(path).map_err(|source| PackageError::Io {
        path: path.display().to_string(),
        source,
    })?;
    let size_bytes = file
        .metadata()
        .map_err(|source| PackageError::Io {
            path: path.display().to_string(),
            source,
        })?
        .len();

    let index_bytes = read_info_member(path)?;
    let index: PackageIndex =
        serde_json::from_slice(&index_bytes).map_err(PackageError::InvalidIndex)?;

    let id = ArtifactId::new(index.name.clone(), index.version.clone())
        .map_err(|diagnostic| PackageError::InvalidIdentity(diagnostic.message().to_owned()))?;
    let digest =
        rattler_digest::compute_file_digest::<Sha256>(path).map_err(|source| PackageError::Io {
            path: path.display().to_string(),
            source,
        })?;
    let digest = Sha256Digest::parse(&hex_lower(digest.as_slice()))
        .map_err(|error| PackageError::InvalidIdentity(error.to_string()))?;

    Ok(CondaPackage {
        artifact: Artifact {
            id,
            digest,
            size_bytes,
            media_type: None,
        },
        index,
    })
}

/// Read `info/index.json` from the archive's info section.
fn read_info_member(path: &Path) -> Result<Vec<u8>, PackageError> {
    let file = File::open(path).map_err(|source| PackageError::Io {
        path: path.display().to_string(),
        source,
    })?;
    let mut archive = rattler_package_streaming::seek::stream_conda_info(file)
        .map_err(|error| PackageError::Unreadable(error.to_string()))?;
    let entries = archive
        .entries()
        .map_err(|error| PackageError::Unreadable(error.to_string()))?;
    for entry in entries {
        let mut entry = entry.map_err(|error| PackageError::Unreadable(error.to_string()))?;
        let entry_path = entry
            .path()
            .map_err(|error| PackageError::Unreadable(error.to_string()))?;
        if entry_path == Path::new("info/index.json") {
            let mut contents = Vec::new();
            std::io::Read::read_to_end(&mut entry, &mut contents)
                .map_err(|error| PackageError::Unreadable(error.to_string()))?;
            return Ok(contents);
        }
    }
    Err(PackageError::MissingIndex)
}

/// Lowercase hexadecimal, the only form a [`Sha256Digest`] accepts.
fn hex_lower(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    bytes.iter().fold(String::new(), |mut text, byte| {
        let _ = write!(text, "{byte:02x}");
        text
    })
}
