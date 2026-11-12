//! Activating a verified payload, transactionally.
//!
//! The state root holds immutable environments and two symlinks:
//!
//! ```text
//! <state>/envs/<version>-<manifest prefix>/   one materialised payload, never edited
//! <state>/current -> envs/<...>               the environment in use
//! <state>/previous -> envs/<...>              what `current` pointed at before the last change
//! ```
//!
//! A restore builds the new environment under `envs/.staging-<pid>` — the same filesystem, so
//! the final `rename` is atomic — and only then repoints `current`. Two rules follow, and both
//! are tested:
//!
//! - **Nothing is written where `current` points.** The environment is populated somewhere
//!   else and moved into place in one step, so a failure mid-copy cannot leave a half-written
//!   environment that `current` already names.
//! - **A failure changes nothing.** The staging directory is removed on the way out, `current`
//!   is untouched, and the previous environment remains complete and usable.
//!
//! Linux-only, deliberately and in line with `decision-132`: atomic replacement of a symlink
//! is `rename(2)`, and the Windows equivalent needs privileges this tool does not ask for.

use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use crate::diagnostics::Finding;
use crate::hash::Hasher;
use crate::payload::{Artifact, Report};
use crate::prefix;

/// The receipt a materialised environment carries, so a later `doctor` can describe it without
/// the payload it came from.
pub const RECEIPT_FILE: &str = "receipt";

/// The directory inside an environment that holds the installed files.
///
/// A prefix is a directory, and the environment holding one is the record of which payload
/// produced it: the archives stay beside it as the bytes that were verified, and `prefix` is what
/// a caller puts on `PATH`.
pub const PREFIX_DIR: &str = "prefix";

/// Receipt format version, so a future change can refuse an old receipt instead of misreading
/// it.
pub const RECEIPT_SCHEMA: &str = "1";

/// Where state lives when the caller does not say.
#[must_use]
pub fn default_root(working_directory: &Path, environment: Option<&str>) -> PathBuf {
    match environment {
        Some(configured) if !configured.trim().is_empty() => PathBuf::from(configured),
        _ => working_directory.join(".universe-airlock").join("state"),
    }
}

/// The layout of a state root.
#[derive(Clone, Debug)]
pub struct Layout {
    root: PathBuf,
}

/// What a successful activation did.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Outcome {
    /// The environment that is now active.
    pub env: PathBuf,
    /// Whether the environment already existed, so nothing was copied.
    pub reused: bool,
    /// What `current` pointed at before this run, when it pointed anywhere.
    pub previous: Option<PathBuf>,
    /// The installed prefix, which is what a caller puts on `PATH`.
    pub prefix: PathBuf,
    /// Files and directories the prefix holds.
    pub entries: u64,
    /// Bytes those files occupy.
    pub bytes: u64,
}

/// A materialised environment's own description of itself.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Receipt {
    /// Version the payload published.
    pub version: String,
    /// Commit the payload was built from.
    pub commit: String,
    /// The payload's identity.
    pub manifest_sha256: String,
    /// What the payload's authenticity amounted to.
    pub signature: String,
    /// The artifacts this environment holds.
    pub artifacts: Vec<Artifact>,
}

impl Receipt {
    /// Render the receipt's bytes.
    #[must_use]
    pub fn render(&self) -> String {
        let mut text = String::new();
        text.push_str(&format!("schema={RECEIPT_SCHEMA}\n"));
        text.push_str(&format!("version={}\n", self.version));
        text.push_str(&format!("commit={}\n", self.commit));
        text.push_str(&format!("manifest_sha256={}\n", self.manifest_sha256));
        text.push_str(&format!("signature={}\n", self.signature));
        for artifact in &self.artifacts {
            text.push_str(&format!(
                "artifact={} {} {}\n",
                artifact.filename, artifact.sha256, artifact.size
            ));
        }
        text
    }

    /// Read a receipt back.
    ///
    /// The format is line-oriented `key=value` rather than JSON because the bootstrap carries
    /// no JSON *writer*, and this record is written and read by this binary alone — it is not
    /// a published artifact. An unknown key is ignored; an unknown schema is not.
    ///
    /// # Errors
    ///
    /// Returns a finding when the text is not a receipt this build understands.
    pub fn parse(text: &str) -> Result<Self, Finding> {
        let mut schema: Option<&str> = None;
        let mut version: Option<&str> = None;
        let mut commit: Option<&str> = None;
        let mut manifest_sha256: Option<&str> = None;
        let mut signature: Option<&str> = None;
        let mut artifacts: Vec<Artifact> = Vec::new();

        for line in text.lines() {
            let Some((key, value)) = line.split_once('=') else {
                continue;
            };
            match key {
                "schema" => schema = Some(value),
                "version" => version = Some(value),
                "commit" => commit = Some(value),
                "manifest_sha256" => manifest_sha256 = Some(value),
                "signature" => signature = Some(value),
                "artifact" => {
                    let mut parts = value.split(' ');
                    let (Some(filename), Some(sha256), Some(size)) =
                        (parts.next(), parts.next(), parts.next())
                    else {
                        return Err(Finding::error(
                            "airlock.state.receipt",
                            format!("the receipt's artifact line is malformed: `{value}`"),
                        ));
                    };
                    let Ok(size) = size.parse::<u64>() else {
                        return Err(Finding::error(
                            "airlock.state.receipt",
                            format!("the receipt's artifact size is not a number: `{size}`"),
                        ));
                    };
                    artifacts.push(Artifact {
                        filename: filename.to_owned(),
                        sha256: sha256.to_owned(),
                        size,
                    });
                }
                _ => {}
            }
        }

        let missing = |field: &str| {
            Finding::error(
                "airlock.state.receipt",
                format!("the receipt names no `{field}`"),
            )
        };
        if schema != Some(RECEIPT_SCHEMA) {
            return Err(Finding::error(
                "airlock.state.receipt",
                format!(
                    "the receipt is schema {} and this bootstrap writes schema {RECEIPT_SCHEMA}",
                    schema.unwrap_or("<absent>")
                ),
            ));
        }
        Ok(Self {
            version: version.ok_or_else(|| missing("version"))?.to_owned(),
            commit: commit.ok_or_else(|| missing("commit"))?.to_owned(),
            manifest_sha256: manifest_sha256
                .ok_or_else(|| missing("manifest_sha256"))?
                .to_owned(),
            signature: signature.unwrap_or("unknown").to_owned(),
            artifacts,
        })
    }
}

impl Layout {
    /// A layout rooted at `root`.
    #[must_use]
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    /// The state root itself.
    #[must_use]
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Where materialised environments live.
    #[must_use]
    pub fn envs(&self) -> PathBuf {
        self.root.join("envs")
    }

    /// The symlink naming the active environment.
    #[must_use]
    pub fn current(&self) -> PathBuf {
        self.root.join("current")
    }

    /// The symlink naming what was active before.
    #[must_use]
    pub fn previous(&self) -> PathBuf {
        self.root.join("previous")
    }

    /// The environment `current` points at, resolved against the state root.
    ///
    /// The link is stored relative (`envs/<name>`) so the root can be moved or copied without
    /// every link inside it breaking.
    #[must_use]
    pub fn current_env(&self) -> Option<PathBuf> {
        resolve(&self.current(), &self.root)
    }

    /// The environment `previous` points at.
    #[must_use]
    pub fn previous_env(&self) -> Option<PathBuf> {
        resolve(&self.previous(), &self.root)
    }
}

fn resolve(link: &Path, root: &Path) -> Option<PathBuf> {
    let target = std::fs::read_link(link).ok()?;
    Some(if target.is_absolute() {
        target
    } else {
        root.join(target)
    })
}

/// Materialise `report`'s payload under `layout` and make it current.
///
/// `payload_root` is read again as the bytes are copied, and every copy is hashed as it is
/// written: the artifact verified a moment ago is the artifact that lands, rather than a file
/// that was replaced in between.
///
/// # Errors
///
/// Returns a finding and leaves `current` untouched, with no staging directory behind.
pub fn activate(layout: &Layout, payload_root: &Path, report: &Report) -> Result<Outcome, Finding> {
    let artifacts = match report.artifacts() {
        Ok(artifacts) => artifacts,
        Err(mut errors) => {
            return Err(if errors.is_empty() {
                Finding::error(
                    "airlock.payload",
                    "the manifest names no artifact a payload directory could hold",
                )
            } else {
                // The caller has already reported the whole set once; activation repeats the
                // first because it runs immediately after and must not assume a clean report.
                errors.remove(0)
            });
        }
    };

    let name = report.identity();
    let target = layout.envs().join(&name);
    let envs = layout.envs();
    std::fs::create_dir_all(&envs).map_err(|error| {
        Finding::error(
            "airlock.state.root",
            format!("cannot create {}: {error}", envs.display()),
        )
    })?;

    let mut reused = false;
    let mut installed_entries: u64 = 0;
    let mut installed_bytes: u64 = 0;
    if target.join(RECEIPT_FILE).exists() {
        // The same payload was restored before. Its environment is immutable, so the run is a
        // pointer move rather than a second copy.
        reused = true;
    } else {
        let staging = Staging::create(&envs)?;
        for artifact in &artifacts {
            copy_verified(
                &payload_root.join(&artifact.filename),
                &staging.path.join(&artifact.filename),
                artifact,
            )?;
        }
        // The archives are kept as the verified bytes; the prefix is what they install. Both
        // live in the staged directory, so neither can appear where `current` already points.
        let prefix_dir = staging.path.join(PREFIX_DIR);
        std::fs::create_dir_all(&prefix_dir).map_err(|error| {
            Finding::error(
                "airlock.state.activate",
                format!("cannot create {}: {error}", prefix_dir.display()),
            )
        })?;
        for artifact in &artifacts {
            let installed =
                prefix::materialise(&staging.path.join(&artifact.filename), &prefix_dir)?;
            installed_entries += installed.entries;
            installed_bytes += installed.bytes;
        }
        let receipt = Receipt {
            version: report.manifest.version.clone(),
            commit: report.manifest.commit.clone(),
            manifest_sha256: report.manifest_digest.clone(),
            signature: report.verdict.as_str().to_owned(),
            artifacts,
        };
        write_receipt(&staging.path, &receipt)?;
        staging.commit(&target)?;
    }

    let prefix = target.join(PREFIX_DIR);
    if reused {
        // An environment restored earlier is immutable, so its own prefix answers for it rather
        // than this run reporting zero files for a directory full of them.
        (installed_entries, installed_bytes) = measure(&prefix);
    }

    let previous = layout.current_env();
    point_at(&layout.current(), &layout.root, &name)?;
    if let Some(previous) = previous.as_ref()
        && Some(previous.clone()) != Some(target.clone())
    {
        let relative = previous.strip_prefix(&layout.root).unwrap_or(previous);
        write_link(&layout.previous(), relative).map_err(|error| {
            Finding::error(
                "airlock.state.activate",
                format!("cannot record the previous environment: {error}"),
            )
        })?;
    }

    Ok(Outcome {
        env: target,
        reused,
        previous,
        prefix,
        entries: installed_entries,
        bytes: installed_bytes,
    })
}

/// Count what a prefix holds, for a reuse that installed nothing this run.
fn measure(prefix: &Path) -> (u64, u64) {
    let mut entries: u64 = 0;
    let mut bytes: u64 = 0;
    let mut queue = vec![prefix.to_path_buf()];
    while let Some(directory) = queue.pop() {
        let Ok(read) = std::fs::read_dir(&directory) else {
            continue;
        };
        for entry in read.flatten() {
            let Ok(metadata) = std::fs::symlink_metadata(entry.path()) else {
                continue;
            };
            entries += 1;
            if metadata.is_dir() {
                queue.push(entry.path());
            } else if metadata.is_file() {
                bytes += metadata.len();
            }
        }
    }
    (entries, bytes)
}

/// A staging directory that deletes itself unless it was promoted.
struct Staging {
    path: PathBuf,
    promoted: bool,
}

impl Staging {
    fn create(envs: &Path) -> Result<Self, Finding> {
        let path = envs.join(format!(".staging-{}", std::process::id()));
        // A previous run that was killed leaves its staging behind; it is ours to replace.
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).map_err(|error| {
            Finding::error(
                "airlock.state.staging",
                format!("cannot create {}: {error}", path.display()),
            )
        })?;
        Ok(Self {
            path,
            promoted: false,
        })
    }

    fn commit(mut self, target: &Path) -> Result<(), Finding> {
        if target.exists() {
            // Another restore of the same payload won the race; both wrote identical bytes, so
            // the existing environment is the one to use.
            return Ok(());
        }
        std::fs::rename(&self.path, target).map_err(|error| {
            Finding::error(
                "airlock.state.staging",
                format!("cannot move the staged environment into place: {error}"),
            )
        })?;
        self.promoted = true;
        Ok(())
    }
}

impl Drop for Staging {
    fn drop(&mut self) {
        if !self.promoted {
            let _ = std::fs::remove_dir_all(&self.path);
        }
    }
}

/// Copy one artifact, hashing what is written rather than what was read.
fn copy_verified(source: &Path, destination: &Path, artifact: &Artifact) -> Result<(), Finding> {
    let mut input = std::fs::File::open(source).map_err(|error| {
        Finding::error(
            "airlock.payload.missing",
            format!("cannot open {}: {error}", source.display()),
        )
    })?;
    let mut output = std::fs::File::create(destination).map_err(|error| {
        Finding::error(
            "airlock.state.activate",
            format!("cannot create {}: {error}", destination.display()),
        )
    })?;

    let mut hasher = Hasher::new();
    let mut buffer = vec![0u8; 64 * 1024];
    let mut written: u64 = 0;
    loop {
        let read = input.read(&mut buffer).map_err(|error| {
            Finding::error(
                "airlock.payload.unreadable",
                format!("cannot read {}: {error}", source.display()),
            )
        })?;
        if read == 0 {
            break;
        }
        output.write_all(&buffer[..read]).map_err(|error| {
            Finding::error(
                "airlock.state.activate",
                format!("cannot write {}: {error}", destination.display()),
            )
        })?;
        hasher.update(&buffer[..read]);
        written += read as u64;
    }
    output.flush().map_err(|error| {
        Finding::error(
            "airlock.state.activate",
            format!("cannot flush {}: {error}", destination.display()),
        )
    })?;

    let digest = hasher.finish();
    if written != artifact.size || digest != artifact.sha256 {
        return Err(Finding::error(
            "airlock.payload.changed",
            format!(
                "{} changed while it was being restored: copied {written} bytes hashing to \
                 {digest}, expected {} bytes hashing to {}",
                artifact.filename, artifact.size, artifact.sha256
            ),
        ));
    }
    Ok(())
}

fn write_receipt(environment: &Path, receipt: &Receipt) -> Result<(), Finding> {
    let path = environment.join(RECEIPT_FILE);
    std::fs::write(&path, receipt.render()).map_err(|error| {
        Finding::error(
            "airlock.state.activate",
            format!("cannot write {}: {error}", path.display()),
        )
    })
}

/// Replace `link` with a symlink to `target`, in one step.
fn write_link(link: &Path, target: &Path) -> std::io::Result<()> {
    let temporary = link.with_extension("replacing");
    let _ = std::fs::remove_file(&temporary);
    make_symlink(target, &temporary)?;
    std::fs::rename(&temporary, link)
}

fn point_at(link: &Path, root: &Path, name: &str) -> Result<(), Finding> {
    let target = Path::new("envs").join(name);
    // The link is only ever stored relative to the root, so this cannot escape it.
    debug_assert!(!target.is_absolute(), "{} {}", root.display(), name);
    write_link(link, &target).map_err(|error| {
        Finding::error(
            "airlock.state.activate",
            format!(
                "cannot point {} at {}: {error}",
                link.display(),
                target.display()
            ),
        )
    })
}

#[cfg(unix)]
fn make_symlink(target: &Path, link: &Path) -> std::io::Result<()> {
    std::os::unix::fs::symlink(target, link)
}

#[cfg(not(unix))]
fn make_symlink(_target: &Path, _link: &Path) -> std::io::Result<()> {
    Err(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        "activating an environment needs symlinks, which this build does not support",
    ))
}

/// What `doctor` reports about a state root.
#[derive(Clone, Debug)]
pub struct Status {
    /// Whether the state root exists at all.
    pub root_exists: bool,
    /// The environment `current` names.
    pub current: Option<PathBuf>,
    /// Whether that environment still exists.
    pub current_exists: bool,
    /// Its receipt, when it has a readable one.
    pub receipt: Option<Receipt>,
    /// Anything inconsistent, as findings that explain themselves.
    pub problems: Vec<Finding>,
}

/// Inspect a state root without changing it.
#[must_use]
pub fn inspect(layout: &Layout) -> Status {
    let root_exists = layout.root().is_dir();
    let current = layout.current_env();
    let current_exists = current.as_ref().is_some_and(|path| path.is_dir());
    let mut problems = Vec::new();

    let mut receipt = None;
    match (&current, current_exists) {
        (Some(path), true) => match std::fs::read_to_string(path.join(RECEIPT_FILE)) {
            Ok(text) => match Receipt::parse(&text) {
                Ok(parsed) => receipt = Some(parsed),
                Err(finding) => problems.push(finding),
            },
            Err(error) => problems.push(Finding::error(
                "airlock.state.receipt",
                format!("cannot read the receipt in {}: {error}", path.display()),
            )),
        },
        (Some(path), false) => problems.push(Finding::error(
            "airlock.state.dangling",
            format!(
                "`current` names {}, which does not exist; the last restore did not complete or \
                 its environment was deleted",
                path.display()
            ),
        )),
        (None, _) => problems.push(Finding::warning(
            "airlock.state.empty",
            format!(
                "{} holds no active environment; run `restore` to activate one",
                layout.root().display()
            ),
        )),
    }

    Status {
        root_exists,
        current,
        current_exists,
        receipt,
        problems,
    }
}

#[cfg(test)]
mod tests {
    use super::{Layout, Receipt, Status, activate, default_root, inspect, measure};
    use crate::hash::sha256_hex;
    use crate::payload::verify;
    use crate::signature::Lane;

    const COMMIT: &str = "0123456789abcdef0123456789abcdef01234567";

    /// A real `.conda` with one executable in it, and `marker` as that executable's output.
    ///
    /// Activation installs a prefix now, so an "artifact" that is not an archive would test the
    /// refusal path instead of the behaviour under test.
    fn conda(marker: &str) -> Vec<u8> {
        use std::io::Write;

        let executable = format!("#!/bin/sh\nprintf '{marker}\\n'\n").into_bytes();
        let index = br#"{"name":"fixture","version":"0.1.0","build":"h1234567_0","build_number":0,"subdir":"linux-64","depends":[]}"#;
        let paths = format!(
            r#"{{"paths":[{{"_path":"bin/fixture","path_type":"hardlink","sha256":"{}","size_in_bytes":{}}}],"paths_version":1}}"#,
            sha256_hex(&executable),
            executable.len()
        );

        let mut info = tar::Builder::new(Vec::new());
        for (name, bytes) in [
            ("info/index.json", index.as_slice()),
            ("info/paths.json", paths.as_bytes()),
        ] {
            let mut header = tar::Header::new_gnu();
            header.set_size(bytes.len() as u64);
            header.set_mode(0o644);
            header.set_mtime(0);
            header.set_cksum();
            info.append_data(&mut header, name, bytes)
                .expect("info entry");
        }
        let info = info.into_inner().expect("info tar");

        let mut payload = tar::Builder::new(Vec::new());
        let mut header = tar::Header::new_gnu();
        header.set_size(executable.len() as u64);
        header.set_mode(0o755);
        header.set_mtime(0);
        header.set_cksum();
        payload
            .append_data(&mut header, "bin/fixture", executable.as_slice())
            .expect("payload entry");
        let payload = payload.into_inner().expect("payload tar");

        let mut buffer = std::io::Cursor::new(Vec::new());
        let mut zip = zip::ZipWriter::new(&mut buffer);
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Stored);
        zip.start_file("metadata.json", options).expect("metadata");
        zip.write_all(br#"{"conda_pkg_format_version": 2}"#)
            .expect("metadata bytes");
        zip.start_file("info-fixture.tar.zst", options)
            .expect("info");
        zip.write_all(&zstd::encode_all(info.as_slice(), 0).expect("compress"))
            .expect("info bytes");
        zip.start_file("pkg-fixture.tar.zst", options)
            .expect("payload");
        zip.write_all(&zstd::encode_all(payload.as_slice(), 0).expect("compress"))
            .expect("payload bytes");
        zip.finish().expect("finish archive");
        buffer.into_inner()
    }

    fn payload() -> tempfile::TempDir {
        let scratch = tempfile::TempDir::new().expect("a temporary payload");
        let bytes = conda("the only artifact");
        std::fs::write(scratch.path().join("airlock-0.1.0.conda"), &bytes).expect("artifact");
        let manifest = format!(
            r#"{{"schema":1,"version":"0.1.0","commit":"{COMMIT}","packages":[
                 {{"name":"universe-airlock","kind":"conda","version":"0.1.0","artifacts":[
                   {{"filename":"airlock-0.1.0.conda","sha256":"{}","size":{},"stores":["conda-static"]}}]}}]}}"#,
            sha256_hex(&bytes),
            bytes.len()
        );
        std::fs::write(scratch.path().join("universe-release.json"), manifest).expect("manifest");
        scratch
    }

    #[test]
    fn activation_materialises_the_payload_and_points_current_at_it() {
        let payload = payload();
        let state = tempfile::TempDir::new().expect("a scratch directory");
        let layout = Layout::new(state.path().join("state"));
        let report = verify(payload.path(), Lane::Developer, Some(COMMIT)).expect("verifies");

        let outcome = activate(&layout, payload.path(), &report).expect("activates");

        assert!(!outcome.reused);
        assert_eq!(outcome.previous, None);
        assert_eq!(layout.current_env().as_deref(), Some(outcome.env.as_path()));
        // The verified archive is kept beside the prefix it produced.
        assert_eq!(
            std::fs::read(outcome.env.join("airlock-0.1.0.conda")).expect("the artifact"),
            conda("the only artifact")
        );
        let receipt = Receipt::parse(
            &std::fs::read_to_string(outcome.env.join(super::RECEIPT_FILE)).expect("a receipt"),
        )
        .expect("parses");
        assert_eq!(receipt.version, "0.1.0");
        assert_eq!(receipt.commit, COMMIT);
        assert_eq!(receipt.signature, "unsigned");
        assert_eq!(receipt.artifacts.len(), 1);

        // Activation installs a prefix now, so the environment holds more than the archives:
        // `prefix` is what a caller puts on `PATH`, and the executable bit is why it can.
        assert_eq!(outcome.prefix, outcome.env.join(super::PREFIX_DIR));
        use std::os::unix::fs::PermissionsExt;
        let installed = outcome.prefix.join("bin/fixture");
        let mode = std::fs::metadata(&installed)
            .expect("the installed executable")
            .permissions()
            .mode()
            & 0o777;
        assert_eq!(mode, 0o755, "the installed executable is {mode:o}");
        assert_eq!(
            std::fs::read(&installed).expect("installed bytes"),
            b"#!/bin/sh\nprintf 'the only artifact\\n'\n"
        );
        assert!(outcome.entries >= 1, "nothing was installed");
        assert!(outcome.bytes >= 1);
    }

    #[test]
    fn restoring_the_same_payload_twice_copies_nothing_the_second_time() {
        let payload = payload();
        let state = tempfile::TempDir::new().expect("a scratch directory");
        let layout = Layout::new(state.path().join("state"));
        let report = verify(payload.path(), Lane::Developer, None).expect("verifies");

        let first = activate(&layout, payload.path(), &report).expect("activates");
        let second = activate(&layout, payload.path(), &report).expect("activates again");

        assert!(!first.reused);
        assert!(second.reused);
        assert_eq!(first.env, second.env);
        assert_eq!(second.previous, Some(first.env.clone()));
        // A reuse installs nothing, but the environment still holds a prefix; reporting zero
        // files for a directory full of them would be the kind of quiet wrong number a receipt
        // exists to prevent.
        assert_eq!(second.entries, measure(&second.prefix).0);
        assert_eq!(second.bytes, measure(&second.prefix).1);
        assert!(
            second.entries >= 1,
            "the reused environment holds no prefix"
        );
    }

    #[test]
    fn a_failure_leaves_the_previous_environment_active_and_intact() {
        // The heart of `UNI-33` #3. The payload is valid when verified and corrupted before
        // the copy, which is the window a transactional restore has to survive.
        let payload = payload();
        let state = tempfile::TempDir::new().expect("a scratch directory");
        let layout = Layout::new(state.path().join("state"));
        let report = verify(payload.path(), Lane::Developer, None).expect("verifies");
        let first = activate(&layout, payload.path(), &report).expect("activates");

        // Publish a different, larger payload as a staging target, then corrupt it mid-flight.
        let second = tempfile::TempDir::new().expect("a scratch directory");
        let bytes = conda("a different artifact");
        std::fs::write(second.path().join("airlock-0.1.0.conda"), &bytes).expect("artifact");
        let manifest = format!(
            r#"{{"schema":1,"version":"0.2.0","commit":"{COMMIT}","packages":[
                 {{"name":"universe-airlock","kind":"conda","version":"0.2.0","artifacts":[
                   {{"filename":"airlock-0.1.0.conda","sha256":"{}","size":{},"stores":["conda-static"]}}]}}]}}"#,
            sha256_hex(&bytes),
            bytes.len()
        );
        std::fs::write(second.path().join("universe-release.json"), manifest).expect("manifest");
        let report = verify(second.path(), Lane::Developer, None).expect("verifies");
        std::fs::write(
            second.path().join("airlock-0.1.0.conda"),
            b"tampered after verification\n",
        )
        .expect("tamper");

        let failure = activate(&layout, second.path(), &report).expect_err("must refuse");

        assert_eq!(failure.code(), "airlock.payload.changed");
        // `current` still names the first environment, and that environment is untouched.
        assert_eq!(layout.current_env().as_deref(), Some(first.env.as_path()));
        assert_eq!(
            std::fs::read(first.env.join("airlock-0.1.0.conda")).expect("still there"),
            std::fs::read(payload.path().join("airlock-0.1.0.conda")).expect("the first payload")
        );
        // And nothing was left behind by the abandoned staging.
        let leftovers: Vec<String> = std::fs::read_dir(layout.envs())
            .expect("envs")
            .flatten()
            .map(|entry| entry.file_name().to_string_lossy().into_owned())
            .filter(|name| name.starts_with(".staging"))
            .collect();
        assert!(leftovers.is_empty(), "{leftovers:?}");
    }

    #[test]
    fn a_second_restore_records_what_it_replaced() {
        let state = tempfile::TempDir::new().expect("a scratch directory");
        let layout = Layout::new(state.path().join("state"));
        let first_payload = payload();
        let first = activate(
            &layout,
            first_payload.path(),
            &verify(first_payload.path(), Lane::Developer, None).expect("verifies"),
        )
        .expect("activates");

        let second = tempfile::TempDir::new().expect("a scratch directory");
        let bytes = conda("a different artifact");
        std::fs::write(second.path().join("airlock-0.1.0.conda"), &bytes).expect("artifact");
        let manifest = format!(
            r#"{{"schema":1,"version":"0.2.0","commit":"{COMMIT}","packages":[
                 {{"name":"universe-airlock","kind":"conda","version":"0.2.0","artifacts":[
                   {{"filename":"airlock-0.1.0.conda","sha256":"{}","size":{},"stores":["conda-static"]}}]}}]}}"#,
            sha256_hex(&bytes),
            bytes.len()
        );
        std::fs::write(second.path().join("universe-release.json"), manifest).expect("manifest");
        let outcome = activate(
            &layout,
            second.path(),
            &verify(second.path(), Lane::Developer, None).expect("verifies"),
        )
        .expect("activates");

        assert_eq!(outcome.previous, Some(first.env.clone()));
        assert_eq!(layout.previous_env(), Some(first.env));
        assert_eq!(layout.current_env(), Some(outcome.env));
    }

    #[test]
    fn doctor_describes_an_empty_and_a_populated_root_without_changing_either() {
        let state = tempfile::TempDir::new().expect("a scratch directory");
        let layout = Layout::new(state.path().join("state"));
        let empty: Status = inspect(&layout);
        assert!(!empty.root_exists);
        assert!(!empty.problems.is_empty(), "an absent root is worth saying");

        let payload = payload();
        activate(
            &layout,
            payload.path(),
            &verify(payload.path(), Lane::Developer, None).expect("verifies"),
        )
        .expect("activates");

        let populated = inspect(&layout);
        assert!(populated.root_exists);
        assert!(populated.current_exists);
        assert!(populated.problems.is_empty(), "{:?}", populated.problems);
        assert_eq!(populated.receipt.expect("a receipt").version, "0.1.0");
    }

    #[test]
    fn doctor_names_a_current_that_points_nowhere() {
        let state = tempfile::TempDir::new().expect("a scratch directory");
        let layout = Layout::new(state.path().join("state"));
        std::fs::create_dir_all(layout.envs()).expect("envs");
        std::os::unix::fs::symlink("envs/gone", layout.current()).expect("link");

        let status = inspect(&layout);
        assert!(!status.current_exists);
        assert_eq!(status.problems[0].code(), "airlock.state.dangling");
    }

    #[test]
    fn the_default_root_is_the_project_unless_the_environment_says_otherwise() {
        let project = std::path::Path::new("/project");
        assert_eq!(
            default_root(project, None),
            std::path::Path::new("/project/.universe-airlock/state")
        );
        assert_eq!(
            default_root(project, Some("/var/lib/airlock")),
            std::path::Path::new("/var/lib/airlock")
        );
    }

    #[test]
    fn a_receipt_round_trips_and_an_unknown_schema_is_refused() {
        let receipt = Receipt {
            version: "0.1.0".to_owned(),
            commit: COMMIT.to_owned(),
            manifest_sha256: "a".repeat(64),
            signature: "unsigned".to_owned(),
            artifacts: vec![crate::payload::Artifact {
                filename: "airlock-0.1.0.conda".to_owned(),
                sha256: "b".repeat(64),
                size: 19,
            }],
        };
        assert_eq!(Receipt::parse(&receipt.render()), Ok(receipt));

        let future = "schema=2\nversion=0.1.0\ncommit=x\nmanifest_sha256=y\n";
        let finding = Receipt::parse(future).expect_err("refuses");
        assert_eq!(finding.code(), "airlock.state.receipt");
        assert!(finding.message().contains("schema 2"));
    }
}
