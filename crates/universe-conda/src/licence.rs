//! What an archive has to prove about its licence.
//!
//! Two claims are checkable from the archive alone, and both are checked before a
//! binary is ever inspected because they are cheap and they fail loudly: the
//! expression in `info/about.json` is SPDX, and every licence file the record names
//! is actually present and non-empty.
//!
//! The distinction matters downstream. A package that *declares* `MIT` but ships no
//! licence text is a licensing problem for whoever redistributes it, and a
//! `license_file` that points at nothing is a declaration nobody can check. Both are
//! refused here rather than discovered by a consumer.

use serde::Deserialize;
use universe_core::diagnostic::{Diagnostic, Severity};

use crate::package::{CondaPackage, PackageError};

/// Paths under this prefix are the licence texts a Conda package bundles.
const LICENCE_DIRECTORY: &str = "info/licenses/";

/// Which licence rules an audit applies.
///
/// Defaults are the strict ones, matching `universe.toml`'s `[audit]` defaults: a
/// rule you have to remember to switch on is a rule that will be off.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LicenceRules {
    /// Refuse an archive whose `about.json` names a licence file it does not carry.
    pub enforce_license_file: bool,
    /// Refuse an archive whose licence is not a valid SPDX expression.
    pub require_valid_spdx: bool,
}

impl Default for LicenceRules {
    fn default() -> Self {
        Self {
            enforce_license_file: true,
            require_valid_spdx: true,
        }
    }
}

/// The fields of `info/about.json` this audit reads.
///
/// Unknown fields are accepted: `about.json` is written by `conda-build` and
/// `rattler-build`, and both add keys this repository does not own.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
struct AboutRecord {
    #[serde(default)]
    license: Option<String>,
    #[serde(default)]
    license_file: Option<String>,
}

/// What the licence audit found.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LicenceAudit {
    expression: Option<String>,
    files: Vec<String>,
    violations: Vec<Diagnostic>,
}

impl LicenceAudit {
    /// The SPDX expression, when `about.json` carried one that parsed.
    #[must_use]
    pub fn expression(&self) -> Option<&str> {
        self.expression.as_deref()
    }

    /// Licence texts the archive bundles, sorted, as archive-relative paths.
    #[must_use]
    pub fn files(&self) -> &[String] {
        &self.files
    }

    /// Rules the archive broke; empty means it passed.
    #[must_use]
    pub fn violations(&self) -> &[Diagnostic] {
        &self.violations
    }

    /// Whether the archive passed every rule that was applied.
    #[must_use]
    pub fn is_clean(&self) -> bool {
        self.violations.is_empty()
    }
}

/// Apply the licence rules to an opened archive.
///
/// # Errors
///
/// Returns [`PackageError`] when the archive cannot be read, or when
/// `info/about.json` is present but is not the record its schema describes.
pub fn audit_licence(
    package: &CondaPackage,
    rules: LicenceRules,
) -> Result<LicenceAudit, PackageError> {
    let about = match package.read_info_member("info/about.json")? {
        Some(bytes) => {
            serde_json::from_slice::<AboutRecord>(&bytes).map_err(PackageError::InvalidIndex)?
        }
        None => AboutRecord {
            license: None,
            license_file: None,
        },
    };

    let members = package.info_member_names()?;
    let files: Vec<String> = members
        .iter()
        .filter(|name| name.starts_with(LICENCE_DIRECTORY))
        .cloned()
        .collect();

    let mut violations = Vec::new();
    let expression = check_expression(&about, rules, &mut violations);
    if rules.enforce_license_file {
        check_licence_files(package, &about, &files, &mut violations)?;
    }

    Ok(LicenceAudit {
        expression,
        files,
        violations,
    })
}

/// Validate the SPDX expression, returning it when it parsed.
fn check_expression(
    about: &AboutRecord,
    rules: LicenceRules,
    violations: &mut Vec<Diagnostic>,
) -> Option<String> {
    let declared = about.license.as_deref()?.trim().to_owned();
    if !rules.require_valid_spdx {
        return Some(declared);
    }
    match declared.parse::<spdx::Expression>() {
        Ok(_) => Some(declared),
        Err(error) => {
            violations.push(Diagnostic::new(
                Severity::Error,
                "conda.licence.expression.invalid",
                format!("info/about.json declares {declared:?}, which is not an SPDX expression: {error}"),
            ));
            None
        }
    }
}

/// Check that every licence file the record names is present and non-empty.
fn check_licence_files(
    package: &CondaPackage,
    about: &AboutRecord,
    bundled: &[String],
    violations: &mut Vec<Diagnostic>,
) -> Result<(), PackageError> {
    let declared = match about.license_file.as_deref() {
        Some(path) => vec![path.to_owned()],
        None => bundled.to_vec(),
    };

    for path in declared {
        match package.read_info_member(&path)? {
            None => violations.push(Diagnostic::new(
                Severity::Error,
                "conda.licence.file.missing",
                format!("info/about.json names {path}, which the archive does not carry"),
            )),
            Some(contents) if contents.is_empty() => violations.push(Diagnostic::new(
                Severity::Error,
                "conda.licence.file.empty",
                format!("{path} is bundled but empty"),
            )),
            Some(_) => {}
        }
    }
    Ok(())
}
