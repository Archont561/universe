//! Content digests in the one form every record uses.
//!
//! A digest is stored and compared as lowercase hexadecimal SHA-256 text. This
//! crate deliberately does not hash bytes: hashing belongs to the code that owns
//! the artifact, while the neutral vocabulary only has to agree on the encoding.

use std::fmt;
use std::str::FromStr;

use serde::de::{self, Deserialize, Deserializer, Visitor};
use serde::ser::{Serialize, Serializer};

use crate::error::ModelError;

/// Number of hexadecimal characters in a SHA-256 digest.
pub const SHA256_HEX_LENGTH: usize = 64;

/// A validated lowercase hexadecimal SHA-256 digest.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Sha256Digest(String);

impl Sha256Digest {
    /// Validate and own a digest.
    ///
    /// # Errors
    ///
    /// Returns [`ModelError::InvalidDigest`] unless the text is exactly 64
    /// lowercase hexadecimal characters.
    pub fn parse(text: &str) -> Result<Self, ModelError> {
        let valid = text.len() == SHA256_HEX_LENGTH
            && text
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte));
        if valid {
            Ok(Self(text.to_owned()))
        } else {
            Err(ModelError::InvalidDigest {
                digest: text.to_owned(),
            })
        }
    }

    /// The digest as lowercase hexadecimal text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for Sha256Digest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl FromStr for Sha256Digest {
    type Err = ModelError;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        Self::parse(text)
    }
}

impl Serialize for Sha256Digest {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for Sha256Digest {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct DigestVisitor;

        impl Visitor<'_> for DigestVisitor {
            type Value = Sha256Digest;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("a lowercase hexadecimal sha256 digest")
            }

            fn visit_str<E: de::Error>(self, value: &str) -> Result<Self::Value, E> {
                Sha256Digest::parse(value).map_err(|error| E::custom(error.to_string()))
            }
        }

        deserializer.deserialize_str(DigestVisitor)
    }
}

#[cfg(test)]
mod tests {
    use super::Sha256Digest;
    use crate::error::ModelError;

    const VALID: &str = "39d87dec0138df1d96638f60fa89e66b30afba6a2c5ee29d7a87233b2c45bda7";

    #[test]
    fn accepts_only_lowercase_hex_of_full_length() {
        let digest = Sha256Digest::parse(VALID).expect("valid digest");
        assert_eq!(digest.as_str(), VALID);
        assert_eq!(digest.to_string(), VALID);

        for rejected in [
            VALID.to_uppercase().as_str(),
            &VALID[..63],
            "zz",
            "",
            &format!("{VALID}0"),
        ] {
            assert!(
                matches!(
                    Sha256Digest::parse(rejected),
                    Err(ModelError::InvalidDigest { .. })
                ),
                "{rejected:?} must be rejected"
            );
        }
    }
}
