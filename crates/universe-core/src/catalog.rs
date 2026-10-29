//! Catalogues of software, data, model, and skill entries.
//!
//! A catalogue is the neutral list of things Universe can describe. It carries no
//! ecosystem vocabulary: an entry says what kind of thing it is, what its native
//! name and version are, and which artifacts publish it. Everything a specific
//! ecosystem needs — channel, build string, registry coordinates — is added by
//! the adapter that owns that ecosystem.

use serde::{Deserialize, Serialize};

use crate::EntryKind;
use crate::artifact::Artifact;
use crate::diagnostic::{Diagnostic, Severity};
use crate::error::ModelError;
use crate::schema::{CATALOG_SCHEMA_VERSION, SchemaVersion};

/// One described thing in a catalogue.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CatalogEntry {
    /// Which family of thing this entry describes.
    pub kind: EntryKind,
    /// Native name inside its ecosystem.
    pub name: String,
    /// Native version string, unparsed and unnormalised.
    pub version: String,
    /// Optional one-line description.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    /// Payloads this entry publishes.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub artifacts: Vec<Artifact>,
}

impl CatalogEntry {
    /// Field-level problems with this entry, in field order.
    #[must_use]
    pub fn validate(&self) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();
        for (field, value) in [("name", &self.name), ("version", &self.version)] {
            if value.trim().is_empty() {
                diagnostics.push(
                    Diagnostic::new(
                        Severity::Error,
                        "catalog.entry.empty",
                        format!("catalog entry {field} must not be empty"),
                    )
                    .with_path(field.to_owned()),
                );
            }
        }
        if let Some(summary) = &self.summary
            && summary.trim().is_empty()
        {
            diagnostics.push(
                Diagnostic::new(
                    Severity::Error,
                    "catalog.entry.summary_empty",
                    "catalog entry summary must not be empty when present",
                )
                .with_path("summary".to_owned()),
            );
        }
        for (index, artifact) in self.artifacts.iter().enumerate() {
            for diagnostic in artifact.validate() {
                diagnostics.push(prefixed(&format!("artifacts[{index}]"), diagnostic));
            }
        }
        diagnostics
    }
}

/// Prefix a diagnostic's path with the parent location that produced it.
fn prefixed(location: &str, diagnostic: Diagnostic) -> Diagnostic {
    let path = match diagnostic.path() {
        Some(path) => format!("{location}.{path}"),
        None => location.to_owned(),
    };
    diagnostic.with_path(path)
}

/// A versioned, ecosystem-neutral catalogue.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Catalog {
    /// Version of the catalogue record family.
    pub schema_version: SchemaVersion,
    /// Described entries, in publication order.
    pub entries: Vec<CatalogEntry>,
}

impl Catalog {
    /// Read a catalogue, rejecting unknown fields and unsupported versions.
    ///
    /// # Errors
    ///
    /// Returns [`ModelError::Json`] when the text is not a catalogue at all, and
    /// [`ModelError::UnsupportedSchemaVersion`] when it declares a version this
    /// build does not implement.
    pub fn from_json(text: &str) -> Result<Self, ModelError> {
        let catalog: Self = serde_json::from_str(text)?;
        check_version("catalog", catalog.schema_version, CATALOG_SCHEMA_VERSION)?;
        Ok(catalog)
    }

    /// Serialize the catalogue to compact JSON.
    ///
    /// # Errors
    ///
    /// Returns the serializer error when the catalogue cannot be represented.
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }

    /// Entries of one kind, in catalogue order.
    pub fn entries_of_kind(&self, kind: EntryKind) -> impl Iterator<Item = &CatalogEntry> {
        self.entries.iter().filter(move |entry| entry.kind == kind)
    }

    /// Field-level problems with this catalogue, with an `entries[<n>]` path.
    #[must_use]
    pub fn validate(&self) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();
        let mut seen: Vec<(&str, &str)> = Vec::new();

        for (index, entry) in self.entries.iter().enumerate() {
            let location = format!("entries[{index}]");
            for diagnostic in entry.validate() {
                diagnostics.push(prefixed(&location, diagnostic));
            }
            let identity = (entry.name.as_str(), entry.version.as_str());
            if seen.contains(&identity) && !identity.0.trim().is_empty() {
                diagnostics.push(
                    Diagnostic::new(
                        Severity::Error,
                        "catalog.entry.duplicate",
                        format!(
                            "catalog entry {name} {version} appears more than once",
                            name = identity.0,
                            version = identity.1
                        ),
                    )
                    .with_path(location.clone()),
                );
            } else {
                seen.push(identity);
            }
            if entry.artifacts.is_empty() {
                diagnostics.push(
                    Diagnostic::new(
                        Severity::Warning,
                        "catalog.entry.no_artifacts",
                        "catalog entry publishes no artifacts",
                    )
                    .with_path(location.clone()),
                );
            }
        }

        diagnostics
    }
}

/// Reject a record family read at a version this build does not implement.
pub(crate) fn check_version(
    family: &'static str,
    found: SchemaVersion,
    supported: SchemaVersion,
) -> Result<(), ModelError> {
    if found == supported {
        Ok(())
    } else {
        Err(ModelError::UnsupportedSchemaVersion {
            family,
            found,
            supported,
        })
    }
}
