//! What an audit can prove about the binaries inside a package.

mod support;

use support::{EM_AARCH64, EM_X86_64, elf64};
use universe_conda::{BinaryRules, audit_binary};

fn codes(violations: &[universe_core::diagnostic::Diagnostic]) -> Vec<&str> {
    violations
        .iter()
        .map(|violation| violation.code())
        .collect()
}

#[test]
fn an_elf_built_for_its_platform_is_accepted_and_its_dependencies_reported() {
    let bytes = elf64(
        EM_X86_64,
        &["libc.so.6", "libz.so.1"],
        Some("$ORIGIN/../lib"),
    );

    let audit = audit_binary(
        "linux-64",
        "lib/libairlock.so",
        &bytes,
        BinaryRules::default(),
    )
    .expect("inspect")
    .expect("an ELF is a binary");

    assert_eq!(audit.format().as_str(), "elf");
    assert_eq!(audit.architecture(), "x86_64");
    assert_eq!(audit.needed(), ["libc.so.6", "libz.so.1"]);
    assert!(audit.violations().is_empty(), "{:?}", audit.violations());
}

#[test]
fn an_elf_for_another_architecture_is_refused() {
    // The machine type says aarch64; the package says linux-64.
    let bytes = elf64(EM_AARCH64, &["libc.so.6"], None);

    let audit = audit_binary(
        "linux-64",
        "lib/libairlock.so",
        &bytes,
        BinaryRules::default(),
    )
    .expect("inspect")
    .expect("an ELF is a binary");

    assert_eq!(
        codes(audit.violations()),
        ["conda.binary.architecture.mismatch"]
    );
}

#[test]
fn an_absolute_runpath_is_a_build_path_leak() {
    let bytes = elf64(
        EM_X86_64,
        &["libc.so.6"],
        Some("/home/runner/work/universe/build/lib"),
    );

    let audit = audit_binary(
        "linux-64",
        "lib/libairlock.so",
        &bytes,
        BinaryRules::default(),
    )
    .expect("inspect")
    .expect("an ELF is a binary");

    assert_eq!(codes(audit.violations()), ["conda.binary.rpath.leak"]);
    assert!(
        audit.violations()[0]
            .message()
            .contains("/home/runner/work"),
        "{}",
        audit.violations()[0].message()
    );
}

#[test]
fn a_native_binary_inside_a_noarch_package_is_refused() {
    let bytes = elf64(EM_X86_64, &["libc.so.6"], None);

    let audit = audit_binary("noarch", "bin/helper", &bytes, BinaryRules::default())
        .expect("inspect")
        .expect("an ELF is a binary");

    assert_eq!(codes(audit.violations()), ["conda.binary.noarch.forbidden"]);
}

#[test]
fn a_script_is_not_a_binary() {
    let audit = audit_binary(
        "noarch",
        "bin/activate",
        b"#!/bin/sh\nexport PATH=\"$CONDA_PREFIX/bin:$PATH\"\n",
        BinaryRules::default(),
    )
    .expect("inspect");

    assert!(audit.is_none());
}
