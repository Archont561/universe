//! Turning a verified archive into an installed prefix.
//!
//! A Conda package is not a tarball that can be dropped in place: `info/paths.json` says which
//! entries are hardlinks, symlinks, or directories, and carries the digest and size every
//! installed file must have. Files built with a prefix placeholder have to be rewritten to the
//! prefix they are installed into, or the environment imports from the build host's paths.
//! That is what this module does, and nothing else: it never resolves, never asks a channel for
//! metadata, and never installs a package the payload did not name (`UNI-52`).
//!
//! Two decisions are worth stating because they are what makes the result checkable:
//!
//! - **The digest is checked after relocation.** `paths.json` describes the file as it will be
//!   installed, so hashing the bytes that landed is the only check that covers the rewrite too.
//! - **Every file in the archive must be named in `paths.json`, and every name in `paths.json`
//!   must appear in the archive.** A package that carries an unlisted payload is a package
//!   whose digest nobody recorded; a name with no payload is an environment missing a file it
//!   was told it has. Both are refused rather than tolerated.
//!
//! What this module does *not* do yet, stated so the gap is visible: `noarch` packages are
//! refused rather than installed (`UNI-52` criterion 4), and it is a replay of one archive at a
//! time — ordering, dependency closure and the lock format are the caller's business.

use std::collections::BTreeSet;
use std::io::Read;
use std::path::{Component, Path, PathBuf};

use serde::Deserialize;

use crate::diagnostics::Finding;
use crate::hash::sha256_hex;

/// What materialising one archive produced.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Installed {
    /// Files, symlinks and directories created.
    pub entries: u64,
    /// Bytes written for regular files.
    pub bytes: u64,
}

/// One entry of `info/paths.json`.
#[derive(Clone, Debug, Deserialize)]
struct PathEntry {
    /// Path inside the prefix, slash-separated.
    #[serde(rename = "_path")]
    path: String,
    /// `hardlink`, `softlink`, or `directory`.
    #[serde(default = "regular")]
    path_type: String,
    /// Digest the installed file must have.
    #[serde(default)]
    sha256: Option<String>,
    /// Size the installed file must have.
    #[serde(default)]
    size_in_bytes: Option<u64>,
    /// Build-time prefix the file carries, when it was built to be relocated.
    #[serde(default)]
    prefix_placeholder: Option<String>,
    /// `text` or `binary`, which decides how a placeholder is rewritten.
    #[serde(default)]
    file_mode: Option<String>,
}

fn regular() -> String {
    "hardlink".to_owned()
}

/// `info/paths.json`.
#[derive(Clone, Debug, Deserialize)]
struct PathsFile {
    paths: Vec<PathEntry>,
}

/// The part of `info/index.json` this module acts on.
#[derive(Clone, Debug, Deserialize)]
struct IndexFile {
    /// `noarch` flavour, when the package is one.
    #[serde(default)]
    noarch: Option<String>,
    /// Subdirectory the package belongs to.
    #[serde(default)]
    subdir: String,
}

/// Install one `.conda` archive into `prefix`.
///
/// `prefix` must already exist and must be the directory the environment will use; the caller
/// owns the transaction, this function owns the contents.
///
/// # Errors
///
/// Returns a finding when the archive cannot be read, is not a `.conda`, carries an entry that
/// escapes the prefix, disagrees with its own `paths.json`, or is `noarch` (which would need the
/// interpreter it is installed for).
pub fn materialise(archive: &Path, prefix: &Path) -> Result<Installed, Finding> {
    let file = std::fs::File::open(archive).map_err(|error| {
        Finding::error(
            "airlock.prefix.archive",
            format!("cannot open {}: {error}", archive.display()),
        )
    })?;
    let mut zip = zip::ZipArchive::new(file).map_err(|error| {
        Finding::error(
            "airlock.prefix.archive",
            format!(
                "{} is not a readable .conda archive: {error}",
                archive.display()
            ),
        )
    })?;

    let mut names: Vec<String> = Vec::new();
    for index in 0..zip.len() {
        let member = zip.by_index(index).map_err(|error| {
            Finding::error(
                "airlock.prefix.archive",
                format!(
                    "cannot read member {index} of {}: {error}",
                    archive.display()
                ),
            )
        })?;
        let name = member.name().map_err(|error| {
            Finding::error(
                "airlock.prefix.archive",
                format!(
                    "member {index} of {} has no readable name: {error}",
                    archive.display()
                ),
            )
        })?;
        names.push(name.into_owned());
    }
    let info_member = single(&names, "info-", archive)?;
    let payload_member = single(&names, "pkg-", archive)?;

    // The info section is small and is read once, in full, before anything is written: a
    // package whose metadata cannot be read must not have placed a single file.
    let info = {
        let mut entry = zip.by_name(&info_member).map_err(|error| {
            Finding::error(
                "airlock.prefix.archive",
                format!(
                    "cannot open {info_member} in {}: {error}",
                    archive.display()
                ),
            )
        })?;
        let mut bytes = Vec::new();
        zstd::stream::read::Decoder::new(&mut entry)
            .and_then(|mut decoder| decoder.read_to_end(&mut bytes))
            .map_err(|error| {
                Finding::error(
                    "airlock.prefix.archive",
                    format!("cannot decompress {info_member}: {error}"),
                )
            })?;
        bytes
    };
    let index = read_member(&info, "info/index.json")
        .and_then(|bytes| serde_json::from_slice::<IndexFile>(&bytes).ok())
        .ok_or_else(|| {
            Finding::error(
                "airlock.prefix.manifest",
                format!("{} carries no readable info/index.json", archive.display()),
            )
        })?;
    if let Some(flavour) = index
        .noarch
        .as_deref()
        .filter(|flavour| !flavour.is_empty())
        .or(Some(index.subdir.as_str()).filter(|subdir| *subdir == "noarch"))
    {
        return Err(Finding::error(
            "airlock.prefix.noarch",
            format!(
                "{} is a noarch ({flavour}) package, which has to be installed for the \
                 interpreter it runs on; this build installs native packages only",
                archive.display()
            ),
        ));
    }

    let paths = read_member(&info, "info/paths.json")
        .ok_or_else(|| {
            Finding::error(
                "airlock.prefix.manifest",
                format!("{} carries no info/paths.json", archive.display()),
            )
        })
        .and_then(|bytes| {
            serde_json::from_slice::<PathsFile>(&bytes).map_err(|error| {
                Finding::error(
                    "airlock.prefix.manifest",
                    format!("info/paths.json is not the record this build reads: {error}"),
                )
            })
        })?;
    let mut recorded: std::collections::BTreeMap<&str, &PathEntry> =
        std::collections::BTreeMap::new();
    for entry in &paths.paths {
        check_relative(&entry.path).map_err(|reason| {
            Finding::error(
                "airlock.prefix.path",
                format!("info/paths.json names `{}`, {reason}", entry.path),
            )
        })?;
        recorded.insert(entry.path.as_str(), entry);
    }
    let expected: BTreeSet<&str> = recorded.keys().copied().collect();
    let mut seen: BTreeSet<&str> = BTreeSet::new();

    let mut installed = Installed::default();
    let mut entry = zip.by_name(&payload_member).map_err(|error| {
        Finding::error(
            "airlock.prefix.archive",
            format!(
                "cannot open {payload_member} in {}: {error}",
                archive.display()
            ),
        )
    })?;
    let decoder = zstd::stream::read::Decoder::new(&mut entry).map_err(|error| {
        Finding::error(
            "airlock.prefix.archive",
            format!("cannot decompress {payload_member}: {error}"),
        )
    })?;
    let mut tar = tar::Archive::new(decoder);
    for item in tar.entries().map_err(|error| streaming(error, archive))? {
        let mut item = item.map_err(|error| streaming(error, archive))?;
        let path = item
            .path()
            .map_err(|error| streaming(error, archive))?
            .to_string_lossy()
            .into_owned();
        check_relative(&path).map_err(|reason| {
            Finding::error(
                "airlock.prefix.path",
                format!("{} carries `{path}`, {reason}", archive.display()),
            )
        })?;
        let kind = item.header().entry_type();
        if kind.is_dir() {
            create_directory(&prefix.join(&path))?;
            // A directory `paths.json` records is satisfied by the archive's own directory
            // entry; one it does not record is simply a parent the archive did not list.
            if let Some(recorded) = recorded.get(path.as_str()) {
                seen.insert(recorded.path.as_str());
            }
            installed.entries += 1;
            continue;
        }
        // Everything that is not a directory has to be named in paths.json, because that is
        // where its digest and its link type are recorded.
        let Some(recorded) = recorded.get(path.as_str()) else {
            return Err(Finding::error(
                "airlock.prefix.unlisted",
                format!(
                    "{} carries `{path}`, which info/paths.json does not name; its bytes were \
                     never recorded",
                    archive.display()
                ),
            ));
        };
        // The recorded link type and the archive's entry type describe the same file. A name
        // recorded as a symlink that arrives as a regular file (or the reverse) is a package
        // whose metadata and payload disagree, and installing it would hide that.
        let recorded_kind = recorded.path_type.as_str();
        // `paths.json` has one word for regular files and hardlinks both; only a symlink is
        // installed differently, so that is the only distinction the archive has to agree with.
        let archive_kind = if kind.is_symlink() {
            "softlink"
        } else {
            "hardlink"
        };
        if recorded_kind != "directory" && recorded_kind != archive_kind {
            return Err(Finding::error(
                "airlock.prefix.type",
                format!(
                    "`{path}` is recorded as a {recorded_kind} and arrives as a {archive_kind}"
                ),
            ));
        }
        let destination = prefix.join(&path);
        create_parent(&destination)?;
        if kind.is_symlink() {
            let target = item
                .link_name()
                .map_err(|error| streaming(error, archive))?
                .ok_or_else(|| {
                    Finding::error(
                        "airlock.prefix.path",
                        format!("`{path}` is a symlink with no target"),
                    )
                })?;
            // A symlink that escapes the prefix would make a package able to reach outside the
            // environment it was installed into.
            if target.is_absolute() || !is_relative(&target) {
                return Err(Finding::error(
                    "airlock.prefix.path",
                    format!(
                        "`{path}` links to `{}`, which is not inside the prefix",
                        target.display()
                    ),
                ));
            }
            std::os::unix::fs::symlink(&target, &destination).map_err(|error| {
                Finding::error(
                    "airlock.prefix.link",
                    format!("cannot create the symlink {path}: {error}"),
                )
            })?;
        } else if kind.is_hard_link() {
            let target = item
                .link_name()
                .map_err(|error| streaming(error, archive))?
                .ok_or_else(|| {
                    Finding::error(
                        "airlock.prefix.path",
                        format!("`{path}` is a hardlink with no target"),
                    )
                })?;
            let target = prefix.join(&target);
            // The archive is ordered so a hardlink follows the file it points at; if it does
            // not, the link would silently point at nothing.
            if !target.exists() {
                return Err(Finding::error(
                    "airlock.prefix.link",
                    format!(
                        "`{path}` hardlinks {} which this archive has not installed",
                        target.display()
                    ),
                ));
            }
            std::fs::hard_link(&target, &destination).map_err(|error| {
                Finding::error(
                    "airlock.prefix.link",
                    format!("cannot create the hardlink {path}: {error}"),
                )
            })?;
        } else if kind.is_file() {
            let mut bytes = Vec::new();
            item.read_to_end(&mut bytes)
                .map_err(|error| streaming(error, archive))?;
            let bytes = relocate(&bytes, recorded, prefix, &path)?;
            write_file(&destination, &bytes, item.header().mode().unwrap_or(0o644))?;
            verify(&path, &bytes, recorded)?;
            installed.bytes += bytes.len() as u64;
        } else {
            return Err(Finding::error(
                "airlock.prefix.entry",
                format!("`{path}` is an archive entry this build cannot install"),
            ));
        }
        seen.insert(recorded.path.as_str());
        installed.entries += 1;
    }

    // A name in paths.json with no entry in the archive is an environment that is missing a
    // file it was told it has; installers refuse this rather than leave the gap.
    let absent: Vec<&str> = expected.difference(&seen).copied().collect();
    if !absent.is_empty() {
        return Err(Finding::error(
            "airlock.prefix.missing",
            format!(
                "{} names {} in info/paths.json but does not carry {}",
                archive.display(),
                absent.join(", "),
                if absent.len() == 1 { "it" } else { "them" }
            ),
        ));
    }
    Ok(installed)
}

/// The one archive member whose name starts with `prefix`.
fn single(names: &[String], prefix: &str, archive: &Path) -> Result<String, Finding> {
    let matches: Vec<&String> = names
        .iter()
        .filter(|name| name.starts_with(prefix) && name.ends_with(".tar.zst"))
        .collect();
    match matches.as_slice() {
        [only] => Ok((*only).clone()),
        _ => Err(Finding::error(
            "airlock.prefix.archive",
            format!(
                "{} holds {} members named `{prefix}*.tar.zst`; a .conda holds exactly one",
                archive.display(),
                matches.len()
            ),
        )),
    }
}

/// One file's bytes out of an already-decompressed info tar.
fn read_member(info: &[u8], name: &str) -> Option<Vec<u8>> {
    let mut archive = tar::Archive::new(std::io::Cursor::new(info));
    for entry in archive.entries().ok()? {
        let mut entry = entry.ok()?;
        if entry.path().ok()?.to_string_lossy() == name {
            let mut bytes = Vec::new();
            entry.read_to_end(&mut bytes).ok()?;
            return Some(bytes);
        }
    }
    None
}

/// Rewrite the build-time prefix the file carries, when it carries one.
///
/// Nothing happens to the common case — a Rust binary or a licence file has no placeholder —
/// which is why this is a rewrite of *occurrences* rather than a fixed-size patch.
fn relocate(
    bytes: &[u8],
    entry: &PathEntry,
    prefix: &Path,
    path: &str,
) -> Result<Vec<u8>, Finding> {
    let Some(placeholder) = entry
        .prefix_placeholder
        .as_deref()
        .filter(|placeholder| !placeholder.is_empty())
    else {
        return Ok(bytes.to_vec());
    };
    let target = prefix.to_string_lossy();
    let binary = entry.file_mode.as_deref() == Some("binary");
    if binary && target.len() > placeholder.len() {
        return Err(Finding::error(
            "airlock.prefix.length",
            format!(
                "`{path}` was built for the prefix `{placeholder}` and the environment prefix \
                 `{target}` is longer, which a fixed-width binary patch cannot express"
            ),
        ));
    }
    let mut out = Vec::with_capacity(bytes.len());
    let mut rest = bytes;
    while let Some(index) = find(rest, placeholder.as_bytes()) {
        out.extend_from_slice(&rest[..index]);
        out.extend_from_slice(target.as_bytes());
        if binary {
            // A binary placeholder is fixed width: the reference instalment pads with NULs so
            // every offset in the file stays where the linker put it.
            out.extend(std::iter::repeat_n(0u8, placeholder.len() - target.len()));
        }
        rest = &rest[index + placeholder.len()..];
    }
    out.extend_from_slice(rest);
    Ok(out)
}

fn find(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() || haystack.len() < needle.len() {
        return None;
    }
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

/// Check one installed file against what `info/paths.json` recorded for it.
fn verify(path: &str, bytes: &[u8], entry: &PathEntry) -> Result<(), Finding> {
    if let Some(expected) = entry.size_in_bytes
        && expected != bytes.len() as u64
    {
        return Err(Finding::error(
            "airlock.prefix.size",
            format!(
                "`{path}` installed {actual} bytes, and info/paths.json records {expected}",
                actual = bytes.len()
            ),
        ));
    }
    if let Some(expected) = entry.sha256.as_deref() {
        let digest = sha256_hex(bytes);
        if !digest.eq_ignore_ascii_case(expected) {
            return Err(Finding::error(
                "airlock.prefix.digest",
                format!(
                    "`{path}` installed bytes hashing to {digest}, and info/paths.json records {expected}"
                ),
            ));
        }
    }
    Ok(())
}

/// Write a file with the mode the archive recorded, made readable regardless.
fn write_file(path: &Path, bytes: &[u8], mode: u32) -> Result<(), Finding> {
    use std::io::Write;
    use std::os::unix::fs::PermissionsExt;
    let mut file = std::fs::File::create(path).map_err(|error| {
        Finding::error(
            "airlock.prefix.write",
            format!("cannot create {}: {error}", path.display()),
        )
    })?;
    file.write_all(bytes).map_err(|error| {
        Finding::error(
            "airlock.prefix.write",
            format!("cannot write {}: {error}", path.display()),
        )
    })?;
    // `0o644` as the floor: an archive that recorded `0o600` would install a file its own user
    // cannot read after a restore run by another account, and the payload's owner already
    // decided what these bytes are.
    let mode = (mode & 0o777).max(0o644);
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode)).map_err(|error| {
        Finding::error(
            "airlock.prefix.write",
            format!("cannot set the mode of {}: {error}", path.display()),
        )
    })
}

fn create_directory(path: &Path) -> Result<(), Finding> {
    std::fs::create_dir_all(path).map_err(|error| {
        Finding::error(
            "airlock.prefix.write",
            format!("cannot create {}: {error}", path.display()),
        )
    })
}

fn create_parent(path: &Path) -> Result<(), Finding> {
    match path.parent() {
        Some(parent) => create_directory(parent),
        None => Ok(()),
    }
}

fn streaming(error: std::io::Error, archive: &Path) -> Finding {
    Finding::error(
        "airlock.prefix.archive",
        format!("cannot read the payload of {}: {error}", archive.display()),
    )
}

/// Why `path` is not a path this module will install, if it is not.
fn check_relative(path: &str) -> Result<(), &'static str> {
    if path.is_empty() {
        return Err("which is empty");
    }
    let candidate = PathBuf::from(path);
    if !is_relative(&candidate) {
        return Err("which is not a relative path inside the prefix");
    }
    Ok(())
}

fn is_relative(path: &Path) -> bool {
    path.components()
        .all(|component| matches!(component, Component::Normal(_) | Component::CurDir))
        && !path.as_os_str().is_empty()
}
