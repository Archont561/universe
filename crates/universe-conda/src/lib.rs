//! The Conda adapter.
//!
//! This crate owns everything that is true of Conda and false of every other
//! ecosystem: how a `.conda` archive is laid out, which `info/` records identify
//! it, what its binaries are allowed to link against, and which channel metadata
//! a solver reads. `universe-core` owns the neutral vocabulary the results are
//! reported in, and knows none of this.

/// Version of this component, equal to the workspace version the whole artifact set
/// is released under (`decision-122`: one tag, one version train).
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

mod binary;
mod licence;
mod package;
mod recipe;

pub use binary::{BinaryAudit, BinaryError, BinaryFormat, BinaryRules, audit_binary};
pub use licence::{LicenceAudit, LicenceRules, audit_licence};
pub use package::{CondaPackage, PackageError, PackageIndex, open_package};
pub use recipe::{RecipeRules, lint_recipe};
