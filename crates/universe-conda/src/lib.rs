//! The Conda adapter.
//!
//! This crate owns everything that is true of Conda and false of every other
//! ecosystem: how a `.conda` archive is laid out, which `info/` records identify
//! it, what its binaries are allowed to link against, and which channel metadata
//! a solver reads. `universe-core` owns the neutral vocabulary the results are
//! reported in, and knows none of this.

mod binary;
mod licence;
mod package;

pub use binary::{BinaryAudit, BinaryError, BinaryFormat, BinaryRules, audit_binary};
pub use licence::{LicenceAudit, LicenceRules, audit_licence};
pub use package::{CondaPackage, PackageError, PackageIndex, open_package};
