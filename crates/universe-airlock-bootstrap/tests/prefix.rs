//! Installing one archive into a prefix.
//!
//! `UNI-52` criterion 1 is a prefix that runs something; criterion 3 is relocation evidence; and
//! criterion 5 is that no resolver and no channel metadata request got involved. This file tests
//! the module that does the work, with archives built here rather than downloaded: a `.conda` is
//! a zip of two zstd-compressed tars, and a builder that spells the format out is a statement of
//! what the reader has to satisfy.
//!
//! The digests in every fixture's `info/paths.json` are computed here with `sha2` directly, not
//! with the module's own hasher, so a mistake in that hasher cannot agree with itself.

use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};
use tempfile::TempDir;

use universe_airlock_bootstrap::prefix::{Installed, materialise};

/// The build-time prefix a relocatable fixture carries.
const BUILD_PREFIX: &str = "/opt/conda/conda-bld/universe_1234/_h_env_placehold";

/// One entry a fixture archive carries.
#[derive(Clone)]
enum Entry {
    File {
        path: &'static str,
        bytes: Vec<u8>,
        mode: u32,
        placeholder: Option<&'static str>,
        file_mode: Option<&'static str>,
    },
    Symlink {
        path: &'static str,
        target: &'static str,
    },
    Hardlink {
        path: &'static str,
        target: &'static str,
    },
    Dir {
        path: &'static str,
    },
    /// An entry type an installer cannot install, to prove the refusal.
    Fifo {
        path: &'static str,
    },
}

/// How a test departs from a well-formed package.
#[derive(Default)]
struct Deviation {
    /// Record this digest and size instead of the bytes the archive carries.
    record: Option<(String, String, u64)>,
    /// Carry this entry in the payload without recording it.
    extra: Option<(&'static str, Vec<u8>)>,
    /// Record this name without carrying it.
    phantom: Option<String>,
    /// Rewrite this entry's recorded link type, to make it disagree with the archive.
    record_type: Option<(String, String)>,
    /// Leave this file out of the info tar.
    omit_info: Option<&'static str>,
    /// Write these bytes as the whole archive file, to test something that is not a `.conda`.
    raw: Option<Vec<u8>>,
    /// Write `info/paths.json` as text that is not the record this build reads.
    bad_paths: bool,
    /// Write a second payload member, which the format does not allow.
    duplicate_member: bool,
    /// Write the payload member as bytes that are not zstd.
    corrupt_payload: bool,
}

/// A `.conda` written under a temporary directory, with a prefix to install into.
struct Fixture {
    directory: TempDir,
    package: PathBuf,
}

impl Fixture {
    /// A well-formed package carrying `entries`.
    fn new(entries: Vec<Entry>, noarch: Option<&str>) -> Self {
        Self::build_in(
            tempfile::tempdir().expect("temp dir"),
            entries,
            noarch,
            Deviation::default(),
        )
    }

    /// The same package, recording `bytes` as the installed content of `path`.
    ///
    /// This is how a relocatable file is described: `paths.json` states what the file looks like
    /// *after* the prefix is rewritten.
    fn recorded(entries: Vec<Entry>, path: &str, bytes: &[u8]) -> Self {
        Self::build_in(
            tempfile::tempdir().expect("temp dir"),
            entries,
            None,
            Deviation {
                record: Some((path.to_owned(), digest(bytes), bytes.len() as u64)),
                ..Deviation::default()
            },
        )
    }

    /// The same package, carrying an entry its `paths.json` does not name.
    fn with_extra_payload(entries: Vec<Entry>, path: &'static str, bytes: &[u8]) -> Self {
        Self::build_in(
            tempfile::tempdir().expect("temp dir"),
            entries,
            None,
            Deviation {
                extra: Some((path, bytes.to_vec())),
                ..Deviation::default()
            },
        )
    }

    /// The same package, naming a file it does not carry.
    fn with_phantom(entries: Vec<Entry>, path: &str) -> Self {
        Self::build_in(
            tempfile::tempdir().expect("temp dir"),
            entries,
            None,
            Deviation {
                phantom: Some(path.to_owned()),
                ..Deviation::default()
            },
        )
    }

    /// Build into a directory the caller owns, so a test can know the prefix path before it
    /// writes the record that describes what will be installed there.
    fn build_in(
        directory: TempDir,
        entries: Vec<Entry>,
        noarch: Option<&str>,
        deviation: Deviation,
    ) -> Self {
        let package = directory.path().join("fixture-0.1.0-h1234567_0.conda");

        let mut index = serde_json::json!({
            "name": "fixture",
            "version": "0.1.0",
            "build": "h1234567_0",
            "build_number": 0,
            "subdir": "linux-64",
            "depends": [],
        });
        if let Some(flavour) = noarch {
            index["noarch"] = serde_json::json!(flavour);
            index["subdir"] = serde_json::json!("noarch");
        }

        let mut records: Vec<serde_json::Value> = Vec::new();
        for entry in &entries {
            match entry {
                Entry::File {
                    path,
                    bytes,
                    placeholder,
                    file_mode,
                    ..
                } => {
                    let (sha256, size) = match &deviation.record {
                        Some((recorded, recorded_digest, size)) if recorded == path => {
                            (recorded_digest.clone(), *size)
                        }
                        _ => (digest(bytes), bytes.len() as u64),
                    };
                    let mut record = serde_json::json!({
                        "_path": path,
                        "path_type": "hardlink",
                        "sha256": sha256,
                        "size_in_bytes": size,
                    });
                    if let Some(placeholder) = placeholder {
                        record["prefix_placeholder"] = serde_json::json!(placeholder);
                    }
                    if let Some(file_mode) = file_mode {
                        record["file_mode"] = serde_json::json!(file_mode);
                    }
                    records.push(record);
                }
                Entry::Symlink { path, .. } => records.push(serde_json::json!({
                    "_path": path,
                    "path_type": "softlink",
                })),
                Entry::Hardlink { path, .. } => records.push(serde_json::json!({
                    "_path": path,
                    "path_type": "hardlink",
                })),
                Entry::Dir { path } => records.push(serde_json::json!({
                    "_path": path,
                    "path_type": "directory",
                })),
                Entry::Fifo { path } => records.push(serde_json::json!({
                    "_path": path,
                    "path_type": "hardlink",
                })),
            }
        }
        // A phantom is named and not carried; an extra is carried and not named. Neither is
        // added to the other side, which is the whole point of the two deviations.
        if let Some(path) = &deviation.phantom {
            records.push(serde_json::json!({
                "_path": path,
                "path_type": "hardlink",
                "sha256": digest(b"never shipped"),
                "size_in_bytes": 13,
            }));
        }

        let mut carried = payload_entries(&entries);
        if let Some((path, bytes)) = &deviation.extra {
            carried.push(((*path).to_owned(), bytes.clone(), 0o644));
        }

        if let Some((path, kind)) = &deviation.record_type {
            for record in &mut records {
                if record["_path"] == serde_json::json!(path) {
                    record["path_type"] = serde_json::json!(kind);
                }
            }
        }
        if let Some(raw) = &deviation.raw {
            std::fs::write(&package, raw).expect("write the raw archive");
            return Self { directory, package };
        }

        let mut info: Vec<(&str, Vec<u8>)> = Vec::new();
        if deviation.omit_info != Some("info/index.json") {
            info.push(("info/index.json", index.to_string().into_bytes()));
        }
        if deviation.omit_info != Some("info/paths.json") {
            let paths = if deviation.bad_paths {
                "{this is not the record}".to_owned()
            } else {
                serde_json::json!({"paths": records, "paths_version": 1}).to_string()
            };
            info.push(("info/paths.json", paths.into_bytes()));
        }
        let info = tar_of(&info);
        if deviation.corrupt_payload || deviation.duplicate_member {
            write_archive_raw(
                &package,
                &info,
                if deviation.corrupt_payload {
                    b"not zstd at all".to_vec()
                } else {
                    compress(&payload_tar(&carried, &entries))
                },
                deviation.duplicate_member,
            );
        } else {
            write_archive(&package, &info, &payload_tar(&carried, &entries));
        }

        Self { directory, package }
    }

    /// Create the prefix directory this fixture installs into.
    fn prefix(&self) -> PathBuf {
        let prefix = self.directory.path().join("prefix");
        std::fs::create_dir_all(&prefix).expect("prefix");
        prefix
    }

    fn install(&self) -> Result<Installed, String> {
        let prefix = self.prefix();
        materialise(&self.package, &prefix).map_err(|finding| format!("{finding:?}"))
    }
}

fn digest(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    let mut text = String::new();
    for byte in hasher.finalize() {
        text.push_str(&format!("{byte:02x}"));
    }
    text
}

fn compress(tar: &[u8]) -> Vec<u8> {
    zstd::encode_all(tar, 0).expect("compress tar")
}

fn tar_of(files: &[(&str, Vec<u8>)]) -> Vec<u8> {
    let mut builder = tar::Builder::new(Vec::new());
    for (name, contents) in files {
        let mut header = tar::Header::new_gnu();
        header.set_size(contents.len() as u64);
        header.set_mode(0o644);
        header.set_mtime(0);
        header.set_cksum();
        builder
            .append_data(&mut header, *name, contents.as_slice())
            .expect("append entry");
    }
    builder.into_inner().expect("finish tar")
}

/// The entries a payload tar must carry, with their bytes and modes.
fn payload_entries(entries: &[Entry]) -> Vec<(String, Vec<u8>, u32)> {
    entries
        .iter()
        .filter_map(|entry| match entry {
            Entry::File {
                path, bytes, mode, ..
            } => Some(((*path).to_owned(), bytes.clone(), *mode)),
            _ => None,
        })
        .collect()
}

fn payload_tar(files: &[(String, Vec<u8>, u32)], entries: &[Entry]) -> Vec<u8> {
    let mut builder = tar::Builder::new(Vec::new());
    for (path, bytes, mode) in files {
        let mut header = tar::Header::new_gnu();
        header.set_size(bytes.len() as u64);
        header.set_mode(*mode);
        header.set_mtime(0);
        header.set_cksum();
        builder
            .append_data(&mut header, path.as_str(), bytes.as_slice())
            .expect("append file");
    }
    for entry in entries {
        let (path, kind, target) = match entry {
            Entry::Symlink { path, target } => (*path, tar::EntryType::Symlink, Some(target)),
            Entry::Hardlink { path, target } => (*path, tar::EntryType::Link, Some(target)),
            Entry::Dir { path } => (*path, tar::EntryType::Directory, None),
            Entry::Fifo { path } => (*path, tar::EntryType::Fifo, None),
            Entry::File { .. } => continue,
        };
        let mut header = tar::Header::new_gnu();
        header.set_entry_type(kind);
        header.set_size(0);
        header.set_mode(0o777);
        header.set_mtime(0);
        if let Some(target) = target {
            header.set_link_name(target).expect("link name");
        }
        header.set_cksum();
        builder
            .append_data(&mut header, path, std::io::empty())
            .expect("append entry");
    }
    builder.into_inner().expect("finish tar")
}

/// The same writer, with the payload member's bytes and count under the caller's control.
fn write_archive_raw(package: &Path, info: &[u8], payload: Vec<u8>, duplicate: bool) {
    write_archive_members(package, info, &payload, duplicate);
}

fn write_archive(package: &Path, info: &[u8], payload: &[u8]) {
    write_archive_members(package, info, &compress(payload), false);
}

fn write_archive_members(package: &Path, info: &[u8], payload: &[u8], duplicate: bool) {
    let file = std::fs::File::create(package).expect("create archive");
    let mut zip = zip::ZipWriter::new(file);
    let options =
        zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
    zip.start_file("metadata.json", options).expect("metadata");
    zip.write_all(br#"{"conda_pkg_format_version": 2}"#)
        .expect("metadata bytes");
    zip.start_file("info-fixture.tar.zst", options)
        .expect("info");
    zip.write_all(&compress(info)).expect("info bytes");
    zip.start_file("pkg-fixture.tar.zst", options)
        .expect("payload");
    zip.write_all(payload).expect("payload bytes");
    if duplicate {
        zip.start_file("pkg-other.tar.zst", options)
            .expect("second payload");
        zip.write_all(payload).expect("second payload bytes");
    }
    zip.finish().expect("finish archive");
}

/// A native-shaped fixture: an executable, a data file, and a symlink beside them.
fn native() -> Vec<Entry> {
    vec![
        Entry::File {
            path: "bin/tool",
            bytes: b"#!/bin/sh\nprintf 'tool 0.1.0\\n'\n".to_vec(),
            mode: 0o755,
            placeholder: None,
            file_mode: None,
        },
        Entry::File {
            path: "share/doc/note.txt",
            bytes: b"a note the package ships\n".to_vec(),
            mode: 0o644,
            placeholder: None,
            file_mode: None,
        },
        Entry::Symlink {
            path: "bin/alias",
            target: "tool",
        },
    ]
}

#[test]
fn a_native_archive_installs_its_files_at_the_modes_it_recorded() {
    let fixture = Fixture::new(native(), None);
    let installed = fixture.install().expect("install");

    assert_eq!(installed.entries, 3, "two files and a symlink");
    let prefix = fixture.prefix();

    // The executable bit is the whole reason a restored prefix can run anything.
    let tool = prefix.join("bin/tool");
    let mode = std::fs::metadata(&tool).expect("tool").permissions().mode() & 0o777;
    assert_eq!(mode, 0o755, "bin/tool was installed as {mode:o}");
    assert_eq!(
        std::fs::read(&tool).expect("tool bytes"),
        b"#!/bin/sh\nprintf 'tool 0.1.0\\n'\n"
    );

    // A symlink, not a copy: a package shipping both would otherwise install the bytes twice
    // and the two would diverge the first time one of them is patched.
    let alias = prefix.join("bin/alias");
    assert_eq!(
        std::fs::read_link(&alias).expect("alias is a symlink"),
        PathBuf::from("tool")
    );
    assert_eq!(
        std::fs::read(&alias).expect("through the symlink"),
        std::fs::read(&tool).expect("tool bytes")
    );
}

#[test]
fn a_text_file_carrying_the_build_prefix_is_rewritten_to_the_install_prefix() {
    // Criterion 3's failure mode: a file that still names the build host restores an environment
    // that reads from paths that do not exist on the target.
    let placeholder = "/opt/conda";
    let directory = tempfile::tempdir().expect("temp dir");
    let prefix = directory.path().join("prefix");
    std::fs::create_dir_all(&prefix).expect("prefix");
    let expected = format!("PYTHONPATH={}/lib\n", prefix.display()).into_bytes();
    let fixture = Fixture::build_in(
        directory,
        vec![Entry::File {
            path: "etc/profile.d/universe.sh",
            bytes: format!("PYTHONPATH={placeholder}/lib\n").into_bytes(),
            mode: 0o644,
            placeholder: Some(placeholder),
            file_mode: Some("text"),
        }],
        None,
        Deviation {
            record: Some((
                "etc/profile.d/universe.sh".to_owned(),
                digest(&expected),
                expected.len() as u64,
            )),
            ..Deviation::default()
        },
    );

    fixture.install().expect("install");

    let installed = std::fs::read(prefix.join("etc/profile.d/universe.sh")).expect("installed");
    assert_eq!(installed, expected, "the placeholder was not rewritten");
    assert!(
        !String::from_utf8_lossy(&installed).contains(placeholder),
        "the installed file still names the build prefix"
    );
}

#[test]
fn a_binary_placeholder_is_padded_so_every_offset_stays_where_the_linker_put_it() {
    let mut original = b"\x7fELF".to_vec();
    original.extend_from_slice(BUILD_PREFIX.as_bytes());
    original.extend_from_slice(b"\0tail");

    let directory = tempfile::tempdir().expect("temp dir");
    let prefix = directory.path().join("prefix");
    std::fs::create_dir_all(&prefix).expect("prefix");
    let short = prefix.to_string_lossy().into_owned();
    assert!(
        short.len() < BUILD_PREFIX.len(),
        "this fixture needs an install prefix shorter than the build one"
    );
    // Padded the way the reference installer patches: same width, NUL-terminated.
    let mut expected = b"\x7fELF".to_vec();
    expected.extend_from_slice(short.as_bytes());
    expected.extend(std::iter::repeat_n(0u8, BUILD_PREFIX.len() - short.len()));
    expected.extend_from_slice(b"\0tail");
    assert_eq!(
        expected.len(),
        original.len(),
        "a binary patch is fixed width"
    );

    let fixture = Fixture::build_in(
        directory,
        vec![Entry::File {
            path: "lib/librelocatable.so",
            bytes: original.clone(),
            mode: 0o755,
            placeholder: Some(BUILD_PREFIX),
            file_mode: Some("binary"),
        }],
        None,
        Deviation {
            record: Some((
                "lib/librelocatable.so".to_owned(),
                digest(&expected),
                expected.len() as u64,
            )),
            ..Deviation::default()
        },
    );
    fixture.install().expect("install");

    let installed = std::fs::read(prefix.join("lib/librelocatable.so")).expect("installed");
    assert_eq!(
        installed.len(),
        original.len(),
        "the installed binary changed size, so its offsets moved"
    );
    assert_eq!(installed, expected);
}

#[test]
fn an_unlisted_file_in_the_archive_is_refused() {
    // A package carrying payload nobody recorded is a package whose bytes nothing checked.
    let fixture =
        Fixture::with_extra_payload(native(), "share/unlisted.bin", b"nobody recorded me");
    let message = fixture.install().expect_err("an unlisted entry is refused");

    assert!(
        message.contains("airlock.prefix.unlisted"),
        "the refusal does not name the unlisted file: {message}"
    );
}

#[test]
fn a_name_the_archive_does_not_carry_is_refused() {
    let fixture = Fixture::with_phantom(native(), "share/doc/missing.txt");
    let message = fixture
        .install()
        .expect_err("a name with no payload is refused");

    assert!(
        message.contains("airlock.prefix.missing"),
        "the refusal is not about the absent file: {message}"
    );
    assert!(
        message.contains("share/doc/missing.txt"),
        "the refusal does not say which file: {message}"
    );
}

#[test]
fn a_file_whose_bytes_do_not_match_the_record_is_refused() {
    // Same length, different bytes: the size check passes and the digest is what refuses.
    let fixture = Fixture::recorded(
        vec![Entry::File {
            path: "share/doc/note.txt",
            bytes: b"a note the package ships\n".to_vec(),
            mode: 0o644,
            placeholder: None,
            file_mode: None,
        }],
        "share/doc/note.txt",
        b"a note the package SHIPS\n",
    );
    let message = fixture.install().expect_err("a digest mismatch is refused");

    assert!(
        message.contains("airlock.prefix.digest"),
        "the refusal is not about the digest: {message}"
    );
    assert!(
        message.contains("share/doc/note.txt"),
        "the refusal does not say which file: {message}"
    );
}

#[test]
fn a_name_that_escapes_the_prefix_is_refused() {
    // `info/paths.json` is the list of what will be written, so a name climbing out of the prefix
    // there is enough to refuse the package: the installer would be writing somewhere the
    // payload never described. (The `tar` writer refuses to *build* such an archive, which is
    // why the fixture states it in the record rather than in the payload.)
    let fixture = Fixture::with_phantom(native(), "../escaped.bin");
    let message = fixture.install().expect_err("a traversal is refused");

    assert!(
        message.contains("airlock.prefix.path"),
        "the refusal is not about the path: {message}"
    );
}

#[test]
fn a_noarch_package_is_refused_rather_than_installed_for_the_wrong_interpreter() {
    // Criterion 4 is open on purpose: a `noarch: python` package installs into the interpreter it
    // is being installed for, and this build has no interpreter to ask. Refusing says so.
    let fixture = Fixture::new(native(), Some("python"));
    let message = fixture.install().expect_err("a noarch package is refused");

    assert!(
        message.contains("airlock.prefix.noarch"),
        "the refusal does not say why: {message}"
    );
    assert!(
        message.contains("python"),
        "the refusal does not name the flavour: {message}"
    );
}

#[test]
fn hardlinks_are_links_and_recorded_directories_are_created() {
    let entries = vec![
        Entry::File {
            path: "lib/libfixture.so.1",
            bytes: b"the shared object\n".to_vec(),
            mode: 0o644,
            placeholder: None,
            file_mode: None,
        },
        Entry::Hardlink {
            path: "lib/libfixture.so",
            target: "lib/libfixture.so.1",
        },
        Entry::Dir {
            path: "share/fixture",
        },
    ];
    let fixture = Fixture::build_in(
        tempfile::tempdir().expect("temp dir"),
        entries,
        None,
        Deviation::default(),
    );
    let prefix = fixture.prefix();

    fixture.install().expect("install");

    // A hardlink shares the bytes: the two names are one file on disk, which is what a package
    // shipping a versioned library and its unversioned name means.
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        let one = std::fs::metadata(prefix.join("lib/libfixture.so.1")).expect("the object");
        let other = std::fs::metadata(prefix.join("lib/libfixture.so")).expect("the link");
        assert_eq!(one.ino(), other.ino(), "the two names are not one file");
    }
    assert!(
        prefix.join("share/fixture").is_dir(),
        "the directory was not created"
    );
}

#[test]
fn something_that_is_not_an_archive_is_refused() {
    let directory = tempfile::tempdir().expect("temp dir");
    let fixture = Fixture::build_in(
        directory,
        native(),
        None,
        Deviation {
            raw: Some(b"this is not a zip file\n".to_vec()),
            ..Deviation::default()
        },
    );

    let message = fixture.install().expect_err("a non-archive is refused");
    assert!(
        message.contains("airlock.prefix.archive"),
        "the refusal is not about the archive: {message}"
    );
}

#[test]
fn an_archive_without_an_index_is_refused() {
    let fixture = Fixture::build_in(
        tempfile::tempdir().expect("temp dir"),
        native(),
        None,
        Deviation {
            omit_info: Some("info/index.json"),
            ..Deviation::default()
        },
    );

    let message = fixture.install().expect_err("no index is refused");
    assert!(
        message.contains("airlock.prefix.manifest"),
        "the refusal is not about the record: {message}"
    );
}

#[test]
fn an_archive_without_paths_json_is_refused() {
    let fixture = Fixture::build_in(
        tempfile::tempdir().expect("temp dir"),
        native(),
        None,
        Deviation {
            omit_info: Some("info/paths.json"),
            ..Deviation::default()
        },
    );

    let message = fixture.install().expect_err("no paths.json is refused");
    assert!(
        message.contains("airlock.prefix.manifest"),
        "the refusal is not about the record: {message}"
    );
}

#[test]
fn a_recorded_link_type_that_disagrees_with_the_archive_is_refused() {
    let fixture = Fixture::build_in(
        tempfile::tempdir().expect("temp dir"),
        native(),
        None,
        Deviation {
            record_type: Some(("share/doc/note.txt".to_owned(), "softlink".to_owned())),
            ..Deviation::default()
        },
    );

    let message = fixture.install().expect_err("a type mismatch is refused");
    assert!(
        message.contains("airlock.prefix.type"),
        "the refusal is not about the type: {message}"
    );
}

#[test]
fn a_placeholder_that_does_not_fit_the_binary_is_refused() {
    // A fixed-width patch cannot express a longer prefix; the reference installer has the same
    // limit, and guessing at a rewrite would corrupt every offset in the file.
    let fixture = Fixture::build_in(
        tempfile::tempdir().expect("temp dir"),
        vec![Entry::File {
            path: "lib/short.so",
            bytes: b"\x7fELF/opt/x\x00".to_vec(),
            mode: 0o755,
            placeholder: Some("/opt/x"),
            file_mode: Some("binary"),
        }],
        None,
        Deviation::default(),
    );

    let message = fixture
        .install()
        .expect_err("an unexpressible patch is refused");
    assert!(
        message.contains("airlock.prefix.length"),
        "the refusal is not about the width: {message}"
    );
}

#[test]
fn a_hardlink_to_something_the_archive_never_installed_is_refused() {
    let fixture = Fixture::build_in(
        tempfile::tempdir().expect("temp dir"),
        vec![Entry::Hardlink {
            path: "lib/dangling.so",
            target: "lib/absent.so",
        }],
        None,
        Deviation::default(),
    );

    let message = fixture
        .install()
        .expect_err("a dangling hardlink is refused");
    assert!(
        message.contains("airlock.prefix.link"),
        "the refusal is not about the link: {message}"
    );
}

#[test]
fn a_symlink_pointing_outside_the_prefix_is_refused() {
    let fixture = Fixture::build_in(
        tempfile::tempdir().expect("temp dir"),
        vec![Entry::Symlink {
            path: "bin/escape",
            target: "../../etc/passwd",
        }],
        None,
        Deviation::default(),
    );

    let message = fixture
        .install()
        .expect_err("an escaping symlink is refused");
    assert!(
        message.contains("airlock.prefix.path"),
        "the refusal is not about the path: {message}"
    );
}

#[test]
fn an_entry_type_this_build_cannot_install_is_refused() {
    let fixture = Fixture::build_in(
        tempfile::tempdir().expect("temp dir"),
        vec![Entry::Fifo {
            path: "run/fixture.sock",
        }],
        None,
        Deviation::default(),
    );

    let message = fixture.install().expect_err("a fifo is refused");
    assert!(
        message.contains("airlock.prefix.entry"),
        "the refusal is not about the entry type: {message}"
    );
}

#[test]
fn an_archive_with_two_payload_members_is_refused() {
    let fixture = Fixture::build_in(
        tempfile::tempdir().expect("temp dir"),
        native(),
        None,
        Deviation {
            duplicate_member: true,
            ..Deviation::default()
        },
    );

    let message = fixture
        .install()
        .expect_err("two payload members are refused");
    assert!(
        message.contains("airlock.prefix.archive"),
        "the refusal is not about the archive: {message}"
    );
    assert!(
        message.contains("exactly one"),
        "the refusal does not say what the format allows: {message}"
    );
}

#[test]
fn a_payload_member_that_is_not_zstd_is_refused() {
    let fixture = Fixture::build_in(
        tempfile::tempdir().expect("temp dir"),
        native(),
        None,
        Deviation {
            corrupt_payload: true,
            ..Deviation::default()
        },
    );

    let message = fixture.install().expect_err("a corrupt payload is refused");
    assert!(
        message.contains("airlock.prefix.archive"),
        "the refusal is not about the archive: {message}"
    );
}

#[test]
fn paths_json_that_is_not_the_record_this_build_reads_is_refused() {
    let fixture = Fixture::build_in(
        tempfile::tempdir().expect("temp dir"),
        native(),
        None,
        Deviation {
            bad_paths: true,
            ..Deviation::default()
        },
    );

    let message = fixture
        .install()
        .expect_err("an unreadable record is refused");
    assert!(
        message.contains("airlock.prefix.manifest"),
        "the refusal is not about the record: {message}"
    );
}

#[test]
fn a_file_whose_size_does_not_match_the_record_is_refused() {
    // Same digest, wrong size: an installer checks both, and a payload that recorded one number
    // wrongly has to be refused rather than trusted on the other.
    let fixture = Fixture::build_in(
        tempfile::tempdir().expect("temp dir"),
        vec![Entry::File {
            path: "share/doc/note.txt",
            bytes: b"a note the package ships\n".to_vec(),
            mode: 0o644,
            placeholder: None,
            file_mode: None,
        }],
        None,
        Deviation {
            record: Some((
                "share/doc/note.txt".to_owned(),
                digest(b"a note the package ships\n"),
                99,
            )),
            ..Deviation::default()
        },
    );

    let message = fixture.install().expect_err("a size mismatch is refused");
    assert!(
        message.contains("airlock.prefix.size"),
        "the refusal is not about the size: {message}"
    );
}
