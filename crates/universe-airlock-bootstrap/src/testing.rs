//! A scratch directory for tests.
//!
//! `tempfile` is not a dependency of this crate, and adding one would change `Cargo.lock` for
//! the sake of a test helper. This is the minimum that is safe: a unique name under the system
//! temp directory, and removal on drop. Tests that need a payload never touch the real
//! filesystem outside it.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

static COUNTER: AtomicU32 = AtomicU32::new(0);

/// A directory that deletes itself.
#[derive(Debug)]
pub struct Scratch {
    path: PathBuf,
}

impl Scratch {
    /// Create an empty directory with a name no other test in this process shares.
    ///
    /// # Panics
    ///
    /// Panics when the directory cannot be created: a test that cannot build its fixture has
    /// nothing to assert, and failing here names the reason.
    #[must_use]
    pub fn new(label: &str) -> Self {
        let unique = COUNTER.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "universe-airlock-bootstrap-{}-{label}-{unique}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).expect("create a scratch directory");
        Self { path }
    }

    /// The directory's path.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Write a file inside it, creating parent directories.
    pub fn write(&self, relative: &str, bytes: &[u8]) -> PathBuf {
        let target = self.path.join(relative);
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent).expect("create a parent directory");
        }
        std::fs::write(&target, bytes).expect("write a fixture file");
        target
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
    }
}
