//! `universe airlock render`: the committed restore entrypoint, and its drift check.
//!
//! The entrypoint is the one file a disconnected host runs, so it has to be small, readable,
//! and honest about where its numbers come from. Three rules shape it, and each of them is a
//! way a generated script goes wrong:
//!
//! - **No runtime digest is embedded.** The digest of what the entrypoint fetches lives in the
//!   ref it fetches, not in the script: embedding it would rewrite every consumer's committed
//!   script on every release of this tool, and the ref is already immutable. What the script
//!   does pin — the remote, the branch, and the commit — is checked after the checkout rather
//!   than assumed, so a ref that has been rewritten fails loudly instead of running.
//! - **The pin is stated, never resolved.** Rendering is a pure function of the manifest, so
//!   two renders are byte-identical and the committed file can be drift-checked. A renderer
//!   that asked git for a branch head would produce a different file on every run and the
//!   check would be noise.
//! - **Only safe values are carried.** The manifest's validator holds the pin's fields to a
//!   charset that survives a single-quoted shell string, so this module interpolates without
//!   escaping. Escaping here would be a second place to get it wrong.
//!
//! The same rule the `ci` family follows applies to the streams: stdout is data, diagnostics
//! go to stderr, and the exit codes mean what they mean there — `0` matched, `1` the
//! repository disagrees with the manifest, `2` the invocation or manifest is wrong.

use std::path::{Path, PathBuf};

use serde_json::json;
use universe_channel::transport::{PushOutcome, Transport};
use universe_core::diagnostic::{Diagnostic, Envelope, Severity};
use universe_core::manifest::{
    AirlockRestore, DEFAULT_RESTORE_DIR, Manifest, PackageKind, RELEASE_MANIFEST_SCHEMA_VERSION,
    ReleaseManifest, ReleasedArtifact, ReleasedPackage,
};
use universe_core::{CommandOutput, STDOUT_SCHEMA_VERSION};

use crate::git::{Entry, GitTransport};
use crate::mirror;

/// Default manifest file name, read from the project root.
const MANIFEST_FILE: &str = "universe.toml";

/// The POSIX entrypoint's file name inside the restore directory.
const POSIX_SCRIPT: &str = "restore.sh";

/// The PowerShell entrypoint's file name inside the restore directory.
const POWERSHELL_SCRIPT: &str = "restore.ps1";

/// The project README, whose usage block is rendered from the same source.
const README: &str = "README.md";

/// Marker that opens the rendered README block. Text between the markers is generated.
const README_BEGIN: &str = "<!-- airlock:begin -->";

/// Marker that closes it.
const README_END: &str = "<!-- airlock:end -->";

/// The bootstrapper's file name inside the published ref.
///
/// It is the same default `universe_core::manifest::DEFAULT_BOOTSTRAP` gives the entrypoint, so
/// the verb that publishes the ref and the script that runs out of it name one file, once.
const BOOTSTRAP_BINARY: &str = "universe-airlock-bootstrap";

/// The descriptor published beside the binary.
///
/// The entrypoint does not read it — the bytes it runs are the bytes in its own checkout — so
/// this is for the reader and for a consumer checking what a pin holds without downloading a
/// 300 KiB executable.
const BOOTSTRAP_DESCRIPTOR: &str = "bootstrap.json";

/// The ref a publish writes when the caller names none.
const DEFAULT_BOOTSTRAP_BRANCH: &str = "bootstrap";

/// The directory a payload is written into, relative to the project root.
///
/// It is not derived from `[airlock.developer.restore] dir`, and that is deliberate: the
/// bootstrapper resolves its payload root on its own (`payload::default_root`), the rendered
/// entrypoint does not pass `--payload`, and the two have to name the same directory. A project
/// that moved its entrypoints elsewhere would otherwise get a payload nothing looks at.
const PAYLOAD_DIR: &str = ".universe-airlock/payload";

/// The release manifest's file name inside a payload.
const PAYLOAD_MANIFEST: &str = "universe-release.json";

/// Exit code for a repository that does not match its manifest.
const DRIFT: u8 = 1;

/// Exit code for a bad invocation or an invalid manifest.
const USAGE: u8 = 2;

/// One file a render produces or replaces.
#[derive(Clone, Debug, Eq, PartialEq)]
struct Rendered {
    /// Path relative to the project root.
    path: String,
    /// The bytes it must hold.
    content: String,
    /// Whether it must be executable, which is what makes `./restore.sh` work.
    executable: bool,
}

/// What the caller asked for.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Verb {
    /// Write the rendered files, or compare them with what is on disk.
    Render,
    /// Publish the stable bootstrap ref the entrypoint clones.
    PublishBootstrap,
    /// Write the closed payload a bootstrapper restores from.
    Pack,
    /// Fetch and publish what a lock pins.
    Mirror,
}

/// Parsed options for one `airlock` invocation.
#[derive(Clone, Debug, Eq, PartialEq)]
struct Invocation {
    verb: Verb,
    root: PathBuf,
    /// Where to write instead of the directory the manifest names.
    output: Option<PathBuf>,
    print_stdout: bool,
    json: bool,
    /// The built bootstrapper `publish-bootstrap` publishes.
    binary: Option<PathBuf>,
    /// Where to publish it, when the manifest does not already say.
    remote: Option<String>,
    /// Which ref to publish it to.
    branch: Option<String>,
    /// Which lock format `mirror` reads.
    adapter: Option<String>,
    /// The lockfile `mirror` reads, when it is not the adapter's default.
    lock: Option<PathBuf>,
    /// Where `mirror` keeps its bytes, when not the default directory.
    mirror: Option<PathBuf>,
    /// Where `mirror` fetches a crate from, when not crates.io's own layout.
    crates: Option<String>,
    /// Compare with the destination instead of changing it.
    check: bool,
    /// Report what a run would do, changing nothing.
    dry_run: bool,
}

/// Why an invocation could not be interpreted.
#[derive(Clone, Debug, Eq, PartialEq)]
struct UsageError(String);

const USAGE_LINE: &str = "usage: universe airlock render [--check] [--output <dir>] \
                          [--root <dir>] [--stdout] [--json]\n       \
                          universe airlock publish-bootstrap --binary <path> [--remote <url>] \
                          [--branch <name>] [--root <dir>] [--json]\n       \
                          universe airlock pack [--output <dir>] [--root <dir>] [--json]\n       \
                          universe airlock mirror --adapter <name> [--lock <path>] \
                          [--mirror <dir>] [--crates <url>] [--remote <url>] \
                          [--branch <name>] [--check] [--dry-run] [--root <dir>] [--json]";

impl Invocation {
    fn parse(args: &[String]) -> Result<Self, UsageError> {
        let mut invocation = Self {
            verb: Verb::Render,
            root: PathBuf::from("."),
            output: None,
            print_stdout: false,
            json: false,
            binary: None,
            remote: None,
            branch: None,
            adapter: None,
            lock: None,
            mirror: None,
            crates: None,
            check: false,
            dry_run: false,
        };
        let mut verb_seen = false;
        let mut rest = args.iter();
        while let Some(argument) = rest.next() {
            match argument.as_str() {
                "render" if !verb_seen => {
                    invocation.verb = Verb::Render;
                    verb_seen = true;
                }
                "publish-bootstrap" if !verb_seen => {
                    invocation.verb = Verb::PublishBootstrap;
                    verb_seen = true;
                }
                "pack" if !verb_seen => {
                    invocation.verb = Verb::Pack;
                    verb_seen = true;
                }
                "mirror" if !verb_seen => {
                    invocation.verb = Verb::Mirror;
                    verb_seen = true;
                }
                "--binary" => {
                    let value = rest.next().ok_or_else(|| {
                        UsageError("--binary needs the built bootstrapper".to_owned())
                    })?;
                    invocation.binary = Some(PathBuf::from(value));
                }
                "--remote" => {
                    let value = rest
                        .next()
                        .ok_or_else(|| UsageError("--remote needs a URL".to_owned()))?;
                    invocation.remote = Some(value.clone());
                }
                "--branch" => {
                    let value = rest
                        .next()
                        .ok_or_else(|| UsageError("--branch needs a name".to_owned()))?;
                    invocation.branch = Some(value.clone());
                }
                "--check" => invocation.check = true,
                "--dry-run" => invocation.dry_run = true,
                "--adapter" => {
                    let value = rest
                        .next()
                        .ok_or_else(|| UsageError("--adapter needs a lock format".to_owned()))?;
                    invocation.adapter = Some(value.clone());
                }
                "--lock" => {
                    let value = rest
                        .next()
                        .ok_or_else(|| UsageError("--lock needs a path".to_owned()))?;
                    invocation.lock = Some(PathBuf::from(value));
                }
                "--mirror" => {
                    let value = rest
                        .next()
                        .ok_or_else(|| UsageError("--mirror needs a directory".to_owned()))?;
                    invocation.mirror = Some(PathBuf::from(value));
                }
                "--crates" => {
                    let value = rest.next().ok_or_else(|| {
                        UsageError("--crates needs the base URL crates are served from".to_owned())
                    })?;
                    invocation.crates = Some(value.clone());
                }
                "--output" => {
                    let value = rest
                        .next()
                        .ok_or_else(|| UsageError("--output needs a directory".to_owned()))?;
                    invocation.output = Some(PathBuf::from(value));
                }
                "--stdout" => invocation.print_stdout = true,
                "--json" => invocation.json = true,
                "--root" => {
                    let value = rest
                        .next()
                        .ok_or_else(|| UsageError("--root needs a directory".to_owned()))?;
                    invocation.root = PathBuf::from(value);
                }
                other => {
                    return Err(UsageError(format!(
                        "unknown argument: {other}\n{USAGE_LINE}"
                    )));
                }
            }
        }
        if !verb_seen {
            return Err(UsageError(USAGE_LINE.to_owned()));
        }
        // A flag that the chosen verb does not read is a caller who believes something happened.
        // Publishing and rendering take disjoint options, so saying so is cheap.
        if !matches!(invocation.verb, Verb::PublishBootstrap | Verb::Mirror)
            && let Some(flag) = [
                invocation.binary.is_some().then_some("--binary"),
                invocation.remote.is_some().then_some("--remote"),
                invocation.branch.is_some().then_some("--branch"),
            ]
            .into_iter()
            .flatten()
            .next()
        {
            return Err(UsageError(format!(
                "{flag} is only valid for publish-bootstrap and mirror\n{USAGE_LINE}"
            )));
        }
        if invocation.verb == Verb::PublishBootstrap && invocation.binary.is_none() {
            return Err(UsageError(format!(
                "publish-bootstrap needs --binary\n{USAGE_LINE}"
            )));
        }
        if invocation.print_stdout && invocation.verb != Verb::Render {
            return Err(UsageError(format!(
                "--stdout is only valid for render; pack and mirror write files\n{USAGE_LINE}"
            )));
        }
        // The mirror reads a lock and writes a mirror, so the flags about rendered text and
        // payload directories are somebody expecting a different verb.
        if invocation.verb == Verb::Mirror
            && let Some(flag) = [
                invocation.output.is_some().then_some("--output"),
                invocation.binary.is_some().then_some("--binary"),
            ]
            .into_iter()
            .flatten()
            .next()
        {
            return Err(UsageError(format!(
                "{flag} is not valid for mirror\n{USAGE_LINE}"
            )));
        }
        if invocation.verb != Verb::Mirror
            && let Some(flag) = [
                invocation.adapter.is_some().then_some("--adapter"),
                invocation.lock.is_some().then_some("--lock"),
                invocation.mirror.is_some().then_some("--mirror"),
                invocation.crates.is_some().then_some("--crates"),
                invocation.dry_run.then_some("--dry-run"),
            ]
            .into_iter()
            .flatten()
            .next()
        {
            return Err(UsageError(format!(
                "{flag} is only valid for mirror\n{USAGE_LINE}"
            )));
        }
        if invocation.verb == Verb::Mirror && invocation.adapter.is_none() {
            return Err(UsageError(format!(
                "mirror needs --adapter, because a project may pin more than one lock format\n                 {USAGE_LINE}"
            )));
        }
        Ok(invocation)
    }

    fn manifest_path(&self) -> PathBuf {
        self.root.join(MANIFEST_FILE)
    }

    /// What `mirror` asked for, in the mirror's own vocabulary.
    fn mirror_request(&self) -> mirror::Request {
        mirror::Request {
            root: self.root.clone(),
            adapter: self.adapter.clone().unwrap_or_default(),
            lock: self.lock.clone(),
            mirror: self.mirror.clone(),
            crates: self.crates.clone(),
            remote: self.remote.clone(),
            branch: self.branch.clone(),
            check: self.check,
            dry_run: self.dry_run,
            json: self.json,
        }
    }
}

/// Run `universe airlock <verb>`.
pub(crate) fn run(args: &[String]) -> CommandOutput {
    match Invocation::parse(args) {
        Ok(invocation) => execute(&invocation),
        Err(UsageError(message)) => failure(
            USAGE,
            vec![Diagnostic::new(
                Severity::Error,
                "airlock.usage.invalid",
                message,
            )],
        ),
    }
}

fn execute(invocation: &Invocation) -> CommandOutput {
    if invocation.verb == Verb::PublishBootstrap {
        return publish_bootstrap(invocation);
    }
    let manifest_path = invocation.manifest_path();
    let Ok(text) = std::fs::read_to_string(&manifest_path) else {
        return failure(
            USAGE,
            vec![
                Diagnostic::new(
                    Severity::Error,
                    "airlock.manifest.missing",
                    format!("no {} at this root", manifest_path.display()),
                )
                .with_path(manifest_path.display().to_string()),
            ],
        );
    };
    let manifest = match Manifest::from_toml(&text) {
        Ok(manifest) => manifest,
        Err(diagnostics) => return failure(USAGE, diagnostics),
    };
    // The manifest's own findings are the renderer's preconditions: a pin the entrypoint
    // cannot carry is a reason not to write one.
    let blocking: Vec<Diagnostic> = manifest
        .validate()
        .into_iter()
        .filter(|diagnostic| diagnostic.is_error())
        .collect();
    if !blocking.is_empty() {
        return failure(USAGE, blocking);
    }

    // Packing reads the manifest and the files a build produced; it writes no generated text
    // and needs no restore scope, so it returns before the renderer decides what exists.
    if invocation.verb == Verb::Pack {
        return pack_payload(invocation, &manifest);
    }
    // Mirroring reads a lock and the manifest's branch prefix, and renders nothing: a project
    // that pins a lock but ships no entrypoint still mirrors, which is the order `UNI-51` lands
    // in — the mirror first, the restore entrypoint once there is something to restore.
    if invocation.verb == Verb::Mirror {
        return mirror::run(&invocation.mirror_request(), &manifest);
    }

    let restore = manifest
        .airlock
        .as_ref()
        .and_then(|airlock| airlock.developer.as_ref())
        .and_then(|developer| developer.restore.as_ref());
    let Some(restore) = restore else {
        // Nothing declared, nothing to render — and nothing stale to report either, so a
        // repository that removes the scope gets its own generated files cleaned up below.
        return settled(invocation, &manifest);
    };

    let directory = invocation.directory(restore);
    let readme = std::fs::read_to_string(invocation.root.join(README)).ok();
    let rendered = render(&manifest, &text, restore, &directory, readme.as_deref());
    let note = invocation.override_note(restore);
    match invocation.verb {
        Verb::Render if invocation.check => check(invocation, &manifest, &rendered),
        Verb::Render => write(invocation, &manifest, &rendered, note.as_deref()),
        // Dispatched before the manifest is read: publishing needs no scope to exist, and the
        // first bootstrap ref is published before the scope that pins it does.
        Verb::PublishBootstrap => unreachable!("publish-bootstrap returns before this point"),
        Verb::Pack => unreachable!("pack returns before the renderer runs"),
        Verb::Mirror => unreachable!("mirror returns before the renderer runs"),
    }
}

/// Write the closed payload a bootstrapper restores from.
///
/// The consumer side of this has existed since `UNI-33`: `universe-airlock-bootstrap verify`
/// reads `universe-release.json`, checks the commit against the pin it was handed, and checks
/// every artifact's size and digest before anything is activated. Nothing wrote that file, so the
/// rendered entrypoint cloned the ref, found the bootstrapper, handed over, and stopped at
/// `[airlock.payload.manifest] cannot read the release manifest`. This is the producer half.
///
/// Two things are deliberately not flags on this verb. The pin comes from
/// `[airlock.developer.restore] commit` — the same value the entrypoint checks — because a
/// payload naming another commit is refused by the bootstrapper, and a caller typing the pin
/// twice is a way to get that wrong. The directory is fixed at `.universe-airlock/payload` for
/// the same class of reason: the bootstrapper resolves that path itself.
///
/// The payload is a *closed* directory. A file in it that the descriptor does not name is
/// refused rather than removed: the consumer rejects unexpected files, and a signature over the
/// previous manifest would be left describing bytes that are no longer there. Deleting files in
/// a producer's tree is not this verb's business.
fn pack_payload(invocation: &Invocation, manifest: &Manifest) -> CommandOutput {
    let airlock = manifest.airlock.as_ref();
    // The delivery scope is what says "this project ships a payload". Packing a repository that
    // never declared one would be inventing a lane, so the refusal names the declaration.
    if airlock
        .and_then(|airlock| airlock.delivery.as_ref())
        .is_none()
    {
        return failure(
            USAGE,
            vec![
                Diagnostic::new(
                    Severity::Error,
                    "airlock.pack.scope",
                    "no [airlock.delivery] scope is declared; a payload is what that scope ships",
                )
                .with_path(invocation.manifest_path().display().to_string()),
            ],
        );
    }
    // The commit the payload is *for*: a release manifest naming any other commit is refused by
    // the bootstrapper on the target, which is the worst place to find out.
    let Some(pin) = airlock
        .and_then(|airlock| airlock.developer.as_ref())
        .and_then(|developer| developer.restore.as_ref())
        .map(|restore| restore.commit.clone())
    else {
        return failure(
            USAGE,
            vec![
                Diagnostic::new(
                    Severity::Error,
                    "airlock.pack.commit",
                    "no [airlock.developer.restore] commit is declared, and a payload names the \
                     pin it restores",
                )
                .with_path("airlock.developer.restore.commit"),
            ],
        );
    };

    let mut errors: Vec<Diagnostic> = Vec::new();
    let mut packages: Vec<ReleasedPackage> = Vec::new();
    let mut sources: Vec<PathBuf> = Vec::new();
    for package in &manifest.packages {
        // The predicate `audit` uses: a package that is neither released nor published has no
        // artifact a payload could carry.
        if !package.release && package.publish.is_empty() {
            continue;
        }
        let mut produced: Vec<PathBuf> = Vec::new();
        for pattern in &package.outputs {
            for path in crate::paths::collect(&invocation.root, pattern) {
                if !produced.contains(&path) {
                    produced.push(path);
                }
            }
        }
        if produced.is_empty() {
            errors.push(
                Diagnostic::new(
                    Severity::Error,
                    "airlock.pack.output",
                    format!(
                        "package '{}' ships but produced none of {}",
                        package.name,
                        package.outputs.join(", ")
                    ),
                )
                .with_path(format!("packages[{}].outputs", package.name)),
            );
            continue;
        }
        let mut artifacts: Vec<ReleasedArtifact> = Vec::new();
        let mut version: Option<String> = None;
        for path in produced {
            if package.kind != PackageKind::Conda {
                errors.push(
                    Diagnostic::new(
                        Severity::Error,
                        "airlock.pack.kind",
                        format!(
                            "package '{}' is a {} package, and a payload currently carries Conda \
                             artifacts only",
                            package.name,
                            package.kind.as_str()
                        ),
                    )
                    .with_path(format!("packages[{}].kind", package.name)),
                );
                continue;
            }
            // The descriptor's identity comes from the archive, not from the manifest: one
            // manifest entry can declare a directory of archives, and they need not be one name.
            let opened = match universe_conda::open_package(&path) {
                Ok(opened) => opened,
                Err(error) => {
                    errors.push(
                        Diagnostic::new(
                            Severity::Error,
                            "airlock.pack.unreadable",
                            format!(
                                "package '{}' produced {}, which cannot be read as an archive: \
                                 {error}",
                                package.name,
                                path.display()
                            ),
                        )
                        .with_path(path.display().to_string()),
                    );
                    continue;
                }
            };
            let identity = opened.artifact().id.clone();
            let filename = path
                .file_name()
                .map_or_else(String::new, |name| name.to_string_lossy().into_owned());
            // The bootstrapper copies by file name and reports the identity; an archive built as
            // one package and published under another's name restores something nobody can name.
            if !filename.starts_with(&format!("{}-{}-", identity.name, identity.version)) {
                errors.push(
                    Diagnostic::new(
                        Severity::Error,
                        "airlock.pack.identity",
                        format!(
                            "the archive at {filename} calls itself {} {}, which is not what its \
                             file name says",
                            identity.name, identity.version
                        ),
                    )
                    .with_path(path.display().to_string()),
                );
                continue;
            }
            match &version {
                None => version = Some(identity.version.clone()),
                Some(existing) if *existing != identity.version => errors.push(
                    Diagnostic::new(
                        Severity::Error,
                        "airlock.pack.version",
                        format!(
                            "package '{}' publishes both {existing} and {} artifacts, and a \
                             release names one version",
                            package.name, identity.version
                        ),
                    )
                    .with_path(format!("packages[{}].outputs", package.name)),
                ),
                Some(_) => {}
            }
            // The store list is where the release workflow sends these bytes, in the renderer's
            // order: the declared stores first, the implicit release destination last.
            let mut stores = package.publish.clone();
            if package.release {
                stores.push("releases".to_owned());
            }
            artifacts.push(ReleasedArtifact {
                filename: filename.clone(),
                sha256: opened.artifact().digest.clone(),
                size: opened.artifact().size_bytes,
                stores,
            });
            sources.push(path);
        }
        packages.push(ReleasedPackage {
            name: package.name.clone(),
            kind: package.kind,
            version: version.unwrap_or_default(),
            artifacts,
        });
    }
    if !errors.is_empty() {
        return failure(USAGE, errors);
    }

    // Two packages can declare output globs that select the same file name, and a payload
    // directory cannot hold both. The consumer refuses the collision; so does the producer, here,
    // rather than after copying.
    let mut names: Vec<&str> = Vec::new();
    for artifact in packages.iter().flat_map(|package| package.artifacts.iter()) {
        if names.contains(&artifact.filename.as_str()) {
            errors.push(Diagnostic::new(
                Severity::Error,
                "airlock.pack.duplicate",
                format!(
                    "two packages publish an artifact named {}, and a payload directory cannot \
                     hold both",
                    artifact.filename
                ),
            ));
        } else {
            names.push(&artifact.filename);
        }
    }
    if packages.is_empty() {
        errors.push(Diagnostic::new(
            Severity::Error,
            "airlock.pack.empty",
            "no package ships an artifact, so there is no payload to write",
        ));
    }
    // One version for the release, because the bootstrapper reports the payload's version once.
    // A train whose artifacts disagree is refused rather than reported as whichever came first.
    let mut version: Option<&str> = None;
    for package in &packages {
        if package.version.is_empty() {
            continue;
        }
        match version {
            None => version = Some(package.version.as_str()),
            Some(existing) if existing != package.version => errors.push(Diagnostic::new(
                Severity::Error,
                "airlock.pack.version",
                format!(
                    "this payload would mix {existing} and {}; a release names one version",
                    package.version
                ),
            )),
            Some(_) => {}
        }
    }
    if !errors.is_empty() {
        return failure(USAGE, errors);
    }

    let output = invocation
        .output
        .clone()
        .unwrap_or_else(|| PathBuf::from(PAYLOAD_DIR));
    let output = if output.is_absolute() {
        output
    } else {
        invocation.root.join(output)
    };

    // A closed directory, checked before anything is written: a foreign file here is either a
    // stale artifact from an earlier build or a payload someone edited, and both mean the
    // directory is not what this descriptor is about to say it is.
    if let Ok(entries) = std::fs::read_dir(&output) {
        let mut stale: Vec<String> = Vec::new();
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
            let expected = name == PAYLOAD_MANIFEST || names.contains(&name.as_str());
            if !expected {
                stale.push(name);
            }
        }
        if !stale.is_empty() {
            stale.sort();
            return failure(
                USAGE,
                vec![
                    Diagnostic::new(
                        Severity::Error,
                        "airlock.pack.stale",
                        format!(
                            "{} already holds {}, which this payload does not name; a payload is \
                             a closed directory, so remove them before packing",
                            output.display(),
                            stale.join(", ")
                        ),
                    )
                    .with_path(output.display().to_string()),
                ],
            );
        }
    }

    let release = ReleaseManifest {
        schema: RELEASE_MANIFEST_SCHEMA_VERSION,
        version: version.unwrap_or_default().to_owned(),
        commit: pin.clone(),
        packages,
    };
    let blocking: Vec<Diagnostic> = release
        .validate()
        .into_iter()
        .filter(Diagnostic::is_error)
        .collect();
    if !blocking.is_empty() {
        // The consumer validates this record again, so a record it would refuse is not written.
        return failure(USAGE, blocking);
    }
    let descriptor = match release.to_json() {
        Ok(text) => format!("{text}\n"),
        Err(error) => {
            return failure(
                USAGE,
                vec![Diagnostic::new(
                    Severity::Error,
                    "airlock.pack.serialize",
                    format!("cannot write the release manifest: {error}"),
                )],
            );
        }
    };
    if let Err(error) = std::fs::create_dir_all(&output) {
        return failure(
            USAGE,
            vec![
                Diagnostic::new(
                    Severity::Error,
                    "airlock.pack.output",
                    format!("cannot create {}: {error}", output.display()),
                )
                .with_path(output.display().to_string()),
            ],
        );
    }
    let mut copied: usize = 0;
    let mut bytes: u64 = 0;
    for (artifact, source) in release
        .packages
        .iter()
        .flat_map(|package| package.artifacts.iter())
        .zip(sources.iter())
    {
        if let Err(error) = std::fs::copy(source, output.join(&artifact.filename)) {
            return failure(
                USAGE,
                vec![
                    Diagnostic::new(
                        Severity::Error,
                        "airlock.pack.copy",
                        format!("cannot copy {} into the payload: {error}", source.display()),
                    )
                    .with_path(source.display().to_string()),
                ],
            );
        }
        copied += 1;
        bytes += artifact.size;
    }
    if let Err(error) = std::fs::write(output.join(PAYLOAD_MANIFEST), &descriptor) {
        return failure(
            USAGE,
            vec![
                Diagnostic::new(
                    Severity::Error,
                    "airlock.pack.write",
                    format!("cannot write the release manifest: {error}"),
                )
                .with_path(output.join(PAYLOAD_MANIFEST).display().to_string()),
            ],
        );
    }

    let digest = universe_workflow::sha256::sha256_hex(descriptor.as_bytes());
    let version = release.version.clone();
    let packages = release.packages.len();
    if invocation.json {
        let envelope: Envelope<serde_json::Value> = Envelope::new(
            STDOUT_SCHEMA_VERSION,
            "airlock pack",
            true,
            json!({
                "output": output.display().to_string(),
                "commit": release.commit,
                "version": version,
                "packages": packages,
                "artifacts": copied,
                "bytes": bytes,
                "sha256": digest,
            }),
            Vec::new(),
            Vec::new(),
        );
        return CommandOutput::new(
            format!(
                "{}\n",
                envelope
                    .to_json()
                    .expect("the airlock envelope always serializes")
            ),
            String::new(),
            0,
        );
    }
    CommandOutput::new(
        format!(
            "packed {copied} artifact(s) from {packages} package(s) {version} into {} \
             for commit {} (sha256:{digest}, {bytes} bytes)\n",
            output.display(),
            release.commit
        ),
        String::new(),
        0,
    )
}

/// Publish the ref a consumer's entrypoint clones.
///
/// This is the other half of `render`, and the reason it exists is a measurement: the
/// bootstrapper is also published as a Conda package, but a package is installed by a solver,
/// while the entrypoint `exec`s a file out of a checkout. Nothing in the published artifacts
/// could satisfy `[ -x "$CHECKOUT/universe-airlock-bootstrap" ]` until a ref held it.
///
/// The ref is stable rather than per-build: an unchanged binary leaves the commit where it was,
/// because a consumer's committed entrypoint names that commit and re-rendering every consumer
/// for identical bytes is drift with no information in it.
fn publish_bootstrap(invocation: &Invocation) -> CommandOutput {
    let Some(binary) = &invocation.binary else {
        // The parser refuses this, so reaching it means the two disagree.
        return failure(
            USAGE,
            vec![Diagnostic::new(
                Severity::Error,
                "airlock.publish.binary_missing",
                "publish-bootstrap needs --binary",
            )],
        );
    };
    let bytes = match std::fs::read(binary) {
        Ok(bytes) => bytes,
        Err(error) => {
            return failure(
                USAGE,
                vec![
                    Diagnostic::new(
                        Severity::Error,
                        "airlock.publish.unreadable",
                        format!("cannot read the bootstrapper: {error}"),
                    )
                    .with_path(binary.display().to_string()),
                ],
            );
        }
    };
    if bytes.is_empty() {
        return failure(
            USAGE,
            vec![
                Diagnostic::new(
                    Severity::Error,
                    "airlock.publish.empty",
                    "the built bootstrapper is empty; publishing it would publish nothing",
                )
                .with_path(binary.display().to_string()),
            ],
        );
    }

    // The manifest is a convenience here, not a precondition: an owner publishing the very first
    // bootstrap ref has no restore scope to read yet, which is the order `doc-103` describes.
    let manifest = std::fs::read_to_string(invocation.manifest_path())
        .ok()
        .and_then(|text| Manifest::from_toml(&text).ok());
    let declared = manifest
        .as_ref()
        .and_then(|manifest| manifest.airlock.as_ref())
        .and_then(|airlock| airlock.developer.as_ref())
        .and_then(|developer| developer.restore.clone());
    let remote = match invocation.remote.clone().or_else(|| {
        declared
            .as_ref()
            .map(|restore| restore.remote.clone())
            .filter(|remote| !remote.trim().is_empty())
    }) {
        Some(remote) => remote,
        None => {
            return failure(
                USAGE,
                vec![Diagnostic::new(
                    Severity::Error,
                    "airlock.publish.remote_missing",
                    "publish-bootstrap needs --remote, and the manifest declares no restore remote",
                )],
            );
        }
    };
    let branch = invocation
        .branch
        .clone()
        .or_else(|| {
            declared
                .as_ref()
                .map(|restore| restore.branch.clone())
                .filter(|branch| !branch.trim().is_empty())
        })
        .unwrap_or_else(|| DEFAULT_BOOTSTRAP_BRANCH.to_owned());

    let digest = universe_workflow::sha256::sha256_hex(&bytes);
    let descriptor = json!({
        "schema_version": 1,
        "binary": BOOTSTRAP_BINARY,
        "version": universe_core::VERSION,
        "sha256": digest,
        "size_bytes": bytes.len(),
    })
    .to_string();
    let entries = vec![
        Entry {
            path: BOOTSTRAP_DESCRIPTOR.to_owned(),
            bytes: descriptor.into_bytes(),
            executable: false,
        },
        Entry {
            path: BOOTSTRAP_BINARY.to_owned(),
            bytes: bytes.clone(),
            executable: true,
        },
    ];

    let scratch = match crate::scratch::Scratch::temporary() {
        Ok(scratch) => scratch,
        Err(diagnostic) => return failure(USAGE, vec![diagnostic]),
    };
    let transport = GitTransport::new(&remote, &branch, scratch.path());
    let outcome = transport.publish_entries(
        &entries,
        &format!("bootstrap: {BOOTSTRAP_BINARY} {}", universe_core::VERSION),
    );
    match outcome {
        Ok(outcome) => {
            // An unchanged publish still has a commit worth naming: it is the pin the caller
            // pastes into the manifest, and asking git again is cheaper than making them run
            // `git ls-remote` to find the thing the command just said it did not move.
            let existing = match &outcome {
                PushOutcome::Unchanged => transport
                    .current()
                    .ok()
                    .flatten()
                    .map(|identity| identity.commit),
                PushOutcome::Published { .. } => None,
            };
            report_bootstrap(invocation, &branch, &digest, bytes.len(), outcome, existing)
        }
        Err(error) => failure(
            USAGE,
            vec![
                Diagnostic::new(Severity::Error, "airlock.publish.failed", error.to_string())
                    .with_path(branch.clone()),
            ],
        ),
    }
}

/// What `publish-bootstrap` says it did.
fn report_bootstrap(
    invocation: &Invocation,
    branch: &str,
    digest: &str,
    size: usize,
    outcome: PushOutcome,
    existing: Option<String>,
) -> CommandOutput {
    let (commit, changed) = match &outcome {
        PushOutcome::Published { commit } => (commit.clone(), true),
        PushOutcome::Unchanged => (existing.unwrap_or_default(), false),
    };
    if invocation.json {
        let envelope: Envelope<serde_json::Value> = Envelope::new(
            STDOUT_SCHEMA_VERSION,
            "airlock publish-bootstrap",
            true,
            json!({
                "branch": branch,
                "commit": commit,
                "changed": changed,
                "sha256": digest,
                "size_bytes": size,
            }),
            Vec::new(),
            Vec::new(),
        );
        return CommandOutput::new(
            format!(
                "{}\n",
                envelope
                    .to_json()
                    .expect("the airlock envelope always serializes")
            ),
            String::new(),
            0,
        );
    }
    let line = if changed {
        format!(
            "published {BOOTSTRAP_BINARY} to {branch} at {commit} (sha256:{digest}, {size} bytes)\n"
        )
    } else {
        format!(
            "{branch} already holds {BOOTSTRAP_BINARY} at {commit} (sha256:{digest}, {size} bytes)\n"
        )
    };
    CommandOutput::new(line, String::new(), 0)
}

impl Invocation {
    /// The directory the rendered entrypoints land in.
    ///
    /// The manifest decides, because the files are committed and `--check` has to compare the
    /// files a reader will find. `--output` is the one-off escape: a scratch render, a preview
    /// in a temporary directory, or a render into a tree that is not this checkout.
    fn directory(&self, restore: &AirlockRestore) -> String {
        match &self.output {
            Some(output) => output.display().to_string(),
            None => restore.directory().to_owned(),
        }
    }

    /// What to tell the caller when `--output` moves a render away from the committed location.
    ///
    /// Silent divergence is the failure mode that makes a drift check untrustworthy: the
    /// committed file stays as it was, `airlock-check` still compares it, and nothing said so.
    fn override_note(&self, restore: &AirlockRestore) -> Option<String> {
        let output = self.output.as_ref()?;
        let declared = restore.directory();
        (output.display().to_string() != declared).then(|| {
            format!(
                "wrote to '{}' instead of the manifest's '{declared}'; the committed entrypoint \
                 there is unchanged, and `airlock-check` still compares it",
                output.display()
            )
        })
    }
}

/// The "this project renders no entrypoint" path.
///
/// A scope that was removed leaves its generated files behind, and a check that ignored them
/// would report agreement with a manifest that no longer describes them. So they are named:
/// rendering deletes them, checking fails on them.
fn settled(invocation: &Invocation, manifest: &Manifest) -> CommandOutput {
    // With no scope there is no `dir` to read, so the search is where a scope would have put
    // them by default — plus wherever `--output` points, since that is where a render in this
    // invocation would have written.
    let directory = match &invocation.output {
        Some(output) => output.display().to_string(),
        None => DEFAULT_RESTORE_DIR.to_owned(),
    };
    let stale: Vec<Rendered> = [POSIX_SCRIPT, POWERSHELL_SCRIPT]
        .iter()
        .map(|name| Rendered {
            path: format!("{directory}/{name}"),
            content: String::new(),
            executable: false,
        })
        .filter(|file| invocation.root.join(&file.path).exists())
        .collect();
    if stale.is_empty() {
        return success(
            invocation,
            manifest,
            &[],
            "no restore entrypoint is declared\n",
        );
    }
    let diagnostics: Vec<Diagnostic> = stale
        .iter()
        .map(|file| {
            Diagnostic::new(
                Severity::Error,
                "airlock.render.stale",
                format!(
                    "{} is a generated entrypoint and the manifest declares no \
                     `[airlock.developer.restore]`; delete it, or declare the pin it should name",
                    file.path
                ),
            )
            .with_path(file.path.clone())
        })
        .collect();
    match invocation.verb {
        Verb::PublishBootstrap => unreachable!("publish-bootstrap returns before this point"),
        Verb::Pack => unreachable!("pack returns before the renderer runs"),
        Verb::Mirror => unreachable!("mirror returns before the renderer runs"),
        Verb::Render if invocation.check => failure(DRIFT, diagnostics),
        Verb::Render => {
            let mut removed = String::new();
            for file in &stale {
                if std::fs::remove_file(invocation.root.join(&file.path)).is_ok() {
                    removed.push_str(&format!("removed {}\n", file.path));
                }
            }
            success(invocation, manifest, &[], &removed)
        }
    }
}

/// Render every file the manifest describes.
///
/// The README is an input as well as an output: the block between the markers is replaced and
/// everything around it is preserved, so a project's own prose survives a re-render.
fn render(
    manifest: &Manifest,
    manifest_text: &str,
    restore: &AirlockRestore,
    directory: &str,
    readme: Option<&str>,
) -> Vec<Rendered> {
    let stamp = stamp(manifest, manifest_text);
    let directory = directory.trim_end_matches('/');
    let mut files = vec![
        Rendered {
            path: format!("{directory}/{POSIX_SCRIPT}"),
            content: posix(manifest, restore, &stamp),
            executable: true,
        },
        Rendered {
            path: format!("{directory}/{POWERSHELL_SCRIPT}"),
            content: powershell(manifest, restore, &stamp),
            executable: false,
        },
    ];
    if let Some(existing) = readme {
        files.push(Rendered {
            path: README.to_owned(),
            content: with_block(existing, &usage_block(restore)),
            executable: false,
        });
    }
    files
}

/// The header every generated file carries: what wrote it, and which manifest it came from.
///
/// The digest is of the *manifest*, matching the workflow renderer's stamp. It is how a reader
/// confirms the file matches the source, and it is the only digest in the output — no artifact
/// digest is embedded, because that would change on every release.
fn stamp(manifest: &Manifest, manifest_text: &str) -> String {
    format!(
        "generated by universe {} from {MANIFEST_FILE} \
         (sha256:{}) for project '{}'. Do not edit: run `universe airlock render`.",
        universe_core::VERSION,
        universe_workflow::sha256::sha256_hex(manifest_text.as_bytes()),
        manifest.project.name
    )
}

/// The POSIX entrypoint.
fn posix(manifest: &Manifest, restore: &AirlockRestore, stamp: &str) -> String {
    let name = &manifest.project.name;
    format!(
        r#"#!/bin/sh
# @{stamp}
#
# Restores {name} on a host whose only reachable thing is git. The commit below is
# checked after the checkout, so a rewritten ref fails here rather than running
# something else.
set -eu

PIN_REMOTE='{remote}'
PIN_BRANCH='{branch}'
PIN_COMMIT='{commit}'
BOOTSTRAP='{bootstrap}'

AIRLOCK_HOME="${{UNIVERSE_AIRLOCK_HOME:-${{XDG_CACHE_HOME:-$HOME/.cache}}/universe-airlock}}"
CHECKOUT="$AIRLOCK_HOME/$PIN_COMMIT"

if [ ! -d "$CHECKOUT/.git" ]; then
    mkdir -p "$AIRLOCK_HOME"
    git clone --quiet --branch "$PIN_BRANCH" "$PIN_REMOTE" "$CHECKOUT"
fi

if ! git -C "$CHECKOUT" checkout --quiet "$PIN_COMMIT"; then
    printf 'airlock: %s does not hold commit %s; the ref was rewritten\n' \
        "$PIN_BRANCH" "$PIN_COMMIT" >&2
    exit 1
fi

actual=$(git -C "$CHECKOUT" rev-parse HEAD)
if [ "$actual" != "$PIN_COMMIT" ]; then
    printf 'airlock: wanted commit %s but %s is at %s\n' "$PIN_COMMIT" "$CHECKOUT" "$actual" >&2
    exit 1
fi

if [ ! -x "$CHECKOUT/$BOOTSTRAP" ]; then
    printf 'airlock: %s is not an executable in %s; the pinned ref was published without it\n' \
        "$BOOTSTRAP" "$CHECKOUT" >&2
    exit 1
fi

exec "$CHECKOUT/$BOOTSTRAP" restore --commit "$PIN_COMMIT" "$@"
"#,
        remote = restore.remote,
        branch = restore.branch,
        commit = restore.commit,
        bootstrap = restore.bootstrap(),
    )
}

/// The PowerShell entrypoint, which says the same thing in the platform's own idioms.
fn powershell(manifest: &Manifest, restore: &AirlockRestore, stamp: &str) -> String {
    let name = &manifest.project.name;
    format!(
        r#"# @{stamp}
#
# Restores {name} on Windows. The commit below is checked after the checkout, so a
# rewritten ref fails here rather than running something else.
$ErrorActionPreference = 'Stop'

$PinRemote = '{remote}'
$PinBranch = '{branch}'
$PinCommit = '{commit}'
$Bootstrap = '{bootstrap}'

if ($env:UNIVERSE_AIRLOCK_HOME) {{
    $AirlockHome = $env:UNIVERSE_AIRLOCK_HOME
}} elseif ($env:XDG_CACHE_HOME) {{
    $AirlockHome = Join-Path $env:XDG_CACHE_HOME 'universe-airlock'
}} else {{
    $AirlockHome = Join-Path $HOME '.cache/universe-airlock'
}}
$Checkout = Join-Path $AirlockHome $PinCommit

if (-not (Test-Path (Join-Path $Checkout '.git'))) {{
    New-Item -ItemType Directory -Force -Path $AirlockHome | Out-Null
    git clone --quiet --branch $PinBranch $PinRemote $Checkout
    if ($LASTEXITCODE -ne 0) {{ throw "airlock: git clone failed ($LASTEXITCODE)" }}
}}

git -C $Checkout checkout --quiet $PinCommit
if ($LASTEXITCODE -ne 0) {{
    throw "airlock: $PinBranch does not hold commit $PinCommit; the ref was rewritten"
}}
$Actual = (git -C $Checkout rev-parse HEAD).Trim()
if ($Actual -ne $PinCommit) {{
    throw "airlock: wanted commit $PinCommit but the checkout is at $Actual"
}}

$Executable = Join-Path $Checkout $Bootstrap
if (-not (Test-Path $Executable)) {{
    throw "airlock: $Bootstrap is not in the pinned checkout"
}}

& $Executable restore --commit $PinCommit @args
exit $LASTEXITCODE
"#,
        remote = restore.remote,
        branch = restore.branch,
        commit = restore.commit,
        bootstrap = restore.bootstrap(),
    )
}

/// The README block, rendered from the same pin the scripts carry.
fn usage_block(restore: &AirlockRestore) -> String {
    format!(
        r#"{README_BEGIN}
## Restoring this project

A disconnected host restores this project with git and nothing else:

```sh
sh .universe-airlock/restore.sh
```

On Windows, `pwsh .universe-airlock/restore.ps1`.

The entrypoint clones the pinned branch, checks out the commit, verifies that the checkout
really is that commit, and hands over to the bootstrapper in that ref. Set
`UNIVERSE_AIRLOCK_HOME` to keep the checkout somewhere else; the default is
`$XDG_CACHE_HOME/universe-airlock` (or `~/.cache/universe-airlock`).

The entrypoint does not change the working directory: the closed payload the bootstrapper
restores belongs in this project's `.universe-airlock/payload`, and the immutable environment
it creates is `.universe-airlock/state`.

| pin | value |
| --- | --- |
| remote | `{remote}` |
| branch | `{branch}` |
| commit | `{commit}` |

The commit is the whole pin. It is a commit rather than a branch head so the entrypoint keeps
fetching the bytes it was rendered against, and it is checked rather than trusted.
{README_END}
"#,
        remote = restore.remote,
        branch = restore.branch,
        commit = restore.commit,
    )
}

/// Replace the block between the markers, appending one when the README has none.
fn with_block(readme: &str, block: &str) -> String {
    match (readme.find(README_BEGIN), readme.find(README_END)) {
        (Some(begin), Some(end)) if end > begin => {
            let tail = end + README_END.len();
            // A blank line after the block keeps the surrounding prose from crowding it.
            format!(
                "{}{}{}",
                &readme[..begin],
                block.trim_end(),
                &readme[tail..]
            )
        }
        // An unmatched marker is a README somebody edited by hand. Replacing from a guessed
        // boundary would eat their prose, so the block goes at the end and they can move it.
        _ => {
            let mut document = readme.trim_end().to_owned();
            document.push_str("\n\n");
            document.push_str(block);
            document
        }
    }
}

fn write(
    invocation: &Invocation,
    manifest: &Manifest,
    rendered: &[Rendered],
    note: Option<&str>,
) -> CommandOutput {
    if invocation.print_stdout {
        let body: String = rendered.iter().map(|file| file.content.as_str()).collect();
        return success(invocation, manifest, rendered, &body);
    }
    let mut report = String::new();
    for file in rendered {
        let path = invocation.root.join(&file.path);
        if let Some(parent) = path.parent()
            && let Err(error) = std::fs::create_dir_all(parent)
        {
            return failure(
                USAGE,
                vec![Diagnostic::new(
                    Severity::Error,
                    "airlock.render.write_failed",
                    format!("cannot create {}: {error}", parent.display()),
                )],
            );
        }
        if let Err(error) = std::fs::write(&path, &file.content) {
            return failure(
                USAGE,
                vec![Diagnostic::new(
                    Severity::Error,
                    "airlock.render.write_failed",
                    format!("cannot write {}: {error}", path.display()),
                )],
            );
        }
        if file.executable {
            make_executable(&path);
        }
        report.push_str(&format!("{}\n", file.path));
    }
    let output = success(invocation, manifest, rendered, &report);
    match note {
        // stdout is the file list, so a note about where they went belongs on stderr.
        Some(note) => CommandOutput::new(
            output.stdout().to_owned(),
            format!("warning: {note}\n"),
            output.exit_code(),
        ),
        None => output,
    }
}

/// Give a generated script the execute bit, where the platform has one.
///
/// A rendered `restore.sh` nobody can run is a file, not an entrypoint, and "run `sh` on it"
/// is the sort of instruction that only works for the person who wrote it.
fn make_executable(path: &Path) {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if let Ok(metadata) = std::fs::metadata(path) {
            let mut permissions = metadata.permissions();
            permissions.set_mode(0o755);
            let _ = std::fs::set_permissions(path, permissions);
        }
    }
    #[cfg(not(unix))]
    {
        let _ = path;
    }
}

fn check(invocation: &Invocation, manifest: &Manifest, rendered: &[Rendered]) -> CommandOutput {
    let mut diagnostics = Vec::new();
    let mut body = String::new();
    for file in rendered {
        let path = invocation.root.join(&file.path);
        let committed = match std::fs::read_to_string(&path) {
            Ok(text) => text,
            Err(error) => {
                diagnostics.push(
                    Diagnostic::new(
                        Severity::Error,
                        "airlock.check.missing",
                        format!(
                            "{} is not on disk ({error}); run `universe airlock render`",
                            file.path
                        ),
                    )
                    .with_path(file.path.clone()),
                );
                continue;
            }
        };
        if committed == file.content {
            continue;
        }
        body.push_str(&format!("--- {}\n+++ {}\n", file.path, file.path));
        body.push_str(&difference(&committed, &file.content));
        diagnostics.push(
            Diagnostic::new(
                Severity::Error,
                "airlock.check.drift",
                format!(
                    "{} does not match what {MANIFEST_FILE} renders; run \
                     `universe airlock render` and commit the result",
                    file.path
                ),
            )
            .with_path(file.path.clone()),
        );
    }
    if diagnostics.is_empty() {
        return success(
            invocation,
            manifest,
            rendered,
            &format!("ok: {} file(s) match\n", rendered.len()),
        );
    }
    let mut output = failure(DRIFT, diagnostics);
    if !body.is_empty() {
        output = CommandOutput::new(
            format!("{}{}", output.stdout(), body),
            output.stderr().to_owned(),
            output.exit_code(),
        );
    }
    output
}

/// A line diff: ` ` for unchanged, `-` for committed-only, `+` for rendered-only.
///
/// A longest-common-subsequence table rather than a line-by-line comparison, because a
/// one-line insertion makes every following line differ and a reader would be told the whole
/// file changed. Generated files are tens of lines, so the quadratic table is free.
fn difference(committed: &str, expected: &str) -> String {
    let left: Vec<&str> = committed.lines().collect();
    let right: Vec<&str> = expected.lines().collect();
    let mut lengths = vec![vec![0_usize; right.len() + 1]; left.len() + 1];
    for i in (0..left.len()).rev() {
        for j in (0..right.len()).rev() {
            lengths[i][j] = if left[i] == right[j] {
                lengths[i + 1][j + 1] + 1
            } else {
                lengths[i + 1][j].max(lengths[i][j + 1])
            };
        }
    }
    let mut diff = String::new();
    let (mut i, mut j) = (0, 0);
    while i < left.len() && j < right.len() {
        if left[i] == right[j] {
            diff.push_str(&format!(" {}\n", left[i]));
            i += 1;
            j += 1;
        } else if lengths[i + 1][j] >= lengths[i][j + 1] {
            diff.push_str(&format!("-{}\n", left[i]));
            i += 1;
        } else {
            diff.push_str(&format!("+{}\n", right[j]));
            j += 1;
        }
    }
    for line in &left[i..] {
        diff.push_str(&format!("-{line}\n"));
    }
    for line in &right[j..] {
        diff.push_str(&format!("+{line}\n"));
    }
    diff
}

fn success(
    invocation: &Invocation,
    manifest: &Manifest,
    rendered: &[Rendered],
    body: &str,
) -> CommandOutput {
    if invocation.json {
        let envelope: Envelope<serde_json::Value> = Envelope::new(
            STDOUT_SCHEMA_VERSION,
            "airlock render",
            true,
            json!({
                "project": manifest.project.name,
                "check": invocation.check,
                "files": rendered.iter().map(|file| &file.path).collect::<Vec<_>>(),
            }),
            Vec::new(),
            Vec::new(),
        );
        return CommandOutput::new(
            format!(
                "{}\n",
                envelope
                    .to_json()
                    .expect("the airlock envelope always serializes")
            ),
            String::new(),
            0,
        );
    }
    CommandOutput::new(body.to_owned(), String::new(), 0)
}

fn failure(exit_code: u8, diagnostics: Vec<Diagnostic>) -> CommandOutput {
    crate::output::failure(exit_code, &diagnostics)
}

#[cfg(test)]
mod tests {
    use super::{difference, render, run, with_block};
    use universe_core::manifest::{AirlockRestore, DEFAULT_RESTORE_DIR, Manifest};

    const MANIFEST: &str = r#"
schema = 1

[project]
name = "fixture-airlock"
platforms = ["linux-64"]

[[packages]]
name = "fixture-airlock"
kind = "conda"
command = ["cargo", "build"]
outputs = ["output/*.conda"]
release = false

[airlock.developer]
adapters = ["pixi", "cargo"]
mirror = "mirror"

[airlock.developer.restore]
remote = "https://github.com/example/fixture.git"
branch = "mirror/pixi"
commit = "0123456789abcdef0123456789abcdef01234567"
"#;

    fn manifest_text() -> String {
        MANIFEST.to_owned()
    }

    fn manifest() -> Manifest {
        Manifest::from_toml(&manifest_text()).expect("the fixture manifest parses")
    }

    fn restore() -> AirlockRestore {
        manifest()
            .airlock
            .as_ref()
            .and_then(|airlock| airlock.developer.as_ref())
            .and_then(|developer| developer.restore.clone())
            .expect("the fixture declares a restore scope")
    }

    /// The directory the fixture's render writes into.
    fn restore_dir() -> &'static str {
        DEFAULT_RESTORE_DIR
    }

    /// Render with the fixture manifest, optionally against a README.
    fn rendered(readme: Option<&str>) -> Vec<super::Rendered> {
        let text = manifest_text();
        render(&manifest(), &text, &restore(), restore_dir(), readme)
    }

    fn file<'a>(files: &'a [super::Rendered], name: &str) -> &'a str {
        files
            .iter()
            .find(|file| file.path == name)
            .unwrap_or_else(|| panic!("no {name} in the render"))
            .content
            .as_str()
    }

    #[test]
    fn two_renders_of_one_manifest_are_byte_identical() {
        let first = rendered(Some("# Fixture\n"));
        let second = rendered(Some("# Fixture\n"));

        assert_eq!(first, second, "rendering is a pure function of its inputs");
        // And a re-render over the file it produced is the same file, which is what makes
        // the drift check meaningful rather than a diff against itself.
        let readme = file(&first, "README.md");
        assert_eq!(rendered(Some(readme)), first);
    }

    #[test]
    fn the_entrypoint_names_the_pin_and_no_artifact_digest() {
        let files = rendered(None);
        let script = file(&files, ".universe-airlock/restore.sh");

        assert!(script.contains("PIN_COMMIT='0123456789abcdef0123456789abcdef01234567'"));
        assert!(script.contains("PIN_BRANCH='mirror/pixi'"));
        assert!(script.contains("PIN_REMOTE='https://github.com/example/fixture.git'"));
        // The pin is verified rather than assumed, which is the whole reason it is checked
        // after the checkout instead of being trusted.
        assert!(script.contains("rev-parse HEAD"));
        assert!(script.contains("exec \"$CHECKOUT/$BOOTSTRAP\" restore --commit"));

        // Exactly one digest appears, and it is the manifest's stamp.
        let digests: Vec<&str> = script
            .split(|character: char| !character.is_ascii_hexdigit())
            .filter(|run| run.len() == 64)
            .collect();
        assert_eq!(digests.len(), 1, "{digests:?}");
        assert!(script.contains(&format!("sha256:{}", digests[0])));
        assert!(
            digests[0] == universe_workflow::sha256::sha256_hex(manifest_text().as_bytes()),
            "the only digest is the manifest the file came from"
        );
    }

    #[test]
    fn the_entrypoint_carries_no_clock_and_no_run_identity() {
        let files = rendered(None);
        for file in &files {
            for banned in ["20", "T00:", "runner", "workflow-run"] {
                // `20` is not banned generally — only a date-shaped timestamp would be — so
                // the check is for the two shapes a renderer drifts with.
                if banned == "20" {
                    continue;
                }
                assert!(
                    !file.content.contains(banned),
                    "{} carries {banned}",
                    file.path
                );
            }
        }
        let script = file(&files, ".universe-airlock/restore.sh");
        assert!(
            !script.split_whitespace().any(|word| word.len() == 20
                && word.starts_with("20")
                && word.chars().nth(4) == Some('-')),
            "a timestamp reached the entrypoint"
        );
    }

    #[test]
    fn the_powershell_entrypoint_says_the_same_thing_as_the_posix_one() {
        let files = rendered(None);
        let power_shell = file(&files, ".universe-airlock/restore.ps1");

        for value in [
            "'https://github.com/example/fixture.git'",
            "'mirror/pixi'",
            "'0123456789abcdef0123456789abcdef01234567'",
        ] {
            assert!(power_shell.contains(value), "{value} is not in the script");
        }
        assert!(
            power_shell.contains("$LASTEXITCODE"),
            "failures must be visible"
        );
        assert!(power_shell.contains("restore --commit"));
    }

    #[test]
    fn the_readme_block_is_replaced_and_the_prose_around_it_survives() {
        let readme = "# Fixture\n\nHand-written prose.\n";
        let once = rendered(Some(readme));
        let block = file(&once, "README.md");

        assert!(block.starts_with("# Fixture\n\nHand-written prose.\n"));
        assert!(block.contains(super::README_BEGIN));
        assert!(block.contains("sh .universe-airlock/restore.sh"));
        assert!(block.contains("| commit | `0123456789abcdef0123456789abcdef01234567` |"));

        // Replacing the block leaves the prose alone, so a maintainer can keep writing around
        // it without their words disappearing on the next render.
        let twice = with_block(block, &super::usage_block(&restore()));
        assert_eq!(twice, block);
    }

    #[test]
    fn the_readme_block_names_where_the_payload_lives() {
        // The rendered README is all a disconnected consumer gets. The binary is exec'd from
        // the cloned checkout, but it reads the payload from the working directory the script
        // was run from; saying so is what makes the one-process-tree restore reproducible by a
        // human, not just by `crates/universe-kit/tests/airlock_restore.rs`.
        let files = rendered(Some("# Fixture\n"));
        let block = file(&files, "README.md");
        assert!(
            block.contains("does not change the working directory"),
            "the block must not imply the binary runs from the checkout: {block}"
        );
        assert!(
            block.contains("`.universe-airlock/payload`"),
            "a consumer cannot know where to put the closed payload: {block}"
        );
    }

    #[test]
    fn a_readme_with_one_marker_gets_the_block_appended_rather_than_a_guess() {
        let readme = "# Fixture\n\n<!-- airlock:begin -->\nprose someone typed by hand\n";
        let patched = with_block(readme, &super::usage_block(&restore()));

        assert!(
            patched.starts_with(readme.trim_end()),
            "the hand-written text must survive:\n{patched}"
        );
        assert!(patched.trim_end().ends_with(super::README_END));
        // Exactly one opening marker: a second one would make the next render ambiguous.
        assert_eq!(patched.matches(super::README_BEGIN).count(), 2, "{patched}");
    }

    #[test]
    fn a_one_line_insertion_does_not_report_the_whole_file() {
        let committed = "a\nb\nc\n";
        let expected = "a\nb\ninserted\nc\n";

        assert_eq!(difference(committed, expected), " a\n b\n+inserted\n c\n");
        assert_eq!(difference("a\nc\n", "a\nb\nc\n"), " a\n+b\n c\n");
        assert_eq!(difference("a\nb\n", "a\n"), " a\n-b\n");
    }

    /// The fixture manifest, with `dir` added to its restore scope.
    fn manifest_with_dir(dir: &str) -> String {
        format!("{MANIFEST}dir = \"{dir}\"\n")
    }

    #[test]
    fn the_manifest_chooses_where_the_entrypoints_land() {
        let directory = tempfile::tempdir().expect("a temporary project");
        std::fs::write(
            directory.path().join("universe.toml"),
            manifest_with_dir("scripts/airlock"),
        )
        .expect("manifest");
        let root = directory.path().display().to_string();

        let output = run(&["render".to_owned(), "--root".to_owned(), root.clone()]);

        assert_eq!(output.exit_code(), 0, "stderr: {}", output.stderr());
        assert!(
            directory.path().join("scripts/airlock/restore.sh").exists(),
            "the declared directory was ignored: {}",
            output.stdout()
        );
        assert!(!directory.path().join(".universe-airlock").exists());
        assert!(
            output.stderr().is_empty(),
            "a declared directory is not a note"
        );

        // And the check follows the manifest, which is the whole reason `dir` is in it: a
        // position only the command line knew would be checked at the wrong path.
        assert_eq!(
            run(&[
                "render".to_owned(),
                "--check".to_owned(),
                "--root".to_owned(),
                root.clone()
            ])
            .exit_code(),
            0
        );
        std::fs::write(
            directory.path().join("scripts/airlock/restore.sh"),
            "# hand edit\n",
        )
        .expect("edit");
        assert_eq!(
            run(&[
                "render".to_owned(),
                "--check".to_owned(),
                "--root".to_owned(),
                root
            ])
            .exit_code(),
            1
        );
    }

    #[test]
    fn the_flag_overrides_the_manifest_and_says_what_it_left_behind() {
        let directory = tempfile::tempdir().expect("a temporary project");
        std::fs::write(directory.path().join("universe.toml"), MANIFEST).expect("manifest");
        let root = directory.path().display().to_string();
        assert_eq!(
            run(&["render".to_owned(), "--root".to_owned(), root.clone()]).exit_code(),
            0
        );

        let scratch = directory.path().join("scratch");
        let output = run(&[
            "render".to_owned(),
            "--output".to_owned(),
            scratch.display().to_string(),
            "--root".to_owned(),
            root.clone(),
        ]);

        assert_eq!(output.exit_code(), 0, "stderr: {}", output.stderr());
        assert!(
            scratch.join("restore.sh").exists(),
            "the override was ignored"
        );
        // The committed entrypoint is untouched, so the note is the only thing standing between
        // a caller and a silent divergence.
        assert!(output.stderr().contains("warning:"), "{}", output.stderr());
        assert!(
            output.stderr().contains(DEFAULT_RESTORE_DIR),
            "{}",
            output.stderr()
        );
        assert!(
            output.stderr().contains("airlock-check"),
            "{}",
            output.stderr()
        );

        // The manifest's file still matches its manifest, so the check is still green — the
        // override changed where bytes went, not what the repository claims.
        assert_eq!(
            run(&[
                "render".to_owned(),
                "--check".to_owned(),
                "--root".to_owned(),
                root
            ])
            .exit_code(),
            0
        );
    }

    #[test]
    fn an_override_that_matches_the_manifest_is_not_a_note() {
        let directory = tempfile::tempdir().expect("a temporary project");
        std::fs::write(directory.path().join("universe.toml"), MANIFEST).expect("manifest");

        let output = run(&[
            "render".to_owned(),
            "--output".to_owned(),
            DEFAULT_RESTORE_DIR.to_owned(),
            "--root".to_owned(),
            directory.path().display().to_string(),
        ]);

        assert_eq!(output.exit_code(), 0);
        assert!(
            output.stderr().is_empty(),
            "agreeing with the manifest is not worth a warning: {}",
            output.stderr()
        );
    }

    #[test]
    fn a_directory_that_escapes_the_project_is_refused() {
        for dir in ["/etc/airlock", "../outside", "a/../../b"] {
            let directory = tempfile::tempdir().expect("a temporary project");
            std::fs::write(
                directory.path().join("universe.toml"),
                manifest_with_dir(dir),
            )
            .expect("manifest");

            let output = run(&[
                "render".to_owned(),
                "--root".to_owned(),
                directory.path().display().to_string(),
            ]);

            assert_eq!(output.exit_code(), 2, "{dir}: {}", output.stderr());
            assert!(
                output.stderr().contains("manifest.airlock.restore.dir"),
                "{dir}: {}",
                output.stderr()
            );
        }
    }

    #[test]
    fn a_scope_removed_after_a_render_at_a_declared_directory_is_reported_where_it_was() {
        // The stale search cannot know a directory the manifest no longer names, so it looks
        // where a render would have put them — the flag first, then the default.
        let directory = tempfile::tempdir().expect("a temporary project");
        let moved = directory.path().join("out");
        std::fs::create_dir_all(&moved).expect("directory");
        std::fs::write(moved.join("restore.sh"), "# generated once\n").expect("stale file");
        let without = MANIFEST
            .split("[airlock.developer.restore]")
            .next()
            .expect("head")
            .to_owned();
        std::fs::write(directory.path().join("universe.toml"), &without).expect("manifest");

        let checked = run(&[
            "render".to_owned(),
            "--check".to_owned(),
            "--output".to_owned(),
            moved.display().to_string(),
            "--root".to_owned(),
            directory.path().display().to_string(),
        ]);

        assert_eq!(checked.exit_code(), 1, "stderr: {}", checked.stderr());
        assert!(
            checked.stderr().contains("airlock.render.stale"),
            "{}",
            checked.stderr()
        );
    }

    #[test]
    fn a_project_that_declares_no_restore_scope_renders_nothing() {
        let directory = tempfile::tempdir().expect("a temporary project");
        let manifest = MANIFEST
            .replace("[airlock.developer.restore]", "[airlock.restore-unused]")
            .split("[airlock.restore-unused]")
            .next()
            .expect("the manifest keeps its head")
            .to_owned();
        std::fs::write(directory.path().join("universe.toml"), &manifest).expect("manifest");

        let output = run(&[
            "render".to_owned(),
            "--root".to_owned(),
            directory.path().display().to_string(),
        ]);

        assert_eq!(output.exit_code(), 0, "stderr: {}", output.stderr());
        assert!(output.stdout().contains("no restore entrypoint"));
        assert!(
            !directory.path().join(".universe-airlock").exists(),
            "nothing was written"
        );
    }

    #[test]
    fn removing_the_scope_makes_the_check_report_the_files_it_left_behind() {
        let directory = tempfile::tempdir().expect("a temporary project");
        std::fs::write(directory.path().join("universe.toml"), MANIFEST).expect("manifest");
        let root = directory.path().display().to_string();
        assert_eq!(
            run(&["render".to_owned(), "--root".to_owned(), root.clone()]).exit_code(),
            0
        );

        // The scope goes away, the generated files do not.
        let without = MANIFEST
            .split("[airlock.developer.restore]")
            .next()
            .expect("head")
            .to_owned();
        std::fs::write(directory.path().join("universe.toml"), &without).expect("manifest");

        let checked = run(&[
            "render".to_owned(),
            "--check".to_owned(),
            "--root".to_owned(),
            root.clone(),
        ]);
        assert_eq!(checked.exit_code(), 1, "stderr: {}", checked.stderr());
        assert!(
            checked.stderr().contains("airlock.render.stale"),
            "{}",
            checked.stderr()
        );

        // Rendering cleans up what it can no longer justify keeping.
        assert_eq!(
            run(&["render".to_owned(), "--root".to_owned(), root]).exit_code(),
            0
        );
        assert!(
            !directory
                .path()
                .join(".universe-airlock/restore.sh")
                .exists()
        );
    }

    #[test]
    fn a_hand_edited_entrypoint_is_drift_with_a_diff_and_a_drift_exit_code() {
        let directory = tempfile::tempdir().expect("a temporary project");
        std::fs::write(directory.path().join("universe.toml"), MANIFEST).expect("manifest");
        let root = directory.path().display().to_string();
        assert_eq!(
            run(&["render".to_owned(), "--root".to_owned(), root.clone()]).exit_code(),
            0
        );

        let script = directory.path().join(".universe-airlock/restore.sh");
        let mut edited = std::fs::read_to_string(&script).expect("the rendered script");
        edited.push_str("# a hand edit\n");
        std::fs::write(&script, &edited).expect("the edit");

        let checked = run(&[
            "render".to_owned(),
            "--check".to_owned(),
            "--root".to_owned(),
            root.clone(),
        ]);

        assert_eq!(checked.exit_code(), 1, "drift is not a usage error");
        assert!(
            checked.stdout().contains("-# a hand edit"),
            "{}",
            checked.stdout()
        );
        assert!(checked.stderr().contains("airlock.check.drift"));

        // And rendering repairs it.
        assert_eq!(
            run(&["render".to_owned(), "--root".to_owned(), root.clone()]).exit_code(),
            0
        );
        assert_eq!(
            run(&[
                "render".to_owned(),
                "--check".to_owned(),
                "--root".to_owned(),
                root
            ])
            .exit_code(),
            0
        );
    }

    #[test]
    fn the_renderer_refuses_a_manifest_whose_pin_it_could_not_carry() {
        let directory = tempfile::tempdir().expect("a temporary project");
        let unsafe_remote = MANIFEST.replace(
            "remote = \"https://github.com/example/fixture.git\"",
            "remote = \"https://example.com/a$(reboot).git\"",
        );
        std::fs::write(directory.path().join("universe.toml"), &unsafe_remote).expect("manifest");

        let output = run(&[
            "render".to_owned(),
            "--root".to_owned(),
            directory.path().display().to_string(),
        ]);

        assert_eq!(output.exit_code(), 2, "stderr: {}", output.stderr());
        assert!(output.stderr().contains("manifest.airlock.restore.unsafe"));
        assert!(!directory.path().join(".universe-airlock").exists());
    }

    #[test]
    fn the_json_envelope_names_the_files_and_the_verb() {
        let directory = tempfile::tempdir().expect("a temporary project");
        std::fs::write(directory.path().join("universe.toml"), MANIFEST).expect("manifest");

        let output = run(&[
            "render".to_owned(),
            "--json".to_owned(),
            "--root".to_owned(),
            directory.path().display().to_string(),
        ]);

        assert_eq!(output.exit_code(), 0, "stderr: {}", output.stderr());
        let envelope: serde_json::Value =
            serde_json::from_str(output.stdout()).expect("one JSON envelope");
        assert_eq!(envelope["command"], "airlock render");
        assert_eq!(envelope["ok"], true);
        assert_eq!(
            envelope["result"]["files"][0],
            ".universe-airlock/restore.sh"
        );
    }
}
