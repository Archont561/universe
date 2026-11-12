//! `universe-airlock sandbox`: install a built set into a prefix and run a command inside it.
//!
//! This is the local development loop the Conda track exists for. A person who has just built a
//! `.conda` wants to run the thing the package ships — the command, the interpreter, the data
//! files beside it — without publishing it, without solving anything, and without their own
//! host environment quietly standing in for the package's dependencies.
//!
//! What it guarantees, in the order the guarantees matter:
//!
//! - **The set is installed before anything runs.** The installer is the bootstrapper's
//!   (`universe_airlock_bootstrap::prefix`), so an archive is digest-checked and `paths.json` is
//!   applied here exactly as it is on a restore. A set that cannot be installed never runs the
//!   command, which is what stops a failing install from looking like a passing package.
//! - **The prefix comes first.** The child's `PATH` begins with `<prefix>/bin`, so a command
//!   whose name also exists on the host runs the set's one.
//! - **The host's Python configuration does not leak in.** `PYTHONPATH`, `PYTHONHOME`,
//!   `PYTHONSTARTUP`, `PYTHONEXECUTABLE`, `VIRTUAL_ENV` and `CONDA_PREFIX_1` are removed and
//!   `PYTHONNOUSERSITE` is set. Without that, a `noarch: python` package imports modules from
//!   outside the set it was installed with, and the run passes for a reason that has nothing to
//!   do with the packages.
//!
//! What it is **not**, and says on every run: an operating-system sandbox. No mount, user, or
//! network namespace is created, so the command can still reach the network and the host
//! filesystem. The isolation is environmental — `PATH`, `CONDA_PREFIX`, and the Python variables
//! above — plus the fact that the prefix is discarded when the run ends unless the caller asks
//! to keep it. Calling that an air gap would be a claim this code cannot support.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

use universe_airlock_bootstrap::{
    diagnostics::{self, Finding},
    hash::hash_file,
    prefix::{self, Pinned, Replayed},
};
use universe_core::{
    CommandOutput, STDOUT_SCHEMA_VERSION,
    diagnostic::{Diagnostic, Envelope},
};

/// Exit code for a run that refused before the command could run.
const EXIT_REFUSED: u8 = 1;

/// Exit code for an invocation this build does not understand.
const EXIT_USAGE: u8 = 2;

/// Variables that point at some *other* installation, and would make the sandbox lie.
///
/// `PYTHONPATH` and friends are the reason this list exists: an installed `noarch: python`
/// package that imports a module is importing it from whichever `site-packages` the interpreter
/// resolves, and the host's variables change that answer. `CONDA_PREFIX_1` is the prefix of an
/// enclosing activated Conda environment in a nested activation, which is this run's shadow.
const FOREIGN: [&str; 6] = [
    "PYTHONPATH",
    "PYTHONHOME",
    "PYTHONSTARTUP",
    "PYTHONEXECUTABLE",
    "VIRTUAL_ENV",
    "CONDA_PREFIX_1",
];

/// Counter for the temporary prefixes of one process, so two runs never name one directory.
static NEXT: AtomicU64 = AtomicU64::new(0);

/// What a caller asked for.
pub struct Request {
    /// `.conda` archives to install together.
    pub packages: Vec<String>,
    /// A mirror directory to install from instead.
    pub mirror: Option<String>,
    /// A directory to keep the prefix in, instead of a temporary one.
    pub dir: Option<String>,
    /// Whether to keep a temporary prefix.
    pub keep: bool,
    /// The command to run, already split from this binary's own arguments.
    pub command: Vec<String>,
}

/// The result a `--json` run reports.
#[derive(serde::Serialize)]
struct ResultValue {
    prefix: String,
    artifacts: usize,
    entries: u64,
    bytes: u64,
    order: Vec<String>,
    command: Vec<String>,
    exit: i32,
    stdout: String,
    stderr: String,
}

/// Install the requested set and run the command inside it.
pub fn run(request: Request, json: bool) -> CommandOutput {
    if request.command.is_empty() {
        return usage_error(
            "sandbox needs a command after `--`: its whole job is running one inside the prefix",
        );
    }
    if !request.packages.is_empty() && request.mirror.is_some() {
        return usage_error(
            "sandbox installs from either --package or --mirror, not both: one set, one source",
        );
    }
    let working_directory = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

    // A temporary prefix is created here, so a refusal below has something to remove.
    let temporary = request.dir.is_none();
    let prefix = match &request.dir {
        Some(directory) => match prepare(Path::new(directory)) {
            Ok(prefix) => prefix,
            Err(finding) => return refused(&[finding]),
        },
        None => match scratch() {
            Ok(prefix) => prefix,
            Err(finding) => {
                return refused(&[finding]);
            }
        },
    };

    let replayed = match install(&request, &working_directory, &prefix) {
        Ok(replayed) => replayed,
        Err(finding) => {
            if temporary {
                let _ = std::fs::remove_dir_all(&prefix);
            }
            return refused(&[finding]);
        }
    };

    // The notes go out before the command runs in the human mode, because a note printed after
    // the program's output reads as a comment on it rather than as a condition of the run.
    let mut notes = replayed.notes.clone();
    notes.push(Finding::warning(
        "airlock.sandbox.environment",
        "this sandbox isolates the environment, not the machine: the command can still reach the \
         network and the filesystem outside the prefix",
    ));

    let (code, child_stdout, child_stderr) = match execute(&request.command, &prefix, json) {
        Ok(executed) => executed,
        Err(finding) => {
            if temporary && !request.keep {
                let _ = std::fs::remove_dir_all(&prefix);
            }
            return refused(&[finding]);
        }
    };

    let kept = request.keep || !temporary;
    if !kept {
        let _ = std::fs::remove_dir_all(&prefix);
    }

    let stdout = if json {
        let result = ResultValue {
            prefix: prefix.display().to_string(),
            artifacts: replayed.artifacts,
            entries: replayed.entries,
            bytes: replayed.bytes,
            order: replayed.order.clone(),
            command: request.command.clone(),
            exit: code,
            stdout: child_stdout,
            stderr: child_stderr,
        };
        let diagnostics: Vec<Diagnostic> = notes
            .iter()
            .map(|finding| Diagnostic::new(finding.severity(), finding.code(), finding.message()))
            .collect();
        let envelope = Envelope::new(
            STDOUT_SCHEMA_VERSION,
            "sandbox",
            true,
            result,
            diagnostics,
            Vec::new(),
        );
        format!(
            "{}\n",
            envelope.to_json().expect("a sandbox report serialises")
        )
    } else {
        format!(
            "sandbox: {}\nartifacts: {}{}\norder: {}\ncommand: {}\nexit: {}\nprefix kept: {}\n",
            prefix.display(),
            replayed.artifacts,
            if replayed.skipped == 0 {
                String::new()
            } else {
                format!(" ({} left out)", replayed.skipped)
            },
            replayed.order.join(", "),
            request.command.join(" "),
            code,
            if kept { "yes" } else { "no" }
        )
    };
    CommandOutput::new(
        stdout,
        diagnostics::render_all(&notes),
        u8::try_from(code).unwrap_or(EXIT_REFUSED),
    )
}

/// Install the set the request names into `prefix`.
fn install(
    request: &Request,
    working_directory: &Path,
    prefix: &Path,
) -> Result<Replayed, Finding> {
    if !request.packages.is_empty() {
        let mut pinned: Vec<Pinned> = Vec::with_capacity(request.packages.len());
        for package in &request.packages {
            let path = absolute(working_directory, package);
            let (sha256, size) = hash_file(&path).map_err(|error| {
                Finding::error(
                    "airlock.sandbox.package",
                    format!(
                        "cannot read {}: {error}; a sandbox installs the packages it is given and \
                         nothing else",
                        path.display()
                    ),
                )
            })?;
            pinned.push(Pinned {
                path: path.display().to_string(),
                sha256,
                size: Some(size),
            });
        }
        // The root is where the paths are resolved from; these are already absolute, so it only
        // has to be a directory that exists.
        return prefix::replay(&pinned, working_directory, prefix);
    }
    let mirror = request
        .mirror
        .as_deref()
        .map(PathBuf::from)
        .unwrap_or_else(|| working_directory.join(".universe-airlock/mirror"));
    prefix::replay_mirror(&mirror, prefix)
}

/// Run the command with the prefix as its environment.
///
/// In the human mode the child writes to this process' own streams, so `git log | sandbox …` and
/// a terminal both behave. The `--json` mode captures them instead, because an envelope has to be
/// the only thing on stdout for a reader to parse it.
fn execute(
    command: &[String],
    prefix: &Path,
    capture: bool,
) -> Result<(i32, String, String), Finding> {
    let mut invocation = Command::new(&command[0]);
    invocation.args(&command[1..]);
    invocation.env("PATH", path_value(prefix));
    invocation.env("CONDA_PREFIX", prefix);
    invocation.env("CONDA_DEFAULT_ENV", prefix.display().to_string());
    // User site-packages are the other half of the same leak: an interpreter's `user base` is a
    // directory outside every prefix, and a package that shadows one of its modules there would
    // be silently replaced by it.
    invocation.env("PYTHONNOUSERSITE", "1");
    for name in FOREIGN {
        invocation.env_remove(name);
    }

    if capture {
        let output = invocation
            .output()
            .map_err(|error| launch_error(&command[0], &error))?;
        let code = output.status.code().unwrap_or(EXIT_REFUSED.into());
        return Ok((
            code,
            String::from_utf8_lossy(&output.stdout).into_owned(),
            String::from_utf8_lossy(&output.stderr).into_owned(),
        ));
    }

    let status = invocation
        .status()
        .map_err(|error| launch_error(&command[0], &error))?;
    Ok((
        status.code().unwrap_or(EXIT_REFUSED.into()),
        String::new(),
        String::new(),
    ))
}

fn launch_error(command: &str, error: &std::io::Error) -> Finding {
    Finding::error(
        "airlock.sandbox.command",
        format!("cannot run `{command}`: {error}"),
    )
}

/// `<prefix>/bin` first, then whatever the caller had.
///
/// The host's `PATH` is kept rather than cleared: a package ships the tools it ships, and a shell
/// script still needs `/bin/sh`. What the order buys is that the set's own command wins when both
/// exist, which is the only way a sandbox exercises the package instead of the host.
fn path_value(prefix: &Path) -> String {
    let bin = prefix.join("bin");
    match std::env::var_os("PATH") {
        Some(inherited) if !inherited.is_empty() => {
            let mut paths = vec![bin.clone()];
            paths.extend(std::env::split_paths(&inherited));
            std::env::join_paths(paths)
                .map(|joined| joined.to_string_lossy().into_owned())
                .unwrap_or_else(|_| bin.display().to_string())
        }
        _ => bin.display().to_string(),
    }
}

/// A directory the caller named, which has to be empty or absent.
///
/// An existing prefix is refused rather than reused: installing this set on top of another would
/// leave a directory that is neither, and the run would then be testing all of it.
fn prepare(directory: &Path) -> Result<PathBuf, Finding> {
    if directory.exists() {
        let empty = std::fs::read_dir(directory)
            .map_err(|error| {
                Finding::error(
                    "airlock.sandbox.dir",
                    format!("cannot read {}: {error}", directory.display()),
                )
            })?
            .next()
            .is_none();
        if !empty {
            return Err(Finding::error(
                "airlock.sandbox.dir",
                format!(
                    "{} is not empty; a sandbox installs into an empty directory, because a \
                     prefix holding another set is a different environment",
                    directory.display()
                ),
            ));
        }
        return Ok(directory.to_path_buf());
    }
    std::fs::create_dir_all(directory).map_err(|error| {
        Finding::error(
            "airlock.sandbox.dir",
            format!("cannot create {}: {error}", directory.display()),
        )
    })?;
    Ok(directory.to_path_buf())
}

/// A fresh directory under the system's temporary directory.
fn scratch() -> Result<PathBuf, Finding> {
    let base = std::env::temp_dir();
    for _ in 0..64 {
        let unique = NEXT.fetch_add(1, Ordering::Relaxed);
        let candidate = base.join(format!(
            "universe-airlock-sandbox-{}-{unique}",
            std::process::id()
        ));
        match std::fs::create_dir(&candidate) {
            Ok(()) => return Ok(candidate),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => {
                return Err(Finding::error(
                    "airlock.sandbox.dir",
                    format!("cannot create {}: {error}", candidate.display()),
                ));
            }
        }
    }
    Err(Finding::error(
        "airlock.sandbox.dir",
        format!(
            "cannot find a free sandbox directory under {}",
            base.display()
        ),
    ))
}

/// A path as the caller named it, resolved against the directory they ran in.
fn absolute(working_directory: &Path, path: &str) -> PathBuf {
    let candidate = PathBuf::from(path);
    if candidate.is_absolute() {
        candidate
    } else {
        working_directory.join(candidate)
    }
}

fn refused(findings: &[Finding]) -> CommandOutput {
    CommandOutput::new(
        String::new(),
        diagnostics::render_all(findings),
        EXIT_REFUSED,
    )
}

fn usage_error(message: &str) -> CommandOutput {
    CommandOutput::new(String::new(), format!("error: {message}\n"), EXIT_USAGE)
}
