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
//! A `noarch` package is the third thing Conda does differently, and it is handled here too:
//! `noarch: generic` installs as recorded, while `noarch: python` records a `site-packages/` tree
//! and optional `python-scripts/` that belong under the interpreter the prefix already holds, and
//! declares the commands it ships. Those commands are generated at install time, against this
//! prefix's own interpreter, because the package cannot know which prefix it will land in.
//!
//! A prefix is more than one archive, so [`replay`] installs a whole set: it checks every
//! artifact against the digest the index recorded *before* it writes anything, then installs
//! them in the order `info/index.json` says they depend on each other. That order is not a
//! convenience — a `noarch: python` package installs into an interpreter the prefix has to
//! already hold — and it is not a solve either: a dependency with no artifact in the set is
//! refused rather than fetched, because a replay has no channel to fall back to.
//!
//! What this module does *not* do: read a lock. A lock has formats; [`replay`] takes what a
//! mirror index already reduced it to — a path and a digest per artifact — and `replay_mirror`
//! reads that index (`airlock.mirror`).

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

/// One archive a replay installs, as the index describing a lock records it.
///
/// Path and digest and nothing else: the archive's own `info/index.json` is the authority on what
/// the package is called, and asking a mirror index to repeat that would be a second place to be
/// wrong about it.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Pinned {
    /// Path of the archive, relative to the directory being replayed.
    pub path: String,
    /// Lowercase hex SHA-256 the archive's bytes must have.
    pub sha256: String,
    /// Size in bytes the archive must have, when the index records one.
    pub size: Option<u64>,
}

/// What a replay installed.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Replayed {
    /// Artifacts installed.
    pub artifacts: usize,
    /// Artifacts the index named that a Conda prefix cannot hold.
    pub skipped: usize,
    /// Files, symlinks and directories the artifacts created.
    pub entries: u64,
    /// Bytes written for regular files.
    pub bytes: u64,
    /// Package names, in the order they were installed.
    pub order: Vec<String>,
    /// What the replay had to say that did not stop it.
    pub notes: Vec<Finding>,
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

/// `info/link.json`, which is where a `noarch` package states its flavour and its commands.
#[derive(Clone, Debug, Deserialize)]
struct LinkFile {
    #[serde(default)]
    noarch: Option<LinkNoarch>,
}

/// The `noarch` block of `info/link.json`.
#[derive(Clone, Debug, Deserialize)]
struct LinkNoarch {
    /// `python` or `generic`.
    #[serde(rename = "type")]
    kind: String,
    /// `name = module:function`, one per command the package ships.
    #[serde(default)]
    entry_points: Vec<String>,
}

/// The part of `info/index.json` this module acts on.
#[derive(Clone, Debug, Deserialize)]
struct IndexFile {
    /// Package name, as the archive declares it.
    #[serde(default)]
    name: String,
    /// Package version, as the archive declares it.
    #[serde(default)]
    version: String,
    /// `noarch` flavour, when the package is one.
    #[serde(default)]
    noarch: Option<String>,
    /// Subdirectory the package belongs to.
    #[serde(default)]
    subdir: String,
    /// What this package needs installed beside it, as `name constraints build`.
    ///
    /// Only the name is read: a replay orders a set that is already solved, and the constraints
    /// were the solver's business.
    #[serde(default)]
    depends: Vec<String>,
}

impl IndexFile {
    /// The name of this package's dependency named first in `spec`, or `None` for an empty one.
    fn dependency_name(spec: &str) -> Option<&str> {
        spec.split_whitespace()
            .next()
            .filter(|name| !name.is_empty())
    }
}

/// Install one `.conda` archive into `prefix`.
///
/// `prefix` must already exist and must be the directory the environment will use; the caller
/// owns the transaction, this function owns the contents.
///
/// # Errors
///
/// Returns a finding when the archive cannot be read, is not a `.conda`, carries an entry that
/// escapes the prefix, disagrees with its own `paths.json`, or is a `noarch` package this module
/// cannot place — a `noarch: python` one in a prefix with no interpreter, with two, or with a
/// payload path outside its two roots, or a flavour other than `python` and `generic`.
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
    let payload_member = single(&names, "pkg-", archive)?;

    // The info section is small and is read once, in full, before anything is written: a
    // package whose metadata cannot be read must not have placed a single file.
    let (index, info) = info_section(&mut zip, archive)?;
    // Where this archive's recorded paths land. A native or `noarch: generic` package is
    // prefix-relative as recorded; a `noarch: python` one is a site-packages tree that has to be
    // put under the interpreter the prefix already holds, and whose commands have to be written.
    let (layout, entry_points) = layout(&index, &info, prefix, archive)?;

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
            create_directory(&layout.destination(&path)?)?;
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
        let destination = layout.destination(&path)?;
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
            let target = layout.link_target(&target.to_string_lossy())?;
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
            let target = prefix.join(layout.link_target(&target.to_string_lossy())?);
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
            let mode = layout.mode(&path, item.header().mode().unwrap_or(0o644));
            write_file(&destination, &bytes, mode)?;
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
    // A `noarch: python` package's commands are generated here rather than carried: they name
    // the interpreter this prefix actually holds, which the package itself could not know.
    let written = write_entry_points(prefix, &entry_points)?;
    installed.entries += written.0;
    installed.bytes += written.1;
    Ok(installed)
}

/// Install a set of archives into `prefix`, dependencies first.
///
/// Two passes, in this order and for different reasons. The first checks every artifact against
/// the digest and size the index recorded and reads its `info/index.json`, so a set that is
/// damaged, incomplete or ambiguous fails before a single file is written. The second installs
/// them in dependency order, because `noarch: python` packages install into an interpreter the
/// prefix has to hold already.
///
/// # Errors
///
/// Returns a finding when the set is empty, an artifact is absent, the wrong size or the wrong
/// digest, two artifacts are one package, a package depends on something the set does not hold,
/// or [`materialise`] refuses one of the archives.
pub fn replay(pinned: &[Pinned], root: &Path, prefix: &Path) -> Result<Replayed, Finding> {
    if pinned.is_empty() {
        return Err(Finding::error(
            "airlock.replay.empty",
            "there is nothing to replay: the index names no artifact",
        ));
    }

    let mut packages: Vec<Package> = Vec::with_capacity(pinned.len());
    for pin in pinned {
        let path = root.join(&pin.path);
        check_artifact(&path, pin)?;
        let file = std::fs::File::open(&path).map_err(|error| {
            Finding::error(
                "airlock.replay.missing",
                format!("cannot open {}: {error}", path.display()),
            )
        })?;
        let mut zip = zip::ZipArchive::new(file).map_err(|error| {
            Finding::error(
                "airlock.prefix.archive",
                format!(
                    "{} is not a readable .conda archive: {error}",
                    path.display()
                ),
            )
        })?;
        let (index, _) = info_section(&mut zip, &path)?;
        if index.name.is_empty() {
            return Err(Finding::error(
                "airlock.replay.identity",
                format!(
                    "{} carries no package name in info/index.json, so there is nothing to order \
                     it by",
                    path.display()
                ),
            ));
        }
        if let Some(existing) = packages.iter().find(|package| package.name == index.name) {
            return Err(Finding::error(
                "airlock.replay.duplicate",
                format!(
                    "{} and {} are both `{}` ({} and {}); a prefix holds one build of a package, \
                     and a set that names two is not one environment",
                    existing.archive.display(),
                    path.display(),
                    index.name,
                    existing.version,
                    index.version
                ),
            ));
        }
        packages.push(Package {
            name: index.name,
            version: index.version,
            depends: index.depends,
            archive: path,
        });
    }

    let (positions, notes) = order(&packages)?;
    std::fs::create_dir_all(prefix).map_err(|error| {
        Finding::error(
            "airlock.replay.prefix",
            format!("cannot create {}: {error}", prefix.display()),
        )
    })?;
    let mut replayed = Replayed {
        artifacts: packages.len(),
        notes,
        ..Replayed::default()
    };
    for position in positions {
        let package = &packages[position];
        let installed = materialise(&package.archive, prefix)?;
        replayed.entries += installed.entries;
        replayed.bytes += installed.bytes;
        replayed.order.push(package.name.clone());
    }
    Ok(replayed)
}

/// Replay the artifacts a mirror index names.
///
/// The mirror index is where a lock's format stops mattering: `airlock.mirror` reduced every
/// pinned artifact to a path and a digest, and this reads that reduction. Artifacts whose `kind`
/// is not `conda` are counted, named in a note, and left out — a Conda prefix holds Conda
/// packages, and installing a wheel is not something this build does.
///
/// # Errors
///
/// Returns a finding when the index is absent or unreadable, declares a schema this build does
/// not read, names no Conda artifact, or when [`replay`] refuses the set.
pub fn replay_mirror(root: &Path, prefix: &Path) -> Result<Replayed, Finding> {
    let index_path = root.join(MIRROR_INDEX);
    let text = std::fs::read_to_string(&index_path).map_err(|error| {
        Finding::error(
            "airlock.replay.index",
            format!(
                "cannot read {}: {error}; a replay installs what a mirror published, and this \
                 is where it says so",
                index_path.display()
            ),
        )
    })?;
    let index: MirrorIndex = serde_json::from_str(&text).map_err(|error| {
        Finding::error(
            "airlock.replay.index",
            format!(
                "{} is not the index this build reads: {error}",
                index_path.display()
            ),
        )
    })?;
    if index.schema != MIRROR_SCHEMA {
        return Err(Finding::error(
            "airlock.replay.schema",
            format!(
                "{} declares mirror index schema {}, and this build reads {MIRROR_SCHEMA}",
                index_path.display(),
                index.schema
            ),
        ));
    }
    let mut pinned: Vec<Pinned> = Vec::with_capacity(index.artifacts.len());
    let mut skipped: Vec<String> = Vec::new();
    for artifact in &index.artifacts {
        if artifact.kind == "conda" {
            pinned.push(Pinned {
                path: artifact.path.clone(),
                sha256: artifact.sha256.clone(),
                size: artifact.size,
            });
        } else {
            skipped.push(format!(
                "{} {} ({})",
                artifact.name, artifact.version, artifact.kind
            ));
        }
    }
    if pinned.is_empty() {
        return Err(Finding::error(
            "airlock.replay.empty",
            format!(
                "{} names {} artifact(s) and none of them is a Conda package, so there is \
                 nothing a prefix could hold",
                index_path.display(),
                index.artifacts.len()
            ),
        ));
    }
    let mut replayed = replay(&pinned, root, prefix)?;
    replayed.skipped = skipped.len();
    if !skipped.is_empty() {
        replayed.notes.push(Finding::warning(
            "airlock.replay.skipped",
            format!(
                "{} artifact(s) a Conda prefix cannot hold were left out: {}",
                skipped.len(),
                skipped.join(", ")
            ),
        ));
    }
    Ok(replayed)
}

/// The index a mirror publishes, as much of it as a replay reads.
#[derive(Clone, Debug, Deserialize)]
struct MirrorIndex {
    /// Shape version of the index.
    schema: u32,
    /// Artifacts the mirror holds.
    artifacts: Vec<MirrorArtifact>,
}

/// One artifact of that index.
#[derive(Clone, Debug, Deserialize)]
struct MirrorArtifact {
    /// Package name.
    name: String,
    /// Package version.
    version: String,
    /// `conda`, `wheel` or `crate`.
    kind: String,
    /// Path inside the mirror.
    path: String,
    /// Lowercase hex SHA-256 of the artifact.
    sha256: String,
    /// Size in bytes, when the index records one.
    #[serde(default)]
    size: Option<u64>,
}

/// The index's own file name, as `airlock.mirror` writes it.
const MIRROR_INDEX: &str = "mirror.json";

/// Shape version of that index this build reads.
const MIRROR_SCHEMA: u32 = 1;

/// One package a replay is about to install.
struct Package {
    /// Name as the archive declares it, which is what dependencies name.
    name: String,
    /// Version as the archive declares it, for the message about a duplicate.
    version: String,
    /// What it depends on, as recorded.
    depends: Vec<String>,
    /// Where the archive is.
    archive: PathBuf,
}

/// Check an artifact against what the index recorded, before anything is installed.
fn check_artifact(path: &Path, pin: &Pinned) -> Result<(), Finding> {
    let missing = || {
        Finding::error(
            "airlock.replay.missing",
            format!(
                "{} is not in the mirror; a replay installs only what the index pins, and it has \
                 no channel to fall back to",
                path.display()
            ),
        )
    };
    let size = std::fs::metadata(path).map_err(|_| missing())?.len();
    if let Some(expected) = pin.size
        && size != expected
    {
        return Err(Finding::error(
            "airlock.replay.size",
            format!(
                "{} is {size} bytes and the index recorded {expected}",
                path.display()
            ),
        ));
    }
    let (digest, _) = crate::hash::hash_file(path).map_err(|error| {
        Finding::error(
            "airlock.replay.digest",
            format!("cannot read {}: {error}", path.display()),
        )
    })?;
    if !digest.eq_ignore_ascii_case(&pin.sha256) {
        return Err(Finding::error(
            "airlock.replay.digest",
            format!(
                "{} hashes to {digest} and the index recorded {}",
                path.display(),
                pin.sha256
            ),
        ));
    }
    Ok(())
}

/// Which packages install before which: dependencies first, ties and cycles by name.
///
/// Deterministic on purpose. Two replays of one set have to install the same files in the same
/// order, or a run that failed once may succeed the second time for no reason anyone can name.
///
/// # Errors
///
/// Returns a finding naming the package and the dependency when the set depends on something it
/// does not hold. A virtual package (`__glibc`, `__unix`) is provided by the host, so it is not
/// a dependency the set has to hold.
fn order(packages: &[Package]) -> Result<(Vec<usize>, Vec<Finding>), Finding> {
    let positions: std::collections::BTreeMap<&str, usize> = packages
        .iter()
        .enumerate()
        .map(|(position, package)| (package.name.as_str(), position))
        .collect();
    let mut dependents: Vec<Vec<usize>> = vec![Vec::new(); packages.len()];
    let mut waiting: Vec<usize> = vec![0; packages.len()];
    for (position, package) in packages.iter().enumerate() {
        for spec in &package.depends {
            let Some(name) = IndexFile::dependency_name(spec) else {
                continue;
            };
            if name.starts_with("__") {
                continue;
            }
            let Some(&dependency) = positions.get(name) else {
                return Err(Finding::error(
                    "airlock.replay.dependency",
                    format!(
                        "{} {} depends on `{name}`, which this set does not hold; a replay \
                         installs what the lock pinned and never fetches the rest",
                        package.name, package.version
                    ),
                ));
            };
            dependents[dependency].push(position);
            waiting[position] += 1;
        }
    }

    let mut ready: BTreeSet<(&str, usize)> = packages
        .iter()
        .enumerate()
        .filter(|(position, _)| waiting[*position] == 0)
        .map(|(position, package)| (package.name.as_str(), position))
        .collect();
    let mut placed = vec![false; packages.len()];
    let mut positions_in_order: Vec<usize> = Vec::with_capacity(packages.len());
    let mut notes: Vec<Finding> = Vec::new();
    while positions_in_order.len() < packages.len() {
        let next = match ready.iter().next().copied() {
            Some((name, position)) => {
                ready.remove(&(name, position));
                position
            }
            None => {
                // Everything left waits on something left, which is a cycle. Conda permits
                // those, so the set is installable; only its order is undetermined, and this
                // picks one and says so.
                let position = (0..packages.len())
                    .filter(|position| !placed[*position])
                    .min_by(|left, right| packages[*left].name.cmp(&packages[*right].name))
                    .expect("something is left while the order is incomplete");
                notes.push(Finding::warning(
                    "airlock.replay.order",
                    format!(
                        "the packages left to install form a cycle, and one of them has to come \
                         first: `{}` was installed there rather than after everything it depends \
                         on",
                        packages[position].name
                    ),
                ));
                position
            }
        };
        placed[next] = true;
        positions_in_order.push(next);
        for dependent in &dependents[next] {
            if placed[*dependent] {
                continue;
            }
            waiting[*dependent] -= 1;
            if waiting[*dependent] == 0 {
                ready.insert((packages[*dependent].name.as_str(), *dependent));
            }
        }
    }
    Ok((positions_in_order, notes))
}

/// Where one archive's recorded paths land in the prefix.
///
/// Two cases, because Conda has two. A native or `noarch: generic` package records paths that are
/// already prefix-relative. A `noarch: python` package records a `site-packages/` tree and,
/// optionally, `python-scripts/` — the first belongs under the interpreter this prefix holds, the
/// second in `bin` — and that is the whole relocation: the payload's bytes are not rewritten.
struct Layout {
    /// The prefix being installed into.
    prefix: PathBuf,
    /// `<prefix>/lib/python3.X/site-packages`, when this is a `noarch: python` package.
    site_packages: Option<PathBuf>,
}

impl Layout {
    /// A layout that installs recorded paths where they are recorded.
    fn native(prefix: &Path) -> Self {
        Self {
            prefix: prefix.to_path_buf(),
            site_packages: None,
        }
    }

    /// Where `recorded` is installed.
    fn destination(&self, recorded: &str) -> Result<PathBuf, Finding> {
        let Some(site_packages) = self.site_packages.as_ref() else {
            return Ok(self.prefix.join(recorded));
        };
        // Conda's rule for this flavour, and the reason the refusal is a refusal: a payload path
        // is either a module tree or a script, and a package carrying anything else would land
        // somewhere no import path and no `bin` looks at.
        if let Some(rest) = recorded.strip_prefix("site-packages/") {
            return Ok(site_packages.join(rest));
        }
        if let Some(rest) = recorded.strip_prefix("python-scripts/") {
            return Ok(self.prefix.join("bin").join(rest));
        }
        Err(Finding::error(
            "airlock.prefix.noarch.path",
            format!(
                "a noarch: python package records `{recorded}`, which is under neither \
                 `site-packages/` nor `python-scripts/`"
            ),
        ))
    }

    /// Where a link's recorded target points, expressed inside the prefix.
    ///
    /// A native package's links are stored as recorded, which keeps them relative and therefore
    /// keeps a prefix copyable. A `noarch: python` package's link is relocated the same way its
    /// files are, and reported relative to the prefix for the same reason.
    fn link_target(&self, recorded: &str) -> Result<PathBuf, Finding> {
        if self.site_packages.is_none() {
            return Ok(PathBuf::from(recorded));
        }
        let destination = self.destination(recorded)?;
        Ok(destination
            .strip_prefix(&self.prefix)
            .map_or(destination.clone(), Path::to_path_buf))
    }

    /// The mode an installed file gets, which is the archive's except where the layout decides.
    fn mode(&self, recorded: &str, mode: u32) -> u32 {
        // A script under `python-scripts/` is a command; an archive that recorded it `0644`
        // would install something nobody can run.
        if self.site_packages.is_some() && recorded.starts_with("python-scripts/") {
            return mode | 0o755;
        }
        mode
    }
}

/// How `archive` is laid out, and the commands it declares.
fn layout(
    index: &IndexFile,
    info: &[u8],
    prefix: &Path,
    archive: &Path,
) -> Result<(Layout, Vec<EntryPoint>), Finding> {
    let Some(flavour) = index
        .noarch
        .as_deref()
        .filter(|flavour| !flavour.is_empty())
    else {
        if index.subdir == "noarch" {
            return Err(Finding::error(
                "airlock.prefix.noarch.type",
                format!(
                    "{} is in the noarch subdir and does not say which flavour it is; this build \
                     installs `python` and `generic`",
                    archive.display()
                ),
            ));
        }
        return Ok((Layout::native(prefix), Vec::new()));
    };
    match flavour {
        "generic" => Ok((Layout::native(prefix), Vec::new())),
        "python" => {
            let link = read_member(info, "info/link.json").ok_or_else(|| {
                Finding::error(
                    "airlock.prefix.noarch.link",
                    format!(
                        "{} is a noarch: python package and carries no info/link.json, which is \
                         where its flavour and its commands are recorded",
                        archive.display()
                    ),
                )
            })?;
            let link: LinkFile = serde_json::from_slice(&link).map_err(|error| {
                Finding::error(
                    "airlock.prefix.noarch.link",
                    format!(
                        "info/link.json in {} is not the record this build reads: {error}",
                        archive.display()
                    ),
                )
            })?;
            let Some(noarch) = link.noarch else {
                return Err(Finding::error(
                    "airlock.prefix.noarch.link",
                    format!(
                        "info/link.json in {} does not say which noarch flavour the package is",
                        archive.display()
                    ),
                ));
            };
            if noarch.kind != "python" {
                return Err(Finding::error(
                    "airlock.prefix.noarch.type",
                    format!(
                        "{} declares noarch \"{}\", and this build installs `python` and `generic`",
                        archive.display(),
                        noarch.kind
                    ),
                ));
            }
            let site_packages = interpreter(prefix, archive)?;
            let entry_points = noarch
                .entry_points
                .iter()
                .map(|spec| EntryPoint::parse(spec, archive))
                .collect::<Result<Vec<_>, _>>()?;
            Ok((
                Layout {
                    prefix: prefix.to_path_buf(),
                    site_packages: Some(site_packages),
                },
                entry_points,
            ))
        }
        other => Err(Finding::error(
            "airlock.prefix.noarch.type",
            format!(
                "{} declares noarch \"{other}\", and this build installs `python` and `generic`",
                archive.display()
            ),
        )),
    }
}

/// The `site-packages` directory of the one interpreter this prefix holds.
///
/// A `noarch: python` package is installed *for* an interpreter, so the prefix has to have one
/// already: the package that carries the interpreter comes first in a lock's order. Two is a
/// question no archive can answer, so it is refused rather than guessed.
fn interpreter(prefix: &Path, archive: &Path) -> Result<PathBuf, Finding> {
    let lib = prefix.join("lib");
    let mut found: Vec<String> = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&lib) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
            if name.starts_with("python3.") && entry.path().is_dir() {
                found.push(name);
            }
        }
    }
    found.sort();
    match found.as_slice() {
        [only] => {
            let site_packages = lib.join(only).join("site-packages");
            create_directory(&site_packages)?;
            Ok(site_packages)
        }
        [] => Err(Finding::error(
            "airlock.prefix.noarch.interpreter",
            format!(
                "{} is a noarch: python package and this prefix holds no lib/python3.* to \
                 install it for; the package that carries the interpreter has to be installed \
                 first",
                archive.display()
            ),
        )),
        _ => Err(Finding::error(
            "airlock.prefix.noarch.ambiguous",
            format!(
                "{} is a noarch: python package and this prefix holds {} interpreters ({}); \
                 which one it is installed for is not something an archive can answer",
                archive.display(),
                found.len(),
                found.join(", ")
            ),
        )),
    }
}

/// One command a `noarch: python` package declares.
#[derive(Clone, Debug, Eq, PartialEq)]
struct EntryPoint {
    /// The command's name, which is the file name in `bin`.
    name: String,
    /// The module the function lives in.
    module: String,
    /// The function the command calls.
    function: String,
}

impl EntryPoint {
    /// The entry point `spec` names, as `name = module:function`.
    ///
    /// Everything this accepts is written into a script that the environment then executes, so
    /// the accepted characters are the ones an identifier and a command name are made of. A spec
    /// that would need escaping is refused rather than escaped: a second escape rule is a second
    /// place to get it wrong.
    fn parse(spec: &str, archive: &Path) -> Result<Self, Finding> {
        let unusable = || {
            Finding::error(
                "airlock.prefix.noarch.entry",
                format!(
                    "{} declares the entry point `{spec}`, and one is written \
                     `name = module:function` from letters, digits, dots, dashes and underscores",
                    archive.display()
                ),
            )
        };
        let (name, target) = spec.split_once('=').ok_or_else(unusable)?;
        let (module, function) = target.split_once(':').ok_or_else(unusable)?;
        let (name, module, function) = (name.trim(), module.trim(), function.trim());
        let simple = |value: &str, allowed: &dyn Fn(char) -> bool| {
            !value.is_empty() && value.chars().all(allowed)
        };
        let identifier = |character: char| character.is_ascii_alphanumeric() || character == '_';
        let command = |character: char| identifier(character) || matches!(character, '-' | '.');
        let path = |character: char| identifier(character) || character == '.';
        if !simple(name, &command) || !simple(module, &path) || !simple(function, &identifier) {
            return Err(unusable());
        }
        Ok(Self {
            name: name.to_owned(),
            module: module.to_owned(),
            function: function.to_owned(),
        })
    }
}

/// Write one script per declared command, and report how many and how many bytes.
fn write_entry_points(prefix: &Path, entry_points: &[EntryPoint]) -> Result<(u64, u64), Finding> {
    if entry_points.is_empty() {
        return Ok((0, 0));
    }
    let bin = prefix.join("bin");
    create_directory(&bin)?;
    let (mut entries, mut bytes) = (0_u64, 0_u64);
    for entry in entry_points {
        let path = bin.join(&entry.name);
        let script = entry.script(prefix);
        write_file(&path, script.as_bytes(), 0o755)?;
        entries += 1;
        bytes += script.len() as u64;
    }
    Ok((entries, bytes))
}

impl EntryPoint {
    /// The script that runs this command out of the prefix it was installed into.
    ///
    /// The same shape Conda generates: the prefix's own interpreter in the shebang, `argv[0]`
    /// normalised so a `.exe` or `-script.pyw` suffix cannot confuse the program, and the
    /// declared function's return value as the exit status.
    fn script(&self, prefix: &Path) -> String {
        format!(
            "#!{}/bin/python\n# -*- coding: utf-8 -*-\nimport re\nimport sys\n\nfrom {} import {}\n\n\
             if __name__ == \"__main__\":\n    sys.argv[0] = re.sub(r\"(-script\\.pyw|\\.exe)?$\", \"\", sys.argv[0])\n    sys.exit({}())\n",
            prefix.display(),
            self.module,
            self.function,
            self.function
        )
    }
}

/// An archive's metadata: its `info-*.tar.zst`, decompressed, and the index it carries.
///
/// Both a replay and an install need it before anything is written, and both need the same
/// refusals: a member list that names two info sections, or an index this build cannot read.
fn info_section(
    zip: &mut zip::ZipArchive<std::fs::File>,
    archive: &Path,
) -> Result<(IndexFile, Vec<u8>), Finding> {
    let mut names: Vec<String> = Vec::with_capacity(zip.len());
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
    Ok((index, info))
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
