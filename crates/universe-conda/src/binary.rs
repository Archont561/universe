//! What an audit can prove about a compiled binary, without running it.
//!
//! Every check here is a header read: the machine type in the file matches the
//! subdirectory the package claims, the search path does not point at the machine
//! that built it, and a `noarch` package holds no compiled code at all. Nothing
//! executes the binary, links against it, or shells out to `ldd`, `otool`, or
//! `dumpbin`, so the auditor can inspect a `win-64` or `osx-arm64` package from a
//! Linux runner — which is exactly what a single-runner release needs.

use goblin::Object;
use universe_core::diagnostic::{Diagnostic, Severity};

/// The object formats an audit recognises.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BinaryFormat {
    /// ELF, as Linux packages carry.
    Elf,
    /// PE, as Windows packages carry.
    Pe,
    /// Mach-O, as macOS packages carry.
    MachO,
}

impl BinaryFormat {
    /// Stable lowercase spelling, as records and diagnostics write it.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Elf => "elf",
            Self::Pe => "pe",
            Self::MachO => "macho",
        }
    }
}

/// Which binary rules an audit applies. Defaults are the strict ones.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BinaryRules {
    /// Refuse a binary whose search path names an absolute build location.
    pub forbid_build_path_leak: bool,
    /// Refuse a binary whose machine type does not match its subdirectory.
    pub verify_arch_headers: bool,
}

impl Default for BinaryRules {
    fn default() -> Self {
        Self {
            forbid_build_path_leak: true,
            verify_arch_headers: true,
        }
    }
}

/// A binary the audit could not read.
#[derive(Debug, thiserror::Error)]
pub enum BinaryError {
    /// The file has a binary's magic but does not parse as one.
    #[error("{path} looks like a binary but does not parse: {reason}")]
    Unparsable {
        /// Path inside the package.
        path: String,
        /// What the parser said.
        reason: String,
    },
}

/// What the audit found in one binary.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BinaryAudit {
    path: String,
    format: BinaryFormat,
    architecture: String,
    needed: Vec<String>,
    rpaths: Vec<String>,
    violations: Vec<Diagnostic>,
}

impl BinaryAudit {
    /// Path inside the package, as the archive spells it.
    #[must_use]
    pub fn path(&self) -> &str {
        &self.path
    }

    /// Which object format the file is.
    #[must_use]
    pub fn format(&self) -> BinaryFormat {
        self.format
    }

    /// The machine type the file declares.
    #[must_use]
    pub fn architecture(&self) -> &str {
        &self.architecture
    }

    /// Dynamic libraries the loader is told to bring in.
    #[must_use]
    pub fn needed(&self) -> &[String] {
        &self.needed
    }

    /// Search paths the file records, whether or not they are acceptable.
    #[must_use]
    pub fn rpaths(&self) -> &[String] {
        &self.rpaths
    }

    /// Rules this binary broke; empty means it passed.
    #[must_use]
    pub fn violations(&self) -> &[Diagnostic] {
        &self.violations
    }
}

/// Inspect one file from a package, returning `None` when it is not a binary.
///
/// # Errors
///
/// Returns [`BinaryError`] when the file carries a binary's magic but cannot be
/// parsed: a corrupt binary is a finding, not an ordinary script.
pub fn audit_binary(
    platform: &str,
    path: &str,
    bytes: &[u8],
    rules: BinaryRules,
) -> Result<Option<BinaryAudit>, BinaryError> {
    let parsed = match Object::parse(bytes) {
        Ok(object) => object,
        Err(error) if looks_like_binary(bytes) => {
            return Err(BinaryError::Unparsable {
                path: path.to_owned(),
                reason: error.to_string(),
            });
        }
        Err(_) => return Ok(None),
    };

    let (format, architecture, needed, rpaths) = match &parsed {
        Object::Elf(elf) => {
            // `DT_RPATH` and `DT_RUNPATH` are different entries with the same
            // consequence for a leak check, and goblin keeps them apart because a
            // loader treats them differently. An audit cares about the union.
            let mut rpaths = strings(&elf.rpaths);
            rpaths.extend(strings(&elf.runpaths));
            (
                BinaryFormat::Elf,
                elf_machine(elf.header.e_machine),
                strings(&elf.libraries),
                rpaths,
            )
        }
        // PE and Mach-O arrive with their own slices; until then a file of either
        // format is reported by name rather than silently passed.
        Object::PE(_) => (BinaryFormat::Pe, String::new(), Vec::new(), Vec::new()),
        Object::Mach(_) => (BinaryFormat::MachO, String::new(), Vec::new(), Vec::new()),
        _ => return Ok(None),
    };

    let mut violations = Vec::new();
    if platform == "noarch" {
        violations.push(Diagnostic::new(
            Severity::Error,
            "conda.binary.noarch.forbidden",
            format!(
                "{path} is a {} binary inside a noarch package",
                format.as_str()
            ),
        ));
    } else {
        if rules.verify_arch_headers
            && let Some(expected) = expected_architecture(platform)
            && !architecture.is_empty()
            && architecture != expected
        {
            violations.push(Diagnostic::new(
                Severity::Error,
                "conda.binary.architecture.mismatch",
                format!("{path} is built for {architecture}, but {platform} carries {expected}"),
            ));
        }
        if rules.forbid_build_path_leak {
            for rpath in &rpaths {
                if rpath.starts_with('/') {
                    violations.push(Diagnostic::new(
                        Severity::Error,
                        "conda.binary.rpath.leak",
                        format!("{path} records the absolute search path {rpath}"),
                    ));
                }
            }
        }
    }
    Ok(Some(BinaryAudit {
        path: path.to_owned(),
        format,
        architecture,
        needed,
        rpaths,
        violations,
    }))
}

/// The machine type a subdirectory's packages are built for.
fn expected_architecture(platform: &str) -> Option<&'static str> {
    match platform {
        "linux-64" | "win-64" | "osx-64" => Some("x86_64"),
        "linux-aarch64" | "osx-arm64" => Some("aarch64"),
        _ => None,
    }
}

/// Whether the bytes carry one of the magics a binary starts with.
fn looks_like_binary(bytes: &[u8]) -> bool {
    const MAGICS: [&[u8]; 4] = [b"\x7fELF", b"MZ", b"\xfe\xed\xfa\xce", b"\xcf\xfa\xed\xfe"];
    MAGICS.iter().any(|magic| bytes.starts_with(magic))
}

/// The machine type an ELF header declares, in the vocabulary a subdirectory uses.
fn elf_machine(machine: u16) -> String {
    match machine {
        goblin::elf::header::EM_X86_64 => "x86_64".to_owned(),
        goblin::elf::header::EM_AARCH64 => "aarch64".to_owned(),
        goblin::elf::header::EM_386 => "x86".to_owned(),
        other => format!("machine-{other}"),
    }
}

/// Own the string table entries goblin borrows from the file's bytes.
fn strings(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| (*value).to_owned()).collect()
}
