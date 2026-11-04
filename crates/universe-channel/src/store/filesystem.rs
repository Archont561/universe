//! A channel that is a directory on this machine.
//!
//! The simplest [`Remote`], and the one that makes the rest testable: a local directory has
//! no auth, no eventual consistency and no rate limit, so a test that fails here is failing
//! on the publisher's logic rather than on the destination's behaviour.
//!
//! Two rules keep a publish from escaping the channel it was given. Paths are joined only
//! after being refused if they contain anything but plain components, so `../etc/passwd` is
//! an error and not a write; and a file is written to a temporary sibling and renamed, so a
//! reader never observes half an artifact.

use std::fs::{self, File};
use std::io::Write as _;
use std::path::{Component, Path, PathBuf};

use super::{Object, Remote, StoreError, digest};

/// A channel rooted at a directory.
#[derive(Clone, Debug)]
pub struct FilesystemRemote {
    root: PathBuf,
}

impl FilesystemRemote {
    /// A remote for the directory `root`, which need not exist yet.
    #[must_use]
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    /// The directory this remote writes into.
    #[must_use]
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// The absolute path one channel path takes.
    ///
    /// # Errors
    ///
    /// Returns [`StoreError::Unsupported`] when `path` is not a plain relative path.
    pub fn resolve(&self, path: &str) -> Result<PathBuf, StoreError> {
        let relative = Path::new(path);
        for component in relative.components() {
            let plain = matches!(component, Component::Normal(_));
            if !plain {
                return Err(StoreError::Unsupported {
                    store: self.root.display().to_string(),
                    endpoint: path.to_owned(),
                    reason: "a channel path may only contain plain components",
                });
            }
        }
        Ok(self.root.join(relative))
    }
}

impl Remote for FilesystemRemote {
    fn head(&self, path: &str) -> Result<Option<Object>, StoreError> {
        let absolute = self.resolve(path)?;
        let metadata = match fs::metadata(&absolute) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(error) => {
                return Err(StoreError::Io {
                    path: absolute.display().to_string(),
                    message: error.to_string(),
                });
            }
        };
        if !metadata.is_file() {
            return Ok(None);
        }
        let bytes = fs::read(&absolute).map_err(|error| StoreError::Io {
            path: absolute.display().to_string(),
            message: error.to_string(),
        })?;
        Ok(Some(Object {
            sha256: Some(digest(&bytes)),
            size: Some(metadata.len()),
        }))
    }

    fn put(&self, path: &str, bytes: &[u8]) -> Result<(), StoreError> {
        let absolute = self.resolve(path)?;
        let parent = absolute.parent().map(Path::to_path_buf);
        if let Some(parent) = parent {
            fs::create_dir_all(&parent).map_err(|error| StoreError::Io {
                path: parent.display().to_string(),
                message: error.to_string(),
            })?;
        }
        // Write beside the destination and rename, so the artifact appears whole.
        let staging = absolute.with_extension("part");
        let mut file = File::create(&staging).map_err(|error| StoreError::Io {
            path: staging.display().to_string(),
            message: error.to_string(),
        })?;
        file.write_all(bytes).map_err(|error| StoreError::Io {
            path: staging.display().to_string(),
            message: error.to_string(),
        })?;
        file.sync_all().map_err(|error| StoreError::Io {
            path: staging.display().to_string(),
            message: error.to_string(),
        })?;
        drop(file);
        fs::rename(&staging, &absolute).map_err(|error| StoreError::Io {
            path: absolute.display().to_string(),
            message: error.to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::FilesystemRemote;
    use crate::store::{Remote, StoreError};

    #[test]
    fn a_plain_relative_path_resolves_beneath_the_root() {
        let directory = tempfile::tempdir().expect("a scratch directory");
        let remote = FilesystemRemote::new(directory.path());

        let resolved = remote
            .resolve("linux-64/foo-1.0.0-0.conda")
            .expect("a plain path");

        assert_eq!(
            resolved,
            directory.path().join("linux-64/foo-1.0.0-0.conda")
        );
        assert_eq!(remote.root(), directory.path());
    }

    #[test]
    fn an_absolute_path_is_refused() {
        let directory = tempfile::tempdir().expect("a scratch directory");
        let remote = FilesystemRemote::new(directory.path());

        let error = remote
            .resolve("/etc/passwd")
            .expect_err("an absolute path escapes the channel");

        assert!(matches!(error, StoreError::Unsupported { .. }), "{error:?}");
    }

    #[test]
    fn a_directory_where_an_artifact_should_be_counts_as_nothing_published() {
        let directory = tempfile::tempdir().expect("a scratch directory");
        let remote = FilesystemRemote::new(directory.path());
        std::fs::create_dir_all(directory.path().join("linux-64/foo-1.0.0-0.conda"))
            .expect("a directory in the artifact's place");

        let object = remote
            .head("linux-64/foo-1.0.0-0.conda")
            .expect("the lookup succeeds");

        assert!(object.is_none());
    }

    #[test]
    fn a_nested_parent_directory_is_created_before_the_artifact() {
        let directory = tempfile::tempdir().expect("a scratch directory");
        let remote = FilesystemRemote::new(directory.path());

        remote
            .put("noarch/sub/foo-1.0.0-0.conda", b"bytes")
            .expect("the artifact is written");

        assert_eq!(
            std::fs::read(directory.path().join("noarch/sub/foo-1.0.0-0.conda"))
                .expect("the artifact"),
            b"bytes"
        );
    }
}
