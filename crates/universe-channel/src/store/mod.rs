//! Uploading artifacts to a destination that is not a Git branch.
//!
//! A branch transport puts a whole tree in one place; the destinations here take one file at
//! a time at an address the *destination* decides. That difference is the entire module: a
//! [`Target`] composes the address and the method, a [`Remote`] executes it, and [`apply`]
//! decides, before anything is sent, whether the upload is needed at all.
//!
//! Three rules, each of them a way a publish gets things wrong:
//!
//! - **An unchanged file is not re-sent.** A destination that reports the digest it already
//!   holds compares equal, and the upload is skipped. A retried job then costs a `HEAD` per
//!   artifact instead of a full transfer.
//! - **A changed file under an occupied name is refused, not overwritten.** Two builds of
//!   `foo-1.0.0-0.conda` in one channel is an index that disagrees with itself. Only `--force`
//!   asks for the overwrite, and the refusal names both digests.
//! - **A dry run performs no mutating request.** Reads are allowed, because the plan cannot
//!   be built without them; every `PUT`, `POST` and `DELETE` is withheld. That is what makes
//!   `--dry-run` a preview of the real run rather than a description of the code.

use std::collections::BTreeMap;

mod filesystem;
mod http;
mod releases;

pub use filesystem::FilesystemRemote;
pub use http::{HttpRemote, Method, Request};
pub use releases::{Asset, Release, ReleaseReport, Releases};

/// One place bytes go, as the publisher needs to describe it.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Target {
    /// Store name, as the manifest declared it.
    pub name: String,
    /// Which protocol this destination speaks.
    pub kind: Kind,
    /// Base URL without a trailing slash, or a directory for [`Kind::Filesystem`].
    pub endpoint: String,
    /// Channel, owner or repository the destination addresses.
    pub channel: Option<String>,
    /// Bearer token, when the destination authenticates.
    pub token: Option<String>,
}

/// The destination protocols this module can write to.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Kind {
    /// A hosted Conda channel on prefix.dev.
    PrefixDev,
    /// Anaconda.org.
    Anaconda,
    /// An S3 bucket.
    S3,
    /// A self-hosted Quetz instance.
    Quetz,
    /// JFrog Artifactory.
    Artifactory,
    /// A local directory.
    Filesystem,
    /// GitHub Releases assets.
    Releases,
}

impl Kind {
    /// Whether an artifact's bytes go to a directory this process can write.
    #[must_use]
    pub const fn is_local(self) -> bool {
        matches!(self, Self::Filesystem)
    }

    /// The HTTP method one upload uses.
    ///
    /// A registry that assigns its own identity takes a `POST`; a store that addresses a
    /// file by its path takes a `PUT`. Getting this wrong is a 405 at publish time, which is
    /// why it is decided here and asserted per kind.
    #[must_use]
    pub const fn method(self) -> Method {
        match self {
            Self::PrefixDev | Self::Anaconda | Self::Quetz => Method::Post,
            Self::S3 | Self::Artifactory | Self::Filesystem | Self::Releases => Method::Put,
        }
    }
}

/// One artifact, with the digest that identifies it.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Upload {
    /// Path inside the channel, slash-separated: `<subdir>/<file name>`.
    pub path: String,
    /// The bytes.
    pub bytes: Vec<u8>,
    /// SHA-256 of `bytes`, lowercase hexadecimal.
    pub sha256: String,
}

impl Upload {
    /// The artifact, from its channel path and bytes.
    #[must_use]
    pub fn new(path: impl Into<String>, bytes: Vec<u8>) -> Self {
        let sha256 = digest(&bytes);
        Self {
            path: path.into(),
            bytes,
            sha256,
        }
    }

    /// The file name alone, which is what a registry addresses an artifact by.
    #[must_use]
    pub fn file_name(&self) -> &str {
        self.path.rsplit('/').next().unwrap_or(&self.path)
    }

    /// The subdir alone, which is what a Conda registry files the artifact under.
    #[must_use]
    pub fn subdir(&self) -> &str {
        self.path.split_once('/').map_or("", |(subdir, _)| subdir)
    }
}

impl Target {
    /// The path one upload takes at this destination, relative to its endpoint.
    ///
    /// # Errors
    ///
    /// Returns [`StoreError::Incomplete`] when a destination that needs a channel was
    /// declared without one — the alternative is a request to the wrong owner's namespace.
    pub fn location(&self, upload: &Upload) -> Result<String, StoreError> {
        let channel = self.channel_needed()?;
        let location = match self.kind {
            // The registry takes the file name and files it under the channel it was told.
            Kind::PrefixDev => format!("api/v1/upload/{channel}"),
            Kind::Anaconda => format!("upload/{channel}"),
            Kind::Quetz => format!("api/channels/{channel}/upload/{}", upload.subdir()),
            Kind::Artifactory => format!("{channel}/{}", upload.path),
            // Path-style addressing: the endpoint carries the bucket.
            Kind::S3 => format!("{channel}/{}", upload.path),
            Kind::Filesystem | Kind::Releases => upload.path.clone(),
        };
        Ok(location)
    }

    fn channel_needed(&self) -> Result<&str, StoreError> {
        match self.channel.as_deref() {
            Some(channel) if !channel.trim().is_empty() => Ok(channel),
            _ => Err(StoreError::Incomplete {
                store: self.name.clone(),
                missing: "channel",
            }),
        }
    }
}

/// What a destination already holds at one path.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Object {
    /// Digest the destination reports, when it reports one at all.
    pub sha256: Option<String>,
    /// Size in bytes, when the destination reports one.
    pub size: Option<u64>,
}

/// A destination that can hold one file at a path.
pub trait Remote {
    /// What the destination holds at `path`, or `None` when it holds nothing.
    ///
    /// # Errors
    ///
    /// Returns [`StoreError`] when the destination cannot be read.
    fn head(&self, path: &str) -> Result<Option<Object>, StoreError>;

    /// Put `bytes` at `path`, replacing whatever is there.
    ///
    /// # Errors
    ///
    /// Returns [`StoreError`] when the destination refuses or the transfer fails.
    fn put(&self, path: &str, bytes: &[u8]) -> Result<(), StoreError>;
}

/// Whether a publish may replace a file whose digest changed.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Policy {
    /// Replace a file whose digest differs from the one being published.
    pub force: bool,
    /// Plan and read, but never write.
    pub dry_run: bool,
}

/// What a publish did, or would do.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Report {
    /// Uploads that were sent.
    pub uploaded: Vec<String>,
    /// Uploads the destination already held, byte for byte.
    pub unchanged: Vec<String>,
    /// Bytes actually sent.
    pub bytes: u64,
    /// Whether nothing was written.
    pub dry_run: bool,
}

impl Report {
    /// Whether the destination is already exactly what was asked for.
    #[must_use]
    pub fn is_noop(&self) -> bool {
        self.uploaded.is_empty()
    }
}

/// Publish `uploads` to `remote` under `policy`.
///
/// # Errors
///
/// Returns [`StoreError::Refused`] when a file is already present under a different digest
/// and `force` is not set, so the destination is never quietly rewritten.
pub fn apply(
    remote: &dyn Remote,
    uploads: &[Upload],
    policy: Policy,
) -> Result<Report, StoreError> {
    let mut report = Report {
        dry_run: policy.dry_run,
        ..Report::default()
    };
    for upload in uploads {
        match remote.head(&upload.path)? {
            Some(existing) if existing.sha256.as_deref() == Some(upload.sha256.as_str()) => {
                report.unchanged.push(upload.path.clone());
            }
            Some(existing) if !policy.force => {
                return Err(StoreError::Refused {
                    path: upload.path.clone(),
                    published: upload.sha256.clone(),
                    present: existing
                        .sha256
                        .unwrap_or_else(|| "an unknown digest".to_owned()),
                });
            }
            _ => {
                if !policy.dry_run {
                    remote.put(&upload.path, &upload.bytes)?;
                    report.bytes += upload.bytes.len() as u64;
                }
                report.uploaded.push(upload.path.clone());
            }
        }
    }
    Ok(report)
}

/// Why a publish to a destination failed.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum StoreError {
    /// The destination could not be reached, or answered with an error status.
    #[error("{url}: {message}")]
    Transport {
        /// What was being called.
        url: String,
        /// What went wrong.
        message: String,
    },

    /// A file already exists under a different digest.
    #[error(
        "'{path}' is already published with digest {present}, but this build is {published}; \
         pass --force to replace it"
    )]
    Refused {
        /// The contested path.
        path: String,
        /// The digest this build would publish.
        published: String,
        /// The digest the destination holds.
        present: String,
    },

    /// The store was declared without something its protocol needs.
    #[error("store '{store}' does not declare a {missing}")]
    Incomplete {
        /// The store.
        store: String,
        /// What it is missing.
        missing: &'static str,
    },

    /// The endpoint uses a scheme this build cannot speak.
    #[error(
        "store '{store}' points at '{endpoint}', whose scheme this build cannot speak: {reason}"
    )]
    Unsupported {
        /// The store.
        store: String,
        /// Its endpoint.
        endpoint: String,
        /// Why the scheme is unusable.
        reason: &'static str,
    },

    /// The destination's answer could not be understood.
    #[error("{url}: {message}")]
    Response {
        /// What was called.
        url: String,
        /// What was wrong with the answer.
        message: String,
    },

    /// The destination is a directory this process could not read or write.
    #[error("{path}: {message}")]
    Io {
        /// The file or directory.
        path: String,
        /// What the operating system said.
        message: String,
    },
}

/// SHA-256 of `bytes`, lowercase hexadecimal.
#[must_use]
pub fn digest(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hasher
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

/// Group uploads by the version they claim, for retention.
///
/// A channel's release holds one asset per build; retention keeps the newest `keep` builds of
/// each version and prunes the rest. Grouping by `(package, version)` rather than by file
/// name is what makes `foo-1.0.0-3` survive while `foo-1.0.0-0` goes.
#[must_use]
pub fn versions(uploads: &[Upload]) -> BTreeMap<(String, String), Vec<String>> {
    let mut grouped: BTreeMap<(String, String), Vec<String>> = BTreeMap::new();
    for upload in uploads {
        let (package, version) = split_name(upload.file_name());
        grouped
            .entry((package, version))
            .or_default()
            .push(upload.file_name().to_owned());
    }
    grouped
}

/// Split `<package>-<version>-<build>.conda` into its package and version.
///
/// Conda file names put the build last, so the last two dashes are the split points; a
/// package or version containing a dash still lands correctly, because only the tail is cut.
#[must_use]
pub fn split_name(file_name: &str) -> (String, String) {
    let stem = file_name
        .strip_suffix(".conda")
        .or_else(|| file_name.strip_suffix(".tar.bz2"))
        .unwrap_or(file_name);
    let parts: Vec<&str> = stem.rsplitn(3, '-').collect();
    match parts.as_slice() {
        [_, version, package] => ((*package).to_owned(), (*version).to_owned()),
        _ => (stem.to_owned(), String::new()),
    }
}
