//! Fixtures for the Conda adapter's tests.
//!
//! A `.conda` is a zip of two zstd-compressed tars, and building one here rather
//! than committing a binary spells out the format the reader has to satisfy. The
//! archives are deterministic: stored members, a fixed timestamp, and no
//! compression-level surprises, so two runs produce the same bytes.

#![allow(dead_code)]

use std::fs::File;
use std::io::Write;
use std::path::Path;

/// An `info/index.json` a fixture can start from, with the fields a real one has.
pub const INDEX: &str = r#"{
  "name": "universe-airlock",
  "version": "0.1.0",
  "build": "h1234567_0",
  "build_number": 0,
  "subdir": "linux-64",
  "license": "MIT",
  "depends": ["libgcc >=13"],
  "platform": "linux",
  "arch": "x86_64"
}"#;

/// Write a `.conda` whose info section holds `files` and whose payload is empty.
pub fn write_conda(path: &Path, files: &[(&str, &[u8])]) {
    let file = File::create(path).expect("create archive");
    let mut zip = zip::ZipWriter::new(file);
    let options =
        zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);

    zip.start_file("metadata.json", options)
        .expect("metadata entry");
    zip.write_all(br#"{"conda_pkg_format_version": 2}"#)
        .expect("metadata bytes");

    zip.start_file("info-fixture.tar.zst", options)
        .expect("info entry");
    zip.write_all(&compress(&tar_bytes(files)))
        .expect("info bytes");

    zip.start_file("pkg-fixture.tar.zst", options)
        .expect("pkg entry");
    zip.write_all(&compress(&tar_bytes(&[])))
        .expect("pkg bytes");

    zip.finish().expect("finish archive");
}

fn compress(tar: &[u8]) -> Vec<u8> {
    zstd::encode_all(tar, 0).expect("compress tar")
}

/// A tar in memory, one file per pair, with fixed ownership and timestamps.
fn tar_bytes(files: &[(&str, &[u8])]) -> Vec<u8> {
    let mut builder = tar::Builder::new(Vec::new());
    for (name, contents) in files {
        let mut header = tar::Header::new_gnu();
        header.set_size(contents.len() as u64);
        header.set_mode(0o644);
        header.set_mtime(0);
        header.set_cksum();
        builder
            .append_data(&mut header, *name, *contents)
            .expect("append entry");
    }
    builder.into_inner().expect("finish tar")
}

/// An ELF64 shared object, built field by field.
///
/// Real linker output is not needed and would not be reproducible across hosts:
/// what the auditor reads is the header's machine type, the dynamic string table,
/// and the `DT_NEEDED`/`DT_RUNPATH` entries. Everything is little-endian and every
/// virtual address equals its file offset, so a single `PT_LOAD` maps the whole
/// file and goblin's address translation is exercised rather than bypassed.
pub fn elf64(machine: u16, needed: &[&str], runpath: Option<&str>) -> Vec<u8> {
    const EHDR: usize = 64;
    const PHDR: usize = 56;
    const PHNUM: usize = 2;

    // The dynamic string table, with each entry's offset recorded as it is added.
    let mut dynstr = Vec::new();
    let mut offsets = Vec::new();
    for text in needed {
        offsets.push((1i64, dynstr.len() as u64));
        dynstr.extend_from_slice(text.as_bytes());
        dynstr.push(0);
    }
    let runpath_offset = runpath.map(|text| {
        let offset = dynstr.len() as u64;
        dynstr.extend_from_slice(text.as_bytes());
        dynstr.push(0);
        offset
    });

    let strtab_at = EHDR + PHDR * PHNUM;
    let dynamic_at = (strtab_at + dynstr.len() + 7) & !7;

    let mut dynamic: Vec<(i64, u64)> = vec![(5i64, strtab_at as u64), (10i64, dynstr.len() as u64)];
    dynamic.extend(offsets);
    if let Some(offset) = runpath_offset {
        dynamic.push((29, offset));
    }
    dynamic.push((0, 0));
    let total = dynamic_at + dynamic.len() * 16;

    let mut bytes = Vec::with_capacity(total);
    bytes.extend_from_slice(&[0x7f, b'E', b'L', b'F', 2, 1, 1, 0]);
    bytes.extend_from_slice(&[0; 8]);
    push_u16(&mut bytes, 3); // ET_DYN
    push_u16(&mut bytes, machine);
    push_u32(&mut bytes, 1); // EV_CURRENT
    push_u64(&mut bytes, 0); // e_entry
    push_u64(&mut bytes, EHDR as u64); // e_phoff
    push_u64(&mut bytes, 0); // e_shoff
    push_u32(&mut bytes, 0); // e_flags
    push_u16(&mut bytes, EHDR as u16);
    push_u16(&mut bytes, PHDR as u16);
    push_u16(&mut bytes, PHNUM as u16);
    push_u16(&mut bytes, 0); // e_shentsize
    push_u16(&mut bytes, 0); // e_shnum
    push_u16(&mut bytes, 0); // e_shstrndx

    // PT_LOAD covering the file, then PT_DYNAMIC pointing at the entries.
    for (kind, offset, size) in [(1u32, 0usize, total), (2, dynamic_at, dynamic.len() * 16)] {
        push_u32(&mut bytes, kind);
        push_u32(&mut bytes, 4 | 2); // PF_R | PF_W
        push_u64(&mut bytes, offset as u64);
        push_u64(&mut bytes, offset as u64);
        push_u64(&mut bytes, offset as u64);
        push_u64(&mut bytes, size as u64);
        push_u64(&mut bytes, size as u64);
        push_u64(&mut bytes, 8);
    }

    bytes.extend_from_slice(&dynstr);
    bytes.resize(dynamic_at, 0);
    for (tag, value) in dynamic {
        push_u64(&mut bytes, tag as u64);
        push_u64(&mut bytes, value);
    }
    bytes
}

/// `EM_X86_64`, as `ELF`'s header spells a 64-bit Intel machine.
pub const EM_X86_64: u16 = 62;
/// `EM_AARCH64`.
pub const EM_AARCH64: u16 = 183;

fn push_u16(bytes: &mut Vec<u8>, value: u16) {
    bytes.extend_from_slice(&value.to_le_bytes());
}

fn push_u32(bytes: &mut Vec<u8>, value: u32) {
    bytes.extend_from_slice(&value.to_le_bytes());
}

fn push_u64(bytes: &mut Vec<u8>, value: u64) {
    bytes.extend_from_slice(&value.to_le_bytes());
}
