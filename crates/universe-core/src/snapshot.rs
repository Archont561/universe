//! Pinned release snapshots.
//!
//! A snapshot is the neutral, immutable pairing of an identifier with one
//! catalogue: the exact entries and artifact digests Universe publishes under that
//! name. Pinning a snapshot is what makes an offline restore reproducible — the
//! identifier names the data, and the digests prove the bytes.

use serde::{Deserialize, Serialize};

use crate::catalog::{Catalog, check_version};
use crate::diagnostic::{Diagnostic, Severity};
use crate::error::ModelError;
use crate::schema::{SNAPSHOT_SCHEMA_VERSION, SchemaVersion};

/// A named, versioned pin over one catalogue.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Snapshot {
    /// Version of the snapshot record family.
    pub schema_version: SchemaVersion,
    /// Stable identifier of this pinned snapshot.
    pub id: String,
    /// The exact catalogue this snapshot pins.
    pub catalog: Catalog,
}

impl Snapshot {
    /// Read a snapshot, rejecting unknown fields and unsupported versions.
    ///
    /// # Errors
    ///
    /// Returns [`ModelError::Json`] when the text is not a snapshot, and
    /// [`ModelError::UnsupportedSchemaVersion`] when either the snapshot or its
    /// catalogue declares a version this build does not implement.
    pub fn from_json(text: &str) -> Result<Self, ModelError> {
        let snapshot: Self = serde_json::from_str(text)?;
        check_version("snapshot", snapshot.schema_version, SNAPSHOT_SCHEMA_VERSION)?;
        check_version(
            "catalog",
            snapshot.catalog.schema_version,
            crate::schema::CATALOG_SCHEMA_VERSION,
        )?;
        Ok(snapshot)
    }

    /// Serialize the snapshot to compact JSON.
    ///
    /// # Errors
    ///
    /// Returns the serializer error when the snapshot cannot be represented.
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }

    /// Field-level problems with this snapshot and its catalogue.
    #[must_use]
    pub fn validate(&self) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();
        if self.id.trim().is_empty() {
            diagnostics.push(
                Diagnostic::new(
                    Severity::Error,
                    "snapshot.id.empty",
                    "snapshot id must not be empty",
                )
                .with_path("id".to_owned()),
            );
        }
        if self.catalog.entries.is_empty() {
            diagnostics.push(
                Diagnostic::new(
                    Severity::Error,
                    "snapshot.catalog.empty",
                    "a snapshot must pin at least one catalogue entry",
                )
                .with_path("catalog.entries".to_owned()),
            );
        }
        for diagnostic in self.catalog.validate() {
            let path = diagnostic
                .path()
                .map_or_else(|| "catalog".to_owned(), |path| format!("catalog.{path}"));
            diagnostics.push(diagnostic.with_path(path));
        }
        diagnostics
    }
}
