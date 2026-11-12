//! Reading a lockfile as the index of a mirror.
//!
//! A lock already names every artifact it needs: an exact URL, a digest, and enough of the
//! package's identity to say which one it is. Nothing here resolves, ranges over versions, or
//! asks a channel anything — it turns text into a list of pinned artifacts, and the mirror
//! verb becomes the same three steps whatever the text was: read, fetch, publish.
//!
//! That is why the format lives behind [`Adapter`] and nothing else does. Adding a lock format
//! is one implementation of four methods; the CLI gains no flag, the renderer gains no job
//! shape, and the publication path is untouched. The adaptation is also the only place a
//! format's *own* idea of a package has to be doubted: a lock that omits a digest, or names a
//! Git dependency whose bytes no registry will serve, is refused here rather than becoming a
//! fetch that fails later with less to say.
//!
//! Two adapters ship. `pixi` reads this repository's own `pixi.lock`, which mixes Conda
//! archives and PyPI wheels; `cargo` reads `Cargo.lock`, whose registry packages are crates.io
//! `.crate` tarballs. `LockAdapter` in the manifest names a third (`bun`), and the renderer
//! emits a job for it, so the refusal for an adapter this build cannot read says which ones it
//! can rather than pretending the format does not exist.

use std::collections::BTreeMap;

use serde_yaml::Value;
use universe_core::diagnostic::{Diagnostic, Severity};
use universe_core::manifest::LockAdapter;

/// What kind of archive a pinned artifact is.
///
/// It decides nothing about how the bytes travel — a mirror stores them all the same way —
/// but a consumer reading the index needs to know whether it is looking at a Conda package, a
/// wheel, or a crate before it can install one.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Locked {
    /// A Conda archive: `.conda` or `.tar.bz2`.
    Conda,
    /// A PyPI wheel.
    Wheel,
    /// A crates.io `.crate` tarball.
    Crate,
}

impl Locked {
    /// Stable lowercase spelling, as the index records it.
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::Conda => "conda",
            Self::Wheel => "wheel",
            Self::Crate => "crate",
        }
    }
}

/// One artifact a lock pins: the unit a mirror moves.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct Pinned {
    /// Package name, as the lock spells it.
    pub(crate) name: String,
    /// Exact version, from the lock and nowhere else.
    pub(crate) version: String,
    /// Which ecosystem's archive this is.
    pub(crate) kind: Locked,
    /// Platform the artifact is for: a Conda subdir, a wheel's platform tag, or `any`.
    pub(crate) subdir: String,
    /// Where the bytes are fetched from, exactly as the lock records it.
    pub(crate) url: String,
    /// Lowercase hex SHA-256, which is what the mirror verifies and stores by.
    pub(crate) sha256: String,
    /// Size in bytes, when the lock records one.
    pub(crate) size: Option<u64>,
}

impl Pinned {
    /// The file name the artifact keeps in the mirror.
    ///
    /// Taken from the URL rather than rebuilt from the identity: a Conda archive's name
    /// carries its build string, and reconstructing it here would be a second place to get
    /// that wrong.
    pub(crate) fn filename(&self) -> String {
        let name = self
            .url
            .split('/')
            .next_back()
            .filter(|name| !name.is_empty())
            .unwrap_or(self.url.as_str());
        // A query string is a property of the request, not of the file.
        name.split('?').next().unwrap_or(name).to_owned()
    }

    /// Where in the mirror the artifact lives.
    ///
    /// Content-addressed, so the path is a function of the bytes: two locks that name the
    /// same artifact produce the same path, and a republish of either leaves the file where it
    /// was. The digest is cut into a two-character directory because a flat directory of
    /// thousands of files is slow to list on the filesystems a mirror is checked out on.
    pub(crate) fn path(&self) -> String {
        format!(
            "blobs/sha256/{}/{}/{}",
            &self.sha256[..2],
            self.sha256,
            self.filename()
        )
    }

    /// How a diagnostic names this artifact: the two fields a person can act on.
    pub(crate) fn identity(&self) -> String {
        format!("{} {}", self.name, self.version)
    }
}

/// One lock format, read as a list of pinned artifacts.
///
/// Deliberately four methods and no state: an adapter that needed to accumulate across calls
/// would be a solver, and a mirror does not solve.
pub(crate) trait Adapter {
    /// Which format this is, in the manifest's own vocabulary.
    fn format(&self) -> LockAdapter;

    /// The file this format is conventionally stored in.
    fn lock_file(&self) -> &'static str;

    /// Read `text` as a lock, or say why it is not one.
    ///
    /// # Errors
    ///
    /// Returns every problem the text has, rather than the first: a lock that is wrong in
    /// three places is one edit away from being read, and reporting one package per run turns
    /// that into three runs.
    fn read(&self, text: &str) -> Result<Vec<Pinned>, Vec<Diagnostic>>;
}

/// The identifier the CLI and the renderer's matrix use, in the order they are listed.
pub(crate) const ADAPTERS: [LockAdapter; 2] = [LockAdapter::Pixi, LockAdapter::Cargo];

/// Where crates.io serves a crate archive, which is a function of the name and the version.
///
/// A `Cargo.lock` names the registry, not the download host, so the layout is knowledge this
/// adapter carries rather than a field it reads. `--crates` overrides it for a registry that
/// speaks the same layout under another host, which is what makes the path testable without
/// reaching crates.io.
pub(crate) const CRATES_BASE: &str = "https://static.crates.io/crates";

/// The adapter for `name`, or a refusal naming the ones this build reads.
///
/// # Errors
///
/// Returns a diagnostic when the name is not one of [`ADAPTERS`]. `bun` is a declared format
/// whose reader has not landed, and the message says so instead of reporting an unknown verb.
pub(crate) fn adapter(name: &str) -> Result<Box<dyn Adapter>, Diagnostic> {
    adapter_with(name, CRATES_BASE)
}

/// The same lookup, reading crates from `crates`.
///
/// # Errors
///
/// Returns a diagnostic when the name is not one of [`ADAPTERS`].
pub(crate) fn adapter_with(name: &str, crates: &str) -> Result<Box<dyn Adapter>, Diagnostic> {
    match ADAPTERS
        .iter()
        .find(|adapter| adapter.as_str() == name)
        .copied()
    {
        Some(LockAdapter::Pixi) => Ok(Box::new(Pixi)),
        Some(LockAdapter::Cargo) => Ok(Box::new(Cargo {
            base: crates.to_owned(),
        })),
        // `Bun` and any future variant: the manifest may declare it, this build cannot read it.
        _ => Err(Diagnostic::new(
            Severity::Error,
            "airlock.mirror.adapter.unknown",
            format!(
                "'{name}' is not a lock format this build reads; it reads {}",
                ADAPTERS
                    .iter()
                    .map(|adapter| adapter.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        )),
    }
}

/// `pixi.lock`, read as YAML.
struct Pixi;

impl Adapter for Pixi {
    fn format(&self) -> LockAdapter {
        LockAdapter::Pixi
    }

    fn lock_file(&self) -> &'static str {
        "pixi.lock"
    }

    fn read(&self, text: &str) -> Result<Vec<Pinned>, Vec<Diagnostic>> {
        let document: Value = serde_yaml::from_str(text).map_err(|error| {
            vec![Diagnostic::new(
                Severity::Error,
                "airlock.mirror.lock.parse",
                format!("cannot read a pixi.lock: {error}"),
            )]
        })?;
        // Schema 6 and 7 both list one record per package with the format as a key, which is
        // the shape this reader understands. An older file is a different shape, not a
        // newer one to be hopeful about.
        let version = document.get("version").and_then(Value::as_u64);
        if !matches!(version, Some(6 | 7)) {
            return Err(vec![Diagnostic::new(
                Severity::Error,
                "airlock.mirror.lock.version",
                format!(
                    "this pixi.lock declares version {}, and this build reads 6 and 7",
                    version.map_or_else(|| "nothing".to_owned(), |value| value.to_string())
                ),
            )]);
        }
        let Some(records) = document.get("packages").and_then(Value::as_sequence) else {
            return Err(vec![Diagnostic::new(
                Severity::Error,
                "airlock.mirror.lock.packages",
                "this pixi.lock lists no packages, so there is nothing to mirror",
            )]);
        };

        let mut errors: Vec<Diagnostic> = Vec::new();
        let mut pinned: Vec<Pinned> = Vec::new();
        for (index, record) in records.iter().enumerate() {
            match conda_or_wheel(record, index) {
                Ok(Some(artifact)) => pinned.push(artifact),
                Ok(None) => {}
                Err(mut diagnostic) => errors.append(&mut diagnostic),
            }
        }
        if !errors.is_empty() {
            return Err(errors);
        }
        Ok(unique(pinned))
    }
}

/// One record of a `pixi.lock`: a Conda archive or a wheel.
///
/// `Ok(None)` is a record this reader does not treat as an artifact, which today is only an
/// entry whose format key is present with no URL — a broken record the caller reports.
fn conda_or_wheel(record: &Value, index: usize) -> Result<Option<Pinned>, Vec<Diagnostic>> {
    let field = format!("packages[{index}]");
    let conda = record.get("conda").and_then(Value::as_str);
    let wheel = record.get("pypi").and_then(Value::as_str);
    let (url, kind) = match (conda, wheel) {
        (Some(url), None) => (url, Locked::Conda),
        (None, Some(url)) => (url, Locked::Wheel),
        _ => {
            return Err(vec![
                Diagnostic::new(
                    Severity::Error,
                    "airlock.mirror.lock.record",
                    "this record names neither exactly one of `conda` and `pypi`",
                )
                .with_path(field),
            ]);
        }
    };
    let mut errors: Vec<Diagnostic> = Vec::new();
    let Some(sha256) = record.get("sha256").and_then(Value::as_str) else {
        return Err(vec![
            Diagnostic::new(
                Severity::Error,
                "airlock.mirror.lock.digest",
                format!(
                    "{url} carries no sha256, and an artifact a mirror cannot verify is a \
                         download with a hope attached"
                ),
            )
            .with_path(format!("{field}.sha256")),
        ]);
    };
    if !is_digest(sha256) {
        errors.push(
            Diagnostic::new(
                Severity::Error,
                "airlock.mirror.lock.digest",
                format!("{url} records a sha256 that is not 64 lowercase hex characters"),
            )
            .with_path(format!("{field}.sha256")),
        );
    }
    let size = record.get("size").and_then(Value::as_u64);
    let (name, version, subdir) = match kind {
        Locked::Conda => match conda_identity(url) {
            Ok(identity) => identity,
            Err(diagnostic) => {
                errors.push(diagnostic.with_path(format!("{field}.conda")));
                return Err(errors);
            }
        },
        _ => match wheel_identity(record, url) {
            Ok(identity) => identity,
            Err(diagnostic) => {
                errors.push(diagnostic.with_path(format!("{field}.pypi")));
                return Err(errors);
            }
        },
    };
    if !errors.is_empty() {
        return Err(errors);
    }
    Ok(Some(Pinned {
        name,
        version,
        kind,
        subdir,
        url: url.to_owned(),
        sha256: sha256.to_owned(),
        size,
    }))
}

/// A Conda archive's identity, from its URL.
///
/// The lock records the URL and nothing else about a Conda package here, so the file name is
/// the source: `name-version-build.conda`, and the directory above it is the subdir.
fn conda_identity(url: &str) -> Result<(String, String, String), Diagnostic> {
    let filename = url
        .split('/')
        .next_back()
        .filter(|name| !name.is_empty())
        .unwrap_or("");
    let stem = filename
        .strip_suffix(".conda")
        .or_else(|| filename.strip_suffix(".tar.bz2"))
        .ok_or_else(|| {
            Diagnostic::new(
                Severity::Error,
                "airlock.mirror.lock.record",
                format!("{url} is not a .conda or .tar.bz2 archive"),
            )
        })?;
    let mut parts = stem.rsplitn(3, '-');
    let build = parts.next().unwrap_or_default();
    let version = parts.next().unwrap_or_default();
    let name = parts.next().unwrap_or_default();
    if name.is_empty() || version.is_empty() || build.is_empty() {
        return Err(Diagnostic::new(
            Severity::Error,
            "airlock.mirror.lock.record",
            format!("{filename} is not spelled name-version-build"),
        ));
    }
    // The subdir is the path segment above the file: `.../linux-64/pkg.conda`.
    let subdir = url
        .split('/')
        .rev()
        .nth(1)
        .filter(|segment| !segment.is_empty() && !segment.contains(':'))
        .unwrap_or("any");
    Ok((name.to_owned(), version.to_owned(), subdir.to_owned()))
}

/// A wheel's identity, from the record's own fields.
///
/// A PyPI record carries `name` and `version`, and they are the lock's statement about the
/// package; the file name only contributes the platform tag it was built for.
fn wheel_identity(record: &Value, url: &str) -> Result<(String, String, String), Diagnostic> {
    let name = record.get("name").and_then(Value::as_str);
    let version = record.get("version").and_then(Value::as_str);
    let (Some(name), Some(version)) = (name, version) else {
        return Err(Diagnostic::new(
            Severity::Error,
            "airlock.mirror.lock.record",
            format!("{url} carries no name and version, and a wheel mirror records both"),
        ));
    };
    let filename = url
        .split('/')
        .next_back()
        .filter(|name| !name.is_empty())
        .unwrap_or("");
    let subdir = filename
        .strip_suffix(".whl")
        .and_then(|stem| stem.rsplit('-').next())
        .filter(|tag| !tag.is_empty())
        .unwrap_or("any");
    Ok((name.to_owned(), version.to_owned(), subdir.to_owned()))
}

/// `Cargo.lock`, read as TOML.
struct Cargo {
    /// The base URL a crate archive hangs off.
    base: String,
}

impl Adapter for Cargo {
    fn format(&self) -> LockAdapter {
        LockAdapter::Cargo
    }

    fn lock_file(&self) -> &'static str {
        "Cargo.lock"
    }

    fn read(&self, text: &str) -> Result<Vec<Pinned>, Vec<Diagnostic>> {
        let document: toml_edit::DocumentMut = text.parse().map_err(|error| {
            vec![Diagnostic::new(
                Severity::Error,
                "airlock.mirror.lock.parse",
                format!("cannot read a Cargo.lock: {error}"),
            )]
        })?;
        let Some(packages) = document
            .get("package")
            .and_then(|item| item.as_array_of_tables())
        else {
            return Err(vec![Diagnostic::new(
                Severity::Error,
                "airlock.mirror.lock.packages",
                "this Cargo.lock lists no packages, so there is nothing to mirror",
            )]);
        };
        let mut errors: Vec<Diagnostic> = Vec::new();
        let mut pinned: Vec<Pinned> = Vec::new();
        for (index, package) in packages.iter().enumerate() {
            match crate_record(package, index, &self.base) {
                Ok(Some(artifact)) => pinned.push(artifact),
                Ok(None) => {}
                Err(diagnostic) => errors.push(diagnostic),
            }
        }
        if !errors.is_empty() {
            return Err(errors);
        }
        Ok(unique(pinned))
    }
}

/// One `[[package]]` of a `Cargo.lock`.
///
/// `Ok(None)` is a workspace member: it has no `source`, so its bytes are this repository's
/// rather than a registry's, and there is nothing to mirror.
fn crate_record(
    package: &toml_edit::Table,
    index: usize,
    base: &str,
) -> Result<Option<Pinned>, Diagnostic> {
    let field = format!("package[{index}]");
    let name = package.get("name").and_then(|item| item.as_str());
    let version = package.get("version").and_then(|item| item.as_str());
    let source = package.get("source").and_then(|item| item.as_str());
    let Some(source) = source else {
        return Ok(None);
    };
    let (Some(name), Some(version)) = (name, version) else {
        return Err(Diagnostic::new(
            Severity::Error,
            "airlock.mirror.lock.record",
            format!("a package sourced from '{source}' has no name and version"),
        )
        .with_path(field));
    };
    // A registry entry is a crate archive at a URL the registry's layout fixes; anything else
    // — a Git dependency, a path, a patch — has bytes no registry serves and no checksum to
    // check them with, which is a thing to say rather than to skip.
    if !source.starts_with("registry+") && !source.starts_with("sparse+") {
        return Err(Diagnostic::new(
            Severity::Error,
            "airlock.mirror.lock.source",
            format!(
                "{name} {version} comes from '{source}', which has no registry archive for a \
                 mirror to fetch; a lock with such a dependency cannot be mirrored"
            ),
        )
        .with_path(field));
    }
    let Some(checksum) = package.get("checksum").and_then(|item| item.as_str()) else {
        return Err(Diagnostic::new(
            Severity::Error,
            "airlock.mirror.lock.digest",
            format!("{name} {version} carries no checksum, and a mirror verifies what it serves"),
        )
        .with_path(format!("{field}.checksum")));
    };
    if !is_digest(checksum) {
        return Err(Diagnostic::new(
            Severity::Error,
            "airlock.mirror.lock.digest",
            format!("{name} {version} records a checksum that is not 64 lowercase hex characters"),
        )
        .with_path(format!("{field}.checksum")));
    }
    Ok(Some(Pinned {
        name: name.to_owned(),
        version: version.to_owned(),
        kind: Locked::Crate,
        subdir: "any".to_owned(),
        // crates.io's download path is a function of the name and version, so a lock that
        // records only the registry does not need the index to be asked where the bytes are.
        url: format!("{base}/{name}/{name}-{version}.crate"),
        sha256: checksum.to_owned(),
        size: None,
    }))
}

/// The same artifact named twice is one artifact.
///
/// A lock can reach one package through two environments, and a mirror that fetched it twice
/// would report work it did not do. Duplicates are dropped by mirror path, which is the
/// digest, and the survivors keep the lock's order.
fn unique(pinned: Vec<Pinned>) -> Vec<Pinned> {
    let mut seen: BTreeMap<String, ()> = BTreeMap::new();
    let mut kept: Vec<Pinned> = Vec::with_capacity(pinned.len());
    for artifact in pinned {
        if seen.insert(artifact.path(), ()).is_none() {
            kept.push(artifact);
        }
    }
    kept.sort_by_key(Pinned::path);
    kept
}

/// Whether `value` is a SHA-256 as hex.
///
/// Checked here rather than at the fetch, because a lock with a malformed digest would
/// otherwise fail after downloading the bytes it cannot check.
fn is_digest(value: &str) -> bool {
    value.len() == 64
        && value
            .chars()
            .all(|character| character.is_ascii_digit() || ('a'..='f').contains(&character))
}

#[cfg(test)]
mod tests {
    use super::{CRATES_BASE, Cargo, LockAdapter, Locked, Pixi, adapter, adapter_with, is_digest};
    use crate::lock::Adapter;

    /// The `Cargo.lock` adapter as the verb builds it.
    fn cargo() -> Cargo {
        Cargo {
            base: CRATES_BASE.to_owned(),
        }
    }

    const DIGEST: &str = "9f2c1d0e8b7a6c5d4e3f2a1b0c9d8e7f6a5b4c3d2e1f0a9b8c7d6e5f4a3b2c1d";

    fn pixi_lock(records: &str) -> String {
        format!("version: 7\npackages:\n{records}")
    }

    #[test]
    fn a_pixi_lock_becomes_one_pinned_artifact_per_record() {
        let text = pixi_lock(&format!(
            "- conda: https://conda.anaconda.org/conda-forge/linux-64/libgcc-16.2.0-ha9f2e26_7.conda\n  \
             sha256: {DIGEST}\n  size: 1234\n\
             - pypi: https://files.pythonhosted.org/packages/aa/bb/click-8.5.0-py3-none-any.whl\n  \
             name: click\n  version: 8.5.0\n  sha256: {DIGEST}\n"
        ));
        let pinned = Pixi.read(&text).expect("a lock this reader understands");
        assert_eq!(pinned.len(), 2);
        // Sorted by mirror path, which is the digest: the order is not the lock's.
        let conda = pinned
            .iter()
            .find(|artifact| artifact.name == "libgcc")
            .expect("the conda record");
        assert_eq!(conda.version, "16.2.0");
        assert_eq!(conda.subdir, "linux-64");
        assert_eq!(conda.kind, Locked::Conda);
        assert_eq!(conda.size, Some(1234));
        assert_eq!(
            conda.path(),
            format!("blobs/sha256/9f/{DIGEST}/libgcc-16.2.0-ha9f2e26_7.conda")
        );
    }

    #[test]
    fn a_name_with_hyphens_keeps_them() {
        let text = pixi_lock(&format!(
            "- conda: https://conda.anaconda.org/conda-forge/linux-64/_openmp_mutex-4.5-20_gnu.conda\n  \
             sha256: {DIGEST}\n"
        ));
        let pinned = Pixi.read(&text).expect("a lock this reader understands");
        assert_eq!(pinned[0].name, "_openmp_mutex");
        assert_eq!(pinned[0].version, "4.5");
    }

    #[test]
    fn a_lock_from_a_schema_this_build_does_not_read_is_refused() {
        let text = format!(
            "version: 5\npackages:\n- url: https://example.invalid/a.conda\n  sha256: {DIGEST}\n"
        );
        let errors = Pixi
            .read(&text)
            .expect_err("version 5 is a different shape");
        assert_eq!(errors[0].code(), "airlock.mirror.lock.version");
        assert!(errors[0].message().contains("5"));
    }

    #[test]
    fn a_record_without_a_digest_is_refused_and_names_the_url() {
        let text = pixi_lock(
            "- conda: https://conda.anaconda.org/conda-forge/linux-64/libgcc-16.2.0-ha9f2e26_7.conda\n",
        );
        let errors = Pixi.read(&text).expect_err("no digest, no mirror");
        assert_eq!(errors[0].code(), "airlock.mirror.lock.digest");
        assert!(errors[0].message().contains("libgcc-16.2.0"));
    }

    #[test]
    fn a_digest_that_is_not_hex_is_refused_before_anything_is_fetched() {
        let text = pixi_lock(
            "- conda: https://conda.anaconda.org/conda-forge/linux-64/libgcc-16.2.0-ha9f2e26_7.conda\n  \
             sha256: 1234\n",
        );
        let errors = Pixi
            .read(&text)
            .expect_err("a short digest is not a digest");
        assert_eq!(errors[0].code(), "airlock.mirror.lock.digest");
        assert!(!is_digest("1234"));
        assert!(is_digest(DIGEST));
    }

    #[test]
    fn a_record_naming_both_formats_is_refused() {
        let text = pixi_lock(&format!(
            "- conda: https://example.invalid/a.conda\n  pypi: https://example.invalid/a.whl\n  \
             sha256: {DIGEST}\n"
        ));
        let errors = Pixi.read(&text).expect_err("one record is one artifact");
        assert_eq!(errors[0].code(), "airlock.mirror.lock.record");
    }

    #[test]
    fn a_record_whose_file_name_is_not_a_conda_archive_is_refused() {
        let text = pixi_lock(&format!(
            "- conda: https://example.invalid/a.zip\n  sha256: {DIGEST}\n"
        ));
        let errors = Pixi.read(&text).expect_err("a zip is not a conda archive");
        assert_eq!(errors[0].code(), "airlock.mirror.lock.record");
    }

    #[test]
    fn a_wheel_record_without_a_version_is_refused() {
        let text = pixi_lock(&format!(
            "- pypi: https://files.pythonhosted.org/packages/aa/bb/click-8.5.0-py3-none-any.whl\n  \
             sha256: {DIGEST}\n"
        ));
        let errors = Pixi
            .read(&text)
            .expect_err("the lock is the only source of version truth");
        assert_eq!(errors[0].code(), "airlock.mirror.lock.record");
    }

    #[test]
    fn a_lock_that_is_not_yaml_is_refused() {
        let errors = Pixi.read("{ not: [ yaml").expect_err("not a lock");
        assert_eq!(errors[0].code(), "airlock.mirror.lock.parse");
    }

    #[test]
    fn two_records_with_the_same_digest_are_one_artifact() {
        let record = format!(
            "- conda: https://conda.anaconda.org/conda-forge/linux-64/libgcc-16.2.0-ha9f2e26_7.conda\n  \
             sha256: {DIGEST}\n"
        );
        let text = pixi_lock(&format!("{record}{record}"));
        let pinned = Pixi.read(&text).expect("a lock this reader understands");
        assert_eq!(pinned.len(), 1);
    }

    #[test]
    fn a_cargo_lock_becomes_one_pinned_crate_per_registry_package() {
        let text = format!(
            "version = 4\n\n\
             [[package]]\nname = \"serde\"\nversion = \"1.0.200\"\n\
             source = \"registry+https://github.com/rust-lang/crates.io-index\"\n\
             checksum = \"{DIGEST}\"\n\n\
             [[package]]\nname = \"universe-core\"\nversion = \"0.1.0\"\n"
        );
        let pinned = cargo().read(&text).expect("a lock this reader understands");
        assert_eq!(pinned.len(), 1, "the workspace member is not an artifact");
        assert_eq!(pinned[0].name, "serde");
        assert_eq!(pinned[0].kind, Locked::Crate);
        assert_eq!(pinned[0].subdir, "any");
        assert_eq!(
            pinned[0].url,
            "https://static.crates.io/crates/serde/serde-1.0.200.crate"
        );
    }

    #[test]
    fn a_git_dependency_is_refused_by_name() {
        let text = "version = 4\n\n[[package]]\nname = \"patched\"\nversion = \"0.1.0\"\n\
                    source = \"git+https://example.invalid/repo?rev=abc#abcdef\"\n";
        let errors = cargo()
            .read(text)
            .expect_err("no registry serves a git dependency");
        assert_eq!(errors[0].code(), "airlock.mirror.lock.source");
        assert!(errors[0].message().contains("patched 0.1.0"));
    }

    #[test]
    fn a_registry_package_without_a_checksum_is_refused() {
        let text = "version = 4\n\n[[package]]\nname = \"serde\"\nversion = \"1.0.200\"\n\
                    source = \"registry+https://github.com/rust-lang/crates.io-index\"\n";
        let errors = cargo().read(text).expect_err("nothing to verify against");
        assert_eq!(errors[0].code(), "airlock.mirror.lock.digest");
    }

    #[test]
    fn a_cargo_lock_with_no_packages_is_refused() {
        let errors = cargo()
            .read("version = 4\n")
            .expect_err("an empty lock mirrors nothing");
        assert_eq!(errors[0].code(), "airlock.mirror.lock.packages");
    }

    #[test]
    fn a_file_that_is_not_toml_is_refused() {
        let errors = cargo().read("[[package]\n").expect_err("not a lock");
        assert_eq!(errors[0].code(), "airlock.mirror.lock.parse");
    }

    /// A file this repository commits, read through the adapter under test.
    fn repository_file(name: &str) -> String {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join(name);
        std::fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("cannot read {}: {error}", path.display()))
    }

    #[test]
    fn this_repository_s_own_lock_reads_as_ninety_seven_archives_and_seven_wheels() {
        // The counts are the task's own measurements of this lock, taken independently of this
        // reader: a parser that quietly dropped a section would disagree with both numbers.
        let pinned = Pixi
            .read(&repository_file("pixi.lock"))
            .expect("the repository's own lock is readable");
        let conda = pinned
            .iter()
            .filter(|artifact| artifact.kind == Locked::Conda)
            .count();
        let wheels = pinned
            .iter()
            .filter(|artifact| artifact.kind == Locked::Wheel)
            .count();
        assert_eq!((conda, wheels), (97, 7));
        assert!(
            pinned.iter().all(|artifact| is_digest(&artifact.sha256)),
            "every record in a real lock carries a sha256"
        );
        assert!(
            pinned
                .iter()
                .all(|artifact| artifact.url.starts_with("https://")),
            "every record in a real lock carries an exact URL"
        );
        assert!(
            pinned.iter().all(|artifact| !artifact.subdir.is_empty()),
            "every record says which platform it is for"
        );
        // One artifact per mirror path, or the tree would have two files fighting for one name.
        let paths: std::collections::BTreeSet<String> =
            pinned.iter().map(super::Pinned::path).collect();
        assert_eq!(paths.len(), pinned.len());
    }

    #[test]
    fn this_repository_s_own_rust_lock_reads_as_registry_crates() {
        let pinned = cargo()
            .read(&repository_file("Cargo.lock"))
            .expect("the repository's own lock is readable");
        assert!(
            pinned.len() > 100,
            "a workspace this size locks more than {} crates",
            pinned.len()
        );
        assert!(pinned.iter().all(
            |artifact| artifact.kind == Locked::Crate && artifact.url.starts_with(CRATES_BASE)
        ));
        // The workspace's own members have no `source`, so they are not artifacts.
        assert!(
            !pinned
                .iter()
                .any(|artifact| artifact.name.starts_with("universe-"))
        );
    }

    #[test]
    fn each_known_adapter_answers_to_its_manifest_name_and_its_own_file() {
        for (name, file) in [("pixi", "pixi.lock"), ("cargo", "Cargo.lock")] {
            let adapter = adapter(name).expect("a known adapter");
            assert_eq!(adapter.lock_file(), file);
            assert_eq!(adapter.format().as_str(), name);
        }
        assert_eq!(Pixi.format(), LockAdapter::Pixi);
        assert_eq!(cargo().format(), LockAdapter::Cargo);
        // `--crates` moves the host, not the spelling of the path under it.
        let moved = adapter_with("cargo", "https://example.invalid/crates").expect("an adapter");
        assert_eq!(moved.format().as_str(), "cargo");
        let text = format!(
            "version = 4\n\n[[package]]\nname = \"serde\"\nversion = \"1.0.200\"\n\
             source = \"registry+https://github.com/rust-lang/crates.io-index\"\n\
             checksum = \"{DIGEST}\"\n"
        );
        let pinned = moved.read(&text).expect("a lock");
        assert_eq!(
            pinned[0].url,
            "https://example.invalid/crates/serde/serde-1.0.200.crate"
        );
    }

    #[test]
    fn an_adapter_this_build_cannot_read_is_refused_with_the_ones_it_can() {
        let Err(error) = adapter("bun") else {
            panic!("no bun reader in this build");
        };
        assert_eq!(error.code(), "airlock.mirror.adapter.unknown");
        assert!(error.message().contains("pixi"));
        assert!(error.message().contains("cargo"));
    }
}
