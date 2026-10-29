//! Explicit, checked schema versions for every machine-readable record family.
//!
//! A record family is versioned by a positive integer that a reader compares
//! against the version it supports. Version `0` is never valid, and an
//! unsupported version is a hard error rather than a best-effort read: a record
//! whose shape is not understood must fail closed.

use std::fmt;

use serde::de::{self, Deserialize, Deserializer, Unexpected, Visitor};
use serde::ser::{Serialize, Serializer};

use crate::error::ModelError;

/// A record-family schema version.
///
/// The field is private and [`SchemaVersion::new`] rejects `0`, so a valid
/// version is the only one that can exist.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct SchemaVersion(u32);

impl SchemaVersion {
    /// Version `1`, the first version of every record family.
    pub const V1: Self = Self(1);

    /// Version `2`.
    pub const V2: Self = Self(2);

    /// Wrap a raw version, rejecting `0`.
    ///
    /// # Errors
    ///
    /// Returns [`ModelError::ZeroSchemaVersion`] when `value` is `0`.
    pub fn new(value: u32) -> Result<Self, ModelError> {
        if value == 0 {
            Err(ModelError::ZeroSchemaVersion)
        } else {
            Ok(Self(value))
        }
    }

    /// The raw version number.
    #[must_use]
    pub const fn get(self) -> u32 {
        self.0
    }
}

impl fmt::Display for SchemaVersion {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}", self.get())
    }
}

impl Serialize for SchemaVersion {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_u32(self.get())
    }
}

impl<'de> Deserialize<'de> for SchemaVersion {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct VersionVisitor;

        impl Visitor<'_> for VersionVisitor {
            type Value = SchemaVersion;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("a positive integer schema version")
            }

            fn visit_u64<E: de::Error>(self, value: u64) -> Result<Self::Value, E> {
                let version = u32::try_from(value)
                    .map_err(|_| E::invalid_value(Unexpected::Unsigned(value), &self))?;
                SchemaVersion::new(version).map_err(|error| E::custom(error.to_string()))
            }
        }

        deserializer.deserialize_u32(VersionVisitor)
    }
}

/// Schema version of the `catalog` record family.
pub const CATALOG_SCHEMA_VERSION: SchemaVersion = SchemaVersion::V1;

/// Schema version of the `snapshot` record family.
pub const SNAPSHOT_SCHEMA_VERSION: SchemaVersion = SchemaVersion::V1;

#[cfg(test)]
mod tests {
    use super::{CATALOG_SCHEMA_VERSION, SNAPSHOT_SCHEMA_VERSION, SchemaVersion};
    use crate::error::ModelError;

    #[test]
    fn rejects_the_zero_version() {
        assert!(matches!(
            SchemaVersion::new(0),
            Err(ModelError::ZeroSchemaVersion)
        ));
        assert_eq!(SchemaVersion::new(1).ok(), Some(SchemaVersion::V1));
        assert_eq!(SchemaVersion::new(2).ok(), Some(SchemaVersion::V2));
        assert_eq!(SchemaVersion::V1.to_string(), "1");
        assert_eq!(SchemaVersion::V2.get(), 2);
        assert_eq!(CATALOG_SCHEMA_VERSION, SchemaVersion::V1);
        assert_eq!(SNAPSHOT_SCHEMA_VERSION, SchemaVersion::V1);
        assert!(SchemaVersion::V1 < SchemaVersion::V2);
    }
}
