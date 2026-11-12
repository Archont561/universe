//! `universe airlock mirror`: publish what a lock pinned, and nothing else.
//!
//! A lock is already a complete, closed description of an environment: every artifact has an
//! exact URL and a digest, and no version is chosen at mirror time. So this verb is three steps
//! and no decisions — read the lock, fetch each artifact the mirror does not already hold, and
//! publish the result as a tree on a branch — and the interesting parts are all about refusing
//! to become something bigger:
//!
//! - **Nothing resolves.** There is no channel request, no repodata read, and no version range.
//!   The lock names bytes; if those bytes are absent or arrive wrong, the run fails and says
//!   which package.
//! - **The store is content-addressed.** An artifact lives at `blobs/sha256/<digest>/<file>`,
//!   so a republish of the same lock writes the same paths, and a republish of a changed lock
//!   adds the new ones beside the old. That is what makes an unchanged lock produce no commit:
//!   the tree is a function of the bytes, and the bytes did not change.
//! - **The mirror is additive.** An artifact a newer lock dropped stays on the branch, because
//!   a consumer restoring an older pin still needs it. Removing bytes from a branch nothing
//!   else can restore is a decision for a retention pass, not a side effect of publishing a
//!   lock that happens to be smaller.
//! - **A cached artifact is verified again.** The cache is on a producer's disk, and a
//!   truncated download from last week is exactly the kind of file a mirror would otherwise
//!   publish with confidence.
//!
//! What it does *not* do is stated in the same place: the tree is assembled in memory, so a
//! lock whose artifacts exceed one publish's transport ceiling is refused rather than sharded
//! across branches (`UNI-47` owns that), and `bun` is a declared adapter whose reader has not
//! landed.

use std::path::{Path, PathBuf};

use serde_json::json;
use universe_channel::store::{Http, Method, Request as HttpRequest};
use universe_channel::transport::{Budgets, ChannelTree, PushOutcome, Transport};
use universe_core::CommandOutput;
use universe_core::diagnostic::{Diagnostic, Severity};
use universe_core::manifest::Manifest;
use universe_workflow::sha256::sha256_hex;

use crate::artifacts::declared_token;
use crate::git::GitTransport;
use crate::lock::{self, Pinned};
use crate::output::{FINDINGS, USAGE, failure, human, json_success};
use crate::scratch::Scratch;

/// The branch prefix a mirror writes to when the manifest names none.
const DEFAULT_PREFIX: &str = "mirror";

/// The remote a run publishes to when the caller names none: the checkout it runs in.
const DEFAULT_REMOTE: &str = "origin";

/// The directory a mirror is kept in, relative to the project root.
///
/// Beside the payload and the state, and ignored for the same reason: it is a cache of bytes
/// the branch already holds, not source this repository owns.
const MIRROR_DIR: &str = ".universe-airlock/mirror";

/// The index a mirrored tree carries, naming what the lock pinned and where it landed.
///
/// It is the handover to whatever restores next: a consumer holding the tree can map a lock
/// entry to its path without re-deriving the mirror's layout, and a reader comparing two
/// branches can see which artifact came from which lock.
const INDEX: &str = "mirror.json";

/// Shape version of that index.
const INDEX_SCHEMA: u32 = 1;

/// What the caller asked for.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct Request {
    /// The project root.
    pub(crate) root: PathBuf,
    /// Which lock format to read, as the manifest spells it.
    pub(crate) adapter: String,
    /// The lockfile, when it is not the adapter's default name in the root.
    pub(crate) lock: Option<PathBuf>,
    /// Where the mirror is kept, when it is not the default directory.
    pub(crate) mirror: Option<PathBuf>,
    /// The Git remote, when it is not the checkout's `origin`.
    pub(crate) remote: Option<String>,
    /// The branch, when it is not `<prefix>/<adapter>`.
    pub(crate) branch: Option<String>,
    /// The base a crate archive is fetched from, when it is not crates.io's own layout.
    pub(crate) crates: Option<String>,
    /// Compare the remote with what this run would publish, and publish nothing.
    pub(crate) check: bool,
    /// Report what a run would do without fetching or publishing.
    pub(crate) dry_run: bool,
    /// Report as a JSON envelope.
    pub(crate) json: bool,
}

/// What one run did, which is also what it says.
struct Mirrored {
    /// Artifacts the lock pins.
    artifacts: usize,
    /// Artifacts this run downloaded.
    fetched: usize,
    /// Artifacts the mirror already held, verified again.
    reused: usize,
    /// Bytes the link pinned, counted from the mirror.
    bytes: u64,
    /// Digest of the index this run wrote.
    index: String,
    /// Where the tree went.
    branch: String,
    /// What the destination did.
    outcome: Option<PushOutcome>,
    /// Whether the destination already held the staged tree, for `--check`.
    matched: Option<bool>,
}

/// Run `universe airlock mirror`.
pub(crate) fn run(request: &Request, manifest: &Manifest) -> CommandOutput {
    let adapter = match request.crates.as_deref().map_or_else(
        || lock::adapter(&request.adapter),
        |base| lock::adapter_with(&request.adapter, base),
    ) {
        Ok(adapter) => adapter,
        Err(diagnostic) => return failure(USAGE, &[diagnostic]),
    };
    let lock_path = request.lock.clone().map_or_else(
        || request.root.join(adapter.lock_file()),
        |path| {
            if path.is_absolute() {
                path
            } else {
                request.root.join(path)
            }
        },
    );
    let Ok(text) = std::fs::read_to_string(&lock_path) else {
        return failure(
            FINDINGS,
            &[Diagnostic::new(
                Severity::Error,
                "airlock.mirror.lock.missing",
                format!("no {} to mirror", lock_path.display()),
            )
            .with_path(lock_path.display().to_string())],
        );
    };
    let pinned = match adapter.read(&text) {
        Ok(pinned) => pinned,
        Err(diagnostics) => return failure(FINDINGS, &diagnostics),
    };
    if pinned.is_empty() {
        // An empty lock is not an empty mirror: publishing an index that names nothing would
        // replace a tree a consumer is restoring from with one that restores nothing.
        return failure(
            FINDINGS,
            &[Diagnostic::new(
                Severity::Error,
                "airlock.mirror.empty",
                format!(
                    "{} pins no artifact, and an empty mirror is not what a restore expects",
                    lock_path.display()
                ),
            )
            .with_path(lock_path.display().to_string())],
        );
    }

    let prefix = manifest
        .airlock
        .as_ref()
        .and_then(|airlock| airlock.developer.as_ref())
        .and_then(|developer| developer.mirror.clone())
        .unwrap_or_else(|| DEFAULT_PREFIX.to_owned());
    let branch = request
        .branch
        .clone()
        .unwrap_or_else(|| format!("{prefix}/{}", adapter.format().as_str()));
    let remote = request
        .remote
        .clone()
        .unwrap_or_else(|| DEFAULT_REMOTE.to_owned());
    let directory = request
        .mirror
        .clone()
        .unwrap_or_else(|| request.root.join(MIRROR_DIR));
    let directory = directory.join(adapter.format().as_str());

    let mut mirrored = Mirrored {
        artifacts: pinned.len(),
        fetched: 0,
        reused: 0,
        bytes: 0,
        index: String::new(),
        branch: branch.clone(),
        outcome: None,
        matched: None,
    };
    let mut to_fetch = Vec::new();
    for artifact in &pinned {
        match cached(&directory, artifact) {
            Ok(Some(bytes)) => {
                mirrored.reused += 1;
                mirrored.bytes += bytes;
            }
            Ok(None) => to_fetch.push(artifact),
            Err(diagnostic) => return failure(FINDINGS, &[diagnostic]),
        }
    }
    if request.dry_run {
        return report(
            request,
            &mirrored,
            Some(pinned.len() - mirrored.reused),
            None,
        );
    }
    for artifact in &to_fetch {
        match download(&directory, artifact) {
            Ok(bytes) => {
                mirrored.fetched += 1;
                mirrored.bytes += bytes;
            }
            Err(diagnostic) => return failure(FINDINGS, &[diagnostic]),
        }
    }

    let index = match write_index(
        &directory,
        adapter.format().as_str(),
        &text,
        &branch,
        &pinned,
    ) {
        Ok(index) => index,
        Err(diagnostic) => return failure(FINDINGS, &[diagnostic]),
    };
    mirrored.index = index;
    let tree = match tree_of(&directory) {
        Ok(tree) => tree,
        Err(diagnostic) => return failure(FINDINGS, &[diagnostic]),
    };
    // The same ceilings a channel publish is measured against: a mirror that no transport
    // would accept should say so before it writes a tree, not during a push.
    if let Err(error) = Budgets::default().check(&tree) {
        return failure(
            FINDINGS,
            &[Diagnostic::new(
                Severity::Error,
                "airlock.mirror.budget",
                error.to_string(),
            )],
        );
    }

    let scratch = match Scratch::temporary() {
        Ok(scratch) => scratch,
        Err(diagnostic) => return failure(FINDINGS, &[diagnostic]),
    };
    // The token a branch store declares, when the environment holds one. `GITHUB_TOKEN` in the
    // rendered job is the whole credential story: the mirror needs `contents: write` and
    // nothing else, so an absent secret falls back to git's own authority rather than failing.
    let token = manifest
        .stores
        .iter()
        .find(|store| store.writable && store.kind.is_git_branch())
        .and_then(declared_token);
    let transport =
        GitTransport::new(&remote, &branch, scratch.path()).authenticated(token.as_deref());
    if request.check {
        return match (transport.current(), transport.staged_tree(&tree)) {
            (Ok(current), Ok(staged)) => {
                let matched = current.is_some_and(|identity| identity.tree == staged);
                mirrored.matched = Some(matched);
                report(request, &mirrored, None, Some(&staged))
            }
            (Err(error), _) | (_, Err(error)) => failure(
                FINDINGS,
                &[Diagnostic::new(
                    Severity::Error,
                    "airlock.mirror.transport.failed",
                    error.to_string(),
                )],
            ),
        };
    }

    let message = format!(
        "mirror {}: {} artifact(s) from {}\n",
        adapter.format().as_str(),
        pinned.len(),
        adapter.lock_file()
    );
    match transport.push(&tree, &message) {
        Ok(outcome) => {
            mirrored.outcome = Some(outcome);
            report(request, &mirrored, None, None)
        }
        Err(error) => failure(
            FINDINGS,
            &[Diagnostic::new(
                Severity::Error,
                "airlock.mirror.transport.failed",
                error.to_string(),
            )],
        ),
    }
}

/// What the mirror already holds for `artifact`, verified again.
///
/// `Ok(None)` means "not cached", and a cached file whose bytes no longer match is not cached
/// either: it is removed and fetched like any other missing artifact, because publishing a
/// file that fails its own digest is worse than downloading it twice.
fn cached(directory: &Path, artifact: &Pinned) -> Result<Option<u64>, Diagnostic> {
    let path = directory.join(artifact.path());
    let Ok(bytes) = std::fs::read(&path) else {
        return Ok(None);
    };
    if sha256_hex(&bytes) != artifact.sha256 {
        std::fs::remove_file(&path)
            .map_err(|error| io("airlock.mirror.mirror.io", &path, &error))?;
        return Ok(None);
    }
    Ok(Some(bytes.len() as u64))
}

/// Fetch one artifact, verify it, and write it into the mirror.
fn download(directory: &Path, artifact: &Pinned) -> Result<u64, Diagnostic> {
    let bytes = match get(&artifact.url) {
        Ok(bytes) => bytes,
        Err(message) => {
            return Err(Diagnostic::new(
                Severity::Error,
                "airlock.mirror.fetch.request",
                format!(
                    "cannot fetch {} from {}: {message}",
                    artifact.identity(),
                    artifact.url
                ),
            )
            .with_path(artifact.url.clone()));
        }
    };
    if let Some(size) = artifact.size
        && bytes.len() as u64 != size
    {
        return Err(Diagnostic::new(
            Severity::Error,
            "airlock.mirror.fetch.size",
            format!(
                "{} is {} bytes at {}, and the lock records {size}; the pins do not agree, so \
                 neither does this mirror",
                artifact.identity(),
                bytes.len(),
                artifact.url
            ),
        )
        .with_path(artifact.url.clone()));
    }
    let digest = sha256_hex(&bytes);
    if digest != artifact.sha256 {
        // The package is named rather than the URL alone: a mirror that fails without naming
        // what it could not verify leaves the operator reading digests by eye.
        return Err(Diagnostic::new(
            Severity::Error,
            "airlock.mirror.fetch.digest",
            format!(
                "{} arrives as sha256:{digest} from {}, and the lock pins sha256:{}",
                artifact.identity(),
                artifact.url,
                artifact.sha256
            ),
        )
        .with_path(artifact.url.clone()));
    }
    let path = directory.join(artifact.path());
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|error| io("airlock.mirror.mirror.io", parent, &error))?;
    }
    std::fs::write(&path, &bytes).map_err(|error| io("airlock.mirror.mirror.io", &path, &error))?;
    Ok(bytes.len() as u64)
}

/// One GET, with nothing but the URL.
///
/// The client is the channel crate's blocking one, built per run: a mirror fetches a lock's
/// worth of archives and then stops, so a pool has nothing to reuse it for.
fn get(url: &str) -> Result<Vec<u8>, String> {
    let client = Http::new().map_err(|error| error.to_string())?;
    let request = HttpRequest::new(Method::Get, url);
    let response = client.send(&request).map_err(|error| error.to_string())?;
    if !response.is_success() {
        return Err(format!("the server answered {}", response.status));
    }
    Ok(response.body)
}

/// Write the index the tree carries, and return its digest.
fn write_index(
    directory: &Path,
    adapter: &str,
    lock: &str,
    branch: &str,
    pinned: &[Pinned],
) -> Result<String, Diagnostic> {
    let mut artifacts: Vec<serde_json::Value> = Vec::with_capacity(pinned.len());
    for artifact in pinned {
        let path = directory.join(artifact.path());
        let size = std::fs::metadata(&path)
            .map_err(|error| io("airlock.mirror.mirror.io", &path, &error))?
            .len();
        artifacts.push(json!({
            "name": artifact.name,
            "version": artifact.version,
            "kind": artifact.kind.as_str(),
            "subdir": artifact.subdir,
            "url": artifact.url,
            "sha256": artifact.sha256,
            "size": size,
            "path": artifact.path(),
        }));
    }
    // No timestamp, no run id, no ordering by anything but the mirror path: two runs of one
    // lock have to produce one file, or a republish manufactures a commit that says nothing.
    let index = json!({
        "schema": INDEX_SCHEMA,
        "adapter": adapter,
        "branch": branch,
        "lock_sha256": sha256_hex(lock.as_bytes()),
        "artifacts": artifacts,
    });
    let text = format!(
        "{}\n",
        serde_json::to_string_pretty(&index).map_err(|error| {
            Diagnostic::new(
                Severity::Error,
                "airlock.mirror.index",
                format!("cannot write the mirror index: {error}"),
            )
        })?
    );
    let path = directory.join(INDEX);
    std::fs::write(&path, &text).map_err(|error| io("airlock.mirror.mirror.io", &path, &error))?;
    Ok(sha256_hex(text.as_bytes()))
}

/// Every file in the mirror directory, as the tree one commit would hold.
fn tree_of(directory: &Path) -> Result<ChannelTree, Diagnostic> {
    let mut tree = ChannelTree::default();
    collect(directory, directory, &mut tree)?;
    Ok(tree)
}

fn collect(root: &Path, directory: &Path, tree: &mut ChannelTree) -> Result<(), Diagnostic> {
    let entries = std::fs::read_dir(directory)
        .map_err(|error| io("airlock.mirror.mirror.io", directory, &error))?;
    let mut paths: Vec<PathBuf> = entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .collect();
    paths.sort();
    for path in paths {
        if path.is_dir() {
            collect(root, &path, tree)?;
            continue;
        }
        let relative = path.strip_prefix(root).unwrap_or(&path);
        // The tree is a Git tree, so its paths are slash-separated whatever the host's
        // separator is; the mirror only ever runs on a POSIX producer, but the conversion is
        // cheaper than the assumption.
        let name = relative
            .components()
            .map(|component| component.as_os_str().to_string_lossy().into_owned())
            .collect::<Vec<_>>()
            .join("/");
        let bytes =
            std::fs::read(&path).map_err(|error| io("airlock.mirror.mirror.io", &path, &error))?;
        tree.insert(name, bytes);
    }
    Ok(())
}

/// One filesystem failure, in the vocabulary the other findings use.
fn io(code: &str, path: &Path, error: &std::io::Error) -> Diagnostic {
    Diagnostic::new(
        Severity::Error,
        code,
        format!("cannot use {}: {error}", path.display()),
    )
    .with_path(path.display().to_string())
}

/// What a run reports, in whichever shape the caller asked for.
fn report(
    request: &Request,
    mirrored: &Mirrored,
    would_fetch: Option<usize>,
    staged: Option<&str>,
) -> CommandOutput {
    let commit = match &mirrored.outcome {
        Some(PushOutcome::Published { commit }) => Some(commit.clone()),
        Some(PushOutcome::Unchanged) => None,
        None => None,
    };
    let unchanged = matches!(mirrored.outcome, Some(PushOutcome::Unchanged));
    if request.json {
        return json_success(
            "airlock mirror",
            json!({
                "adapter": request.adapter,
                "branch": mirrored.branch,
                "artifacts": mirrored.artifacts,
                "fetched": mirrored.fetched,
                "reused": mirrored.reused,
                "would_fetch": would_fetch,
                "bytes": mirrored.bytes,
                "index": mirrored.index,
                "commit": commit,
                "unchanged": unchanged,
                "matched": mirrored.matched,
                "tree": staged,
                "dry_run": request.dry_run,
            }),
            Vec::new(),
            Vec::new(),
        );
    }
    if request.dry_run {
        return human(format!(
            "would mirror {} artifact(s) for {} into {}: {} to fetch, {} already cached\n",
            mirrored.artifacts,
            request.adapter,
            mirrored.branch,
            would_fetch.unwrap_or_default(),
            mirrored.reused
        ));
    }
    if let Some(matched) = mirrored.matched {
        return human(format!(
            "{} {} {}\n",
            mirrored.branch,
            if matched {
                "already holds what this lock pins"
            } else {
                "differs from what this lock would publish"
            },
            staged.map_or_else(String::new, |tree| format!("(tree {tree})"))
        ));
    }
    match &mirrored.outcome {
        Some(PushOutcome::Unchanged) => human(format!(
            "{} already holds {} artifact(s) for {}; nothing was published\n",
            mirrored.branch, mirrored.artifacts, request.adapter
        )),
        _ => human(format!(
            "mirrored {} artifact(s) for {} into {}: {} fetched, {} reused ({} bytes, \
             sha256:{}){}\n",
            mirrored.artifacts,
            request.adapter,
            mirrored.branch,
            mirrored.fetched,
            mirrored.reused,
            mirrored.bytes,
            mirrored.index,
            commit.map_or_else(String::new, |commit| format!(", commit {commit}"))
        )),
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::{INDEX, cached, io, tree_of, write_index};
    use crate::lock::{Locked, Pinned};

    const DIGEST: &str = "9f2c1d0e8b7a6c5d4e3f2a1b0c9d8e7f6a5b4c3d2e1f0a9b8c7d6e5f4a3b2c1d";

    fn artifact() -> Pinned {
        Pinned {
            name: "libgcc".to_owned(),
            version: "16.2.0".to_owned(),
            kind: Locked::Conda,
            subdir: "linux-64".to_owned(),
            url: "https://example.invalid/linux-64/libgcc-16.2.0-h1234567_0.conda".to_owned(),
            sha256: DIGEST.to_owned(),
            size: Some(4),
        }
    }

    #[test]
    fn a_mirror_that_does_not_hold_an_artifact_says_so() {
        let directory = tempfile::tempdir().expect("a directory");
        assert_eq!(
            cached(directory.path(), &artifact()).expect("no cached file"),
            None
        );
    }

    #[test]
    fn a_cached_artifact_is_measured() {
        let directory = tempfile::tempdir().expect("a directory");
        let path = directory.path().join(artifact().path());
        std::fs::create_dir_all(path.parent().expect("a parent")).expect("the blob directory");
        std::fs::write(&path, b"four").expect("the blob");
        // The fixture's digest is not of these bytes, so this is the mismatch path: the file is
        // removed, and the caller is told there is nothing cached rather than being handed
        // bytes that fail their own checksum.
        assert_eq!(cached(directory.path(), &artifact()).expect("cached"), None);
        assert!(!path.exists(), "a corrupt cached blob was left in place");
    }

    #[test]
    fn a_cached_artifact_that_matches_is_reused() {
        let directory = tempfile::tempdir().expect("a directory");
        let bytes = b"four";
        let mut artifact = artifact();
        artifact.sha256 = super::sha256_hex(bytes);
        let path = directory.path().join(artifact.path());
        std::fs::create_dir_all(path.parent().expect("a parent")).expect("the blob directory");
        std::fs::write(&path, bytes).expect("the blob");
        assert_eq!(
            cached(directory.path(), &artifact).expect("cached"),
            Some(4)
        );
    }

    #[test]
    fn the_tree_holds_every_file_under_its_slash_separated_path() {
        let directory = tempfile::tempdir().expect("a directory");
        std::fs::create_dir_all(directory.path().join("blobs/sha256/9f")).expect("nested");
        std::fs::write(directory.path().join("blobs/sha256/9f/blob"), b"x").expect("a blob");
        std::fs::write(directory.path().join(INDEX), b"{}\n").expect("the index");
        let tree = tree_of(directory.path()).expect("a tree");
        let mut paths: Vec<&str> = tree.paths().map(String::as_str).collect();
        paths.sort_unstable();
        assert_eq!(paths, vec!["blobs/sha256/9f/blob", INDEX]);
    }

    #[test]
    fn the_index_is_the_same_text_for_the_same_lock() {
        let first = tempfile::tempdir().expect("a directory");
        let second = tempfile::tempdir().expect("a directory");
        for directory in [&first, &second] {
            let path = directory.path().join(artifact().path());
            std::fs::create_dir_all(path.parent().expect("a parent")).expect("the blob directory");
            std::fs::write(&path, b"four").expect("the blob");
        }
        let pinned = vec![artifact()];
        let left = write_index(first.path(), "pixi", "version: 7\n", "mirror/pixi", &pinned)
            .expect("an index");
        let right = write_index(
            second.path(),
            "pixi",
            "version: 7\n",
            "mirror/pixi",
            &pinned,
        )
        .expect("an index");
        assert_eq!(
            left, right,
            "the index digest is not a function of the lock"
        );
        let text = std::fs::read_to_string(first.path().join(INDEX)).expect("the index");
        assert!(!text.contains("timestamp"), "the index carries a timestamp");
        let parsed: serde_json::Value = serde_json::from_str(&text).expect("JSON");
        assert_eq!(
            parsed["artifacts"][0]["size"], 4,
            "the size is measured, not recorded"
        );
        assert_eq!(parsed["lock_sha256"], super::sha256_hex(b"version: 7\n"));
    }

    #[test]
    fn a_filesystem_failure_names_the_path_it_could_not_use() {
        let error = std::io::Error::other("no space left");
        let diagnostic = io(
            "airlock.mirror.mirror.io",
            Path::new("/a/mirror/blob"),
            &error,
        );
        assert_eq!(diagnostic.code(), "airlock.mirror.mirror.io");
        assert!(diagnostic.message().contains("/a/mirror/blob"));
        assert!(diagnostic.message().contains("no space left"));
    }

    #[test]
    fn a_lock_this_build_cannot_read_is_reported_before_anything_is_fetched() {
        let root = tempfile::tempdir().expect("a project");
        std::fs::write(root.path().join("pixi.lock"), "{ not: [ yaml").expect("the lock");
        let manifest = universe_core::manifest::Manifest::from_toml(
            "schema = 1\n\n[project]\nname = \"fixture-mirror\"\nplatforms = [\"linux-64\"]\n",
        )
        .expect("a manifest");
        let output = super::run(
            &super::Request {
                root: root.path().to_path_buf(),
                adapter: "pixi".to_owned(),
                lock: None,
                mirror: None,
                crates: None,
                remote: None,
                branch: None,
                check: false,
                dry_run: false,
                json: false,
            },
            &manifest,
        );
        assert_eq!(output.exit_code(), 1);
        assert!(output.stderr().contains("airlock.mirror.lock.parse"));
        assert!(output.stdout().is_empty());
    }
}
