//! Where a publish stages its scratch repository.
//!
//! Publication needs a working tree somewhere: git writes files, stages them, and asks for a
//! tree id. That directory must not be inside the payload — a channel tree that contains its
//! own staging area is a channel whose next publish would include the previous one's leftovers
//! — so it is created under the system temporary directory by default, and removed when the
//! publish ends, whether it succeeded or not.
//!
//! `--output` still names one explicitly, which is what a CI job diagnosing a failed push
//! wants; the same containment rule applies to it.
//!
//! No dependency: `tempfile` is a test-only crate here, and this needs a unique name and a
//! `Drop`, both of which are a dozen lines. Removing the directory is best-effort — a publish
//! that succeeded should not fail because a scratch file could not be unlinked.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use universe_core::diagnostic::{Diagnostic, Severity};

/// A directory the publisher stages into for one run.
#[derive(Debug)]
pub(crate) struct Scratch {
    path: PathBuf,
    /// Whether this run created it. A directory that already existed belongs to the caller,
    /// so it is left where it was rather than deleted on the way out.
    created: bool,
}

impl Scratch {
    /// A fresh directory for this run, under the system temporary directory.
    ///
    /// # Errors
    ///
    /// Returns a diagnostic when the directory cannot be created, naming the path: a publish
    /// with nowhere to stage is a publish that has not started.
    pub(crate) fn temporary() -> Result<Self, Diagnostic> {
        static SEQUENCE: AtomicU64 = AtomicU64::new(0);
        let name = format!(
            "universe-publish-{}-{}",
            std::process::id(),
            SEQUENCE.fetch_add(1, Ordering::Relaxed)
        );
        Self::create(std::env::temp_dir().join(name))
    }

    /// A named directory for this run, created if it does not exist.
    ///
    /// # Errors
    ///
    /// Returns a diagnostic when the directory cannot be created.
    pub(crate) fn named(path: &Path) -> Result<Self, Diagnostic> {
        Self::create(path.to_path_buf())
    }

    fn create(path: PathBuf) -> Result<Self, Diagnostic> {
        let created = !path.exists();
        std::fs::create_dir_all(&path).map_err(|error| {
            Diagnostic::new(
                Severity::Error,
                "publish.scratch.unwritable",
                format!(
                    "cannot create the staging directory {}: {error}",
                    path.display()
                ),
            )
        })?;
        Ok(Self { path, created })
    }

    /// The directory a transport stages into.
    pub(crate) fn path(&self) -> &Path {
        &self.path
    }

    /// Refuse a staging directory that sits inside the payload it would publish.
    ///
    /// The payload is where the artifacts live; staging *there* would make the next publish
    /// read its predecessor's working tree. `root` is the project root and `artifacts` are the
    /// files a fragment names, so this asks the narrow question — is the scratch inside a
    /// directory that already holds an artifact — rather than the broad one.
    ///
    /// # Errors
    ///
    /// Returns a diagnostic naming both paths when it does.
    pub(crate) fn ensure_outside(
        scratch: &Path,
        root: &Path,
        artifacts: &[PathBuf],
    ) -> Result<(), Diagnostic> {
        let scratch = absolute(scratch);
        for artifact in artifacts {
            let Some(directory) = absolute(&root.join(artifact))
                .parent()
                .map(Path::to_path_buf)
            else {
                continue;
            };
            if scratch.starts_with(&directory) || directory == scratch {
                return Err(Diagnostic::new(
                    Severity::Error,
                    "publish.scratch.inside-payload",
                    format!(
                        "the staging directory {} is inside {}, which holds the artifacts this \
                         publish would ship; stage somewhere outside the payload — the default \
                         is a temporary directory",
                        scratch.display(),
                        directory.display()
                    ),
                ));
            }
        }
        Ok(())
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        // Best effort, and nothing to report: a leftover scratch directory in the system
        // temporary directory is the operating system's problem, and failing a successful
        // publish over it would be the wrong trade.
        if self.created {
            let _ = std::fs::remove_dir_all(&self.path);
        }
    }
}

/// An absolute form of `path`, without touching the filesystem.
fn absolute(path: &Path) -> PathBuf {
    let joined = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join(path)
    };
    // A lexical normalisation, not `canonicalize`: the directory may not exist yet, and the
    // question is about the path a caller wrote rather than the one the filesystem resolves.
    let mut normal = PathBuf::new();
    for component in joined.components() {
        match component {
            std::path::Component::ParentDir => {
                normal.pop();
            }
            std::path::Component::CurDir => {}
            other => normal.push(other),
        }
    }
    normal
}

#[cfg(test)]
mod tests {
    use super::Scratch;
    use std::path::{Path, PathBuf};

    #[test]
    fn a_temporary_scratch_exists_while_it_is_alive_and_not_after() {
        let scratch = Scratch::temporary().expect("a scratch directory");
        let path = scratch.path().to_path_buf();
        assert!(path.is_dir(), "{} was not created", path.display());
        drop(scratch);
        assert!(
            !path.exists(),
            "{} outlived the publish that owned it",
            path.display()
        );
    }

    #[test]
    fn two_scratches_in_one_process_do_not_collide() {
        let first = Scratch::temporary().expect("a scratch directory");
        let second = Scratch::temporary().expect("a scratch directory");
        assert_ne!(first.path(), second.path());
    }

    #[test]
    fn a_directory_the_caller_named_is_left_where_it_was() {
        let parent = tempfile::tempdir().expect("a temporary parent");
        let path = parent.path().join("staging");
        std::fs::create_dir(&path).expect("a directory the caller made first");
        let scratch = Scratch::named(&path).expect("a scratch directory");
        assert_eq!(scratch.path(), path);
        drop(scratch);
        assert!(path.is_dir(), "a pre-existing directory is the caller's");
    }

    #[test]
    fn staging_inside_a_directory_that_holds_artifacts_is_refused() {
        let root = Path::new("/project");
        let artifacts = vec![PathBuf::from("output/example.conda")];
        let error = Scratch::ensure_outside(Path::new("/project/output/publish"), root, &artifacts)
            .expect_err("refused");
        assert_eq!(error.code(), "publish.scratch.inside-payload");
        assert!(error.message().contains("/project/output"));
    }

    #[test]
    fn a_temporary_directory_outside_the_project_is_accepted() {
        let root = Path::new("/project");
        let artifacts = vec![PathBuf::from("output/example.conda")];
        assert!(
            Scratch::ensure_outside(Path::new("/tmp/universe-publish-1"), root, &artifacts).is_ok()
        );
    }
}
