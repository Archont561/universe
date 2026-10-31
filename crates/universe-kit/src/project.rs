//! One project root, loaded once, so every verb reads the manifest the same way.

use std::path::{Path, PathBuf};

use universe_core::CommandOutput;
use universe_core::diagnostic::{Diagnostic, Severity};
use universe_core::manifest::Manifest;

use crate::output::{USAGE, failure};

/// Default manifest file name, read from the project root.
pub(crate) const MANIFEST_FILE: &str = "universe.toml";

/// A project root, its manifest text, and the parsed manifest.
pub(crate) struct Project {
    /// Directory the manifest was read from.
    pub(crate) root: PathBuf,
    /// Parsed manifest.
    pub(crate) manifest: Manifest,
}

impl Project {
    /// Read and parse the manifest at `root`.
    ///
    /// Returns the failure output rather than a diagnostic so the caller's only decision is
    /// where to put it — an unreadable or unparsable manifest is always a usage failure.
    pub(crate) fn load(root: &Path) -> Result<Self, CommandOutput> {
        let path = root.join(MANIFEST_FILE);
        let Ok(text) = std::fs::read_to_string(&path) else {
            return Err(failure(
                USAGE,
                &[Diagnostic::new(
                    Severity::Error,
                    "manifest.missing",
                    format!("no {} at this root", path.display()),
                )
                .with_path(path.display().to_string())],
            ));
        };
        match Manifest::from_toml(&text) {
            Ok(manifest) => Ok(Self {
                root: root.to_path_buf(),
                manifest,
            }),
            Err(diagnostics) => Err(failure(USAGE, &diagnostics)),
        }
    }

    /// Refuse a manifest that does not validate.
    ///
    /// Every verb that acts on a manifest calls this first: planning a build from a manifest
    /// that names an unknown store, or building one whose package publishes nowhere it can
    /// reach, would succeed at the wrong thing.
    pub(crate) fn require_valid(&self) -> Result<(), CommandOutput> {
        let errors: Vec<Diagnostic> = self
            .manifest
            .validate()
            .into_iter()
            .filter(Diagnostic::is_error)
            .collect();
        if errors.is_empty() {
            Ok(())
        } else {
            Err(failure(USAGE, &errors))
        }
    }
}
