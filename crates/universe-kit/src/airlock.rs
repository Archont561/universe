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
use universe_core::diagnostic::{Diagnostic, Envelope, Severity};
use universe_core::manifest::{AirlockRestore, DEFAULT_RESTORE_DIR, Manifest};
use universe_core::{CommandOutput, STDOUT_SCHEMA_VERSION};

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
    /// Write the rendered files.
    Render,
    /// Compare the rendered files with what is on disk.
    Check,
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
}

/// Why an invocation could not be interpreted.
#[derive(Clone, Debug, Eq, PartialEq)]
struct UsageError(String);

const USAGE_LINE: &str = "usage: universe airlock render [--check] [--output <dir>] \
                          [--root <dir>] [--stdout] [--json]";

impl Invocation {
    fn parse(args: &[String]) -> Result<Self, UsageError> {
        let mut invocation = Self {
            verb: Verb::Render,
            root: PathBuf::from("."),
            output: None,
            print_stdout: false,
            json: false,
        };
        let mut verb_seen = false;
        let mut rest = args.iter();
        while let Some(argument) = rest.next() {
            match argument.as_str() {
                "render" if !verb_seen => {
                    invocation.verb = Verb::Render;
                    verb_seen = true;
                }
                "--check" => invocation.verb = Verb::Check,
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
        Ok(invocation)
    }

    fn manifest_path(&self) -> PathBuf {
        self.root.join(MANIFEST_FILE)
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
        Verb::Render => write(invocation, &manifest, &rendered, note.as_deref()),
        Verb::Check => check(invocation, &manifest, &rendered),
    }
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
        Verb::Render => {
            let mut removed = String::new();
            for file in &stale {
                if std::fs::remove_file(invocation.root.join(&file.path)).is_ok() {
                    removed.push_str(&format!("removed {}\n", file.path));
                }
            }
            success(invocation, manifest, &[], &removed)
        }
        Verb::Check => failure(DRIFT, diagnostics),
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
                "check": invocation.verb == Verb::Check,
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
