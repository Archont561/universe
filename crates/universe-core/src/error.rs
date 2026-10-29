//! Typed failures for reading or validating ecosystem-neutral records.
//!
//! Every variant names what was wrong with which input, so a caller can report a
//! field-level diagnostic instead of a generic parse failure.

use crate::schema::SchemaVersion;

/// A record could not be read or validated.
#[derive(Debug, thiserror::Error)]
pub enum ModelError {
    /// A schema version declared as `0`, which no record family defines.
    #[error("schema version must be a positive integer")]
    ZeroSchemaVersion,

    /// A record family was read at a version this build does not implement.
    #[error("unsupported {family} schema version {found}; this build supports {supported}")]
    UnsupportedSchemaVersion {
        /// Record family that was read.
        family: &'static str,
        /// Version found in the record.
        found: SchemaVersion,
        /// Version this build implements.
        supported: SchemaVersion,
    },

    /// A digest was not 64 lowercase hexadecimal characters.
    #[error("invalid sha256 digest {digest:?}: expected 64 lowercase hexadecimal characters")]
    InvalidDigest {
        /// The rejected digest text.
        digest: String,
    },

    /// A required field was present but empty.
    #[error("field {field} must not be empty")]
    EmptyField {
        /// Name of the offending field.
        field: &'static str,
    },

    /// The record was not valid JSON for its declared shape.
    #[error("invalid record: {0}")]
    Json(#[from] serde_json::Error),
}
