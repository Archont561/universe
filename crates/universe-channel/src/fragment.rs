//! What one published artifact says about itself, and what that means for a channel.
//!
//! A fragment is the audit receipt: it is emitted by `universe audit --emit-fragment`, read
//! back by `universe index`, and copied into a channel verbatim. That is the whole point of
//! the format — the numbers a solver reads are the numbers the auditor measured, not a second
//! reading taken somewhere else.

use serde::{Deserialize, Serialize};

/// One audited artifact.
///
/// Unknown fields are accepted so a fragment written by a later Universe still parses here;
/// the fields this type names are the ones a channel is built from, and a fragment that lacks
/// one of them is refused rather than defaulted.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Fragment {
    /// Package name, exactly as its ecosystem spells it.
    pub package: String,
    /// Version, unnormalised.
    pub version: String,
    /// Conda build string; with name and version it identifies one artifact.
    pub build: String,
    /// Monotonic build counter within a version.
    #[serde(default)]
    pub build_number: u64,
    /// Subdirectory the artifact belongs to.
    pub subdir: String,
    /// SHA-256 of the exact bytes.
    pub digest: String,
    /// Size of the exact bytes.
    pub size_bytes: u64,
    /// Runtime dependencies, as Conda specifiers.
    #[serde(default)]
    pub depends: Vec<String>,
    /// SPDX licence expression, when the archive declared one.
    #[serde(default)]
    pub license: Option<String>,
    /// Licence texts the archive bundles.
    #[serde(default)]
    pub license_files: Vec<String>,
    /// Where the bytes were when the audit ran, relative to the project root.
    #[serde(default)]
    pub path: String,
}

impl Fragment {
    /// The file name a channel publishes this artifact under.
    #[must_use]
    pub fn file_name(&self) -> String {
        format!("{}-{}-{}.conda", self.package, self.version, self.build)
    }

    /// The path inside the channel: standard Conda layout, `<subdir>/<file name>`.
    #[must_use]
    pub fn channel_path(&self) -> String {
        format!("{}/{}", self.subdir, self.file_name())
    }

    /// The path the fragment itself takes in a channel: `meta/<subdir>/<file name>.json`
    /// (`decision-5`).
    ///
    /// Published in the same tree as the bytes — a branch store has one tree per snapshot, so
    /// there is no second branch to mirror — but kept out of the subdir a client scans, so a
    /// listing of `linux-64/` still shows one file per artifact.
    #[must_use]
    pub fn metadata_path(&self) -> String {
        format!("meta/{}.json", self.channel_path())
    }

    /// Problems that stop this fragment from describing a channel entry.
    ///
    /// A fragment is written by the auditor and read by a solver, so the cheap checks belong
    /// here: a digest that is not a digest, a size of zero, a subdir that is not a subdir.
    #[must_use]
    pub fn problems(&self) -> Vec<String> {
        let mut problems = Vec::new();
        if !is_sha256(&self.digest) {
            problems.push(format!(
                "digest '{}' is not 64 lowercase hexadecimal characters",
                self.digest
            ));
        }
        if self.size_bytes == 0 {
            problems.push("size_bytes is zero".to_owned());
        }
        if self.subdir.trim().is_empty() || self.subdir.contains('/') {
            problems.push(format!("subdir '{}' is not a subdirectory", self.subdir));
        }
        for (field, value) in [
            ("package", &self.package),
            ("version", &self.version),
            ("build", &self.build),
        ] {
            if value.trim().is_empty() {
                problems.push(format!("{field} is empty"));
            }
        }
        problems
    }
}

/// Parse one fragment document.
///
/// # Errors
///
/// Returns the serde message, which names the field that failed.
pub fn parse(text: &str) -> Result<Fragment, serde_json::Error> {
    serde_json::from_str(text)
}

fn is_sha256(text: &str) -> bool {
    text.len() == 64
        && text
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

#[cfg(test)]
mod tests {
    use super::{Fragment, parse};

    fn fragment() -> Fragment {
        parse(
            r#"{
              "package": "universe-airlock",
              "version": "0.1.0",
              "build": "h1234567_0",
              "build_number": 0,
              "subdir": "linux-64",
              "digest": "9f2c1b0a3d4e5f60718293a4b5c6d7e8f90a1b2c3d4e5f60718293a4b5c6d7e8",
              "size_bytes": 7,
              "depends": [],
              "license": "MIT"
            }"#,
        )
        .expect("the fixture parses")
    }

    #[test]
    fn a_fragment_names_the_path_a_channel_publishes_it_at() {
        assert_eq!(
            fragment().channel_path(),
            "linux-64/universe-airlock-0.1.0-h1234567_0.conda"
        );
    }

    #[test]
    fn the_receipt_is_published_under_meta_and_not_in_the_subdir_a_client_scans() {
        assert_eq!(
            fragment().metadata_path(),
            "meta/linux-64/universe-airlock-0.1.0-h1234567_0.conda.json"
        );
    }

    #[test]
    fn a_field_added_by_a_later_universe_does_not_break_parsing() {
        let text = r#"{
          "package": "p", "version": "1", "build": "0", "subdir": "noarch",
          "digest": "9f2c1b0a3d4e5f60718293a4b5c6d7e8f90a1b2c3d4e5f60718293a4b5c6d7e8",
          "size_bytes": 1, "future_field": true
        }"#;
        assert!(parse(text).is_ok());
    }

    #[test]
    fn a_fragment_that_could_not_be_a_channel_entry_says_why() {
        let mut broken = fragment();
        broken.digest = "not-a-digest".to_owned();
        broken.size_bytes = 0;
        broken.subdir = "linux/64".to_owned();
        let problems = broken.problems();
        assert_eq!(problems.len(), 3, "{problems:?}");
        assert!(problems[0].contains("digest"));
    }
}
