//! `universe publish`: put a rendered channel on a Git branch.
//!
//! The order here is the whole argument of the verb. Fragments are read and proved against
//! their bytes, the tree is assembled and measured against the budgets, and only then does
//! anything leave the machine. A tree that cannot be measured is never pushed, so a failure
//! costs a rebuild and not a broken channel.

use std::path::{Path, PathBuf};

use serde_json::json;
use universe_channel::transport::{Budgets, ChannelTree, PushOutcome, Transport};
use universe_core::CommandOutput;
use universe_core::diagnostic::{Diagnostic, Severity};
use universe_core::manifest::Store;

use crate::channel::{FRAGMENT_DIRECTORY, build_tree, read_fragments, select, spec_for, verify};
use crate::git::GitTransport;
use crate::options::{Options, VerbSpec};
use crate::output::{FINDINGS, USAGE, failure, human, json_success};
use crate::project::Project;

/// Default branch name for the channel.
const DEFAULT_BRANCH: &str = "channel";

/// Run `universe publish`.
pub(crate) fn run(args: &[String]) -> CommandOutput {
    let options = match Options::parse(args, VerbSpec::publish()) {
        Ok(options) => options,
        Err(message) => {
            return failure(
                USAGE,
                &[Diagnostic::new(
                    Severity::Error,
                    "publish.usage.invalid",
                    message,
                )],
            );
        }
    };

    let project = match Project::load(&options.root) {
        Ok(project) => project,
        Err(output) => return output,
    };
    if let Err(output) = project.require_valid() {
        return output;
    }

    let Some(remote) = options.remote.clone() else {
        return failure(
            USAGE,
            &[Diagnostic::new(
                Severity::Error,
                "publish.remote.required",
                "--remote names the Git remote the channel branch lives on",
            )],
        );
    };
    let branch = options
        .branch
        .clone()
        .unwrap_or_else(|| DEFAULT_BRANCH.to_owned());

    let fragment_directory = options
        .fragments
        .clone()
        .unwrap_or_else(|| project.root.join(FRAGMENT_DIRECTORY));
    let fragments = match read_fragments(&fragment_directory) {
        Ok(fragments) => select(&fragments, &options.packages),
        Err(diagnostic) => return failure(FINDINGS, &[diagnostic]),
    };
    let verified = match verify(&project.root, &fragments) {
        Ok(verified) => verified,
        Err(diagnostic) => return failure(FINDINGS, &[diagnostic]),
    };

    let spec = spec_for(&project.manifest, options.base_url.as_deref());
    let tree = match build_tree(&verified, &spec) {
        Ok(tree) => tree,
        Err(diagnostic) => return failure(FINDINGS, &[diagnostic]),
    };

    let store = match store_for(&project, options.store.as_deref()) {
        Ok(store) => store,
        Err(diagnostic) => return failure(USAGE, &[diagnostic]),
    };
    let budgets = budgets(store);
    // Whole files are the unit of transport, so git's content addressing can skip an
    // unchanged artifact. Only a file above the blob ceiling is cut, and the ceiling is the
    // store's when it declares one.
    let tree = tree.split_oversized(budgets.blob);
    if let Err(error) = budgets.check(&tree) {
        return failure(
            FINDINGS,
            &[Diagnostic::new(
                Severity::Error,
                "publish.budget.exceeded",
                error.to_string(),
            )],
        );
    }

    let scratch = options
        .output
        .clone()
        .unwrap_or_else(|| scratch_directory(&project.root));
    let transport = GitTransport::new(&remote, &branch, &scratch);
    if options.check {
        return check(&transport, &tree, &branch, options.json);
    }
    if options.dry_run {
        return describe(&tree, &branch, options.json, budgets);
    }

    let message = message(&spec.name, &verified, &branch);
    match transport.push(&tree, &message) {
        Ok(PushOutcome::Unchanged) => report_unchanged(&tree, &branch, options.json),
        Ok(PushOutcome::Published { commit }) => {
            report_published(&tree, &branch, &commit, options.json)
        }
        Err(error) => failure(
            FINDINGS,
            &[Diagnostic::new(
                Severity::Error,
                "publish.transport.failed",
                error.to_string(),
            )],
        ),
    }
}

/// The store this publish is measured against.
///
/// A budget belongs to a destination, so a publish that names no store still has to find the
/// one it writes to. Falling back to the crate's defaults instead — which is what this verb
/// used to do — applies a manifest's ceilings only when the invocation remembered `--store`,
/// and a CI job that forgets the flag publishes a snapshot the manifest forbids. Resolution
/// therefore goes: the name given, else the single writable store that holds a branch, else
/// the defaults when the manifest declares no such store at all. Two candidates is a
/// question only the operator can answer, so it is refused rather than guessed.
fn store_for<'a>(
    project: &'a Project,
    named: Option<&str>,
) -> Result<Option<&'a Store>, Diagnostic> {
    if let Some(name) = named {
        let Some(declared) = project
            .manifest
            .stores
            .iter()
            .find(|candidate| candidate.name == name)
        else {
            return Err(Diagnostic::new(
                Severity::Error,
                "publish.store.unknown",
                format!("no store named '{name}' is declared in universe.toml"),
            ));
        };
        if !declared.kind.is_git_branch() {
            return Err(Diagnostic::new(
                Severity::Error,
                "publish.store.kind",
                format!(
                    "store '{name}' is a '{}' store, but `publish` writes a tree to a Git branch; \
                     a 'branch' or 'static' store is what `--store` may name here",
                    declared.kind.as_str()
                ),
            ));
        }
        return Ok(Some(declared));
    }

    let mut candidates = project
        .manifest
        .stores
        .iter()
        .filter(|candidate| candidate.writable && candidate.kind.is_git_branch());
    let first = candidates.next();
    let second = candidates.next();
    match (first, second) {
        (None, _) => Ok(None),
        (Some(only), None) => Ok(Some(only)),
        (Some(left), Some(right)) => Err(Diagnostic::new(
            Severity::Error,
            "publish.store.ambiguous",
            format!(
                "universe.toml declares two writable branch stores, '{}' and '{}'; \
                 pass --store to say which one this publish is measured against",
                left.name, right.name
            ),
        )),
    }
}

/// The budgets a store declares, with the crate's defaults for anything absent.
fn budgets(store: Option<&Store>) -> Budgets {
    let mut budgets = Budgets::default();
    let Some(declared) = store else {
        return budgets;
    };
    if let Some(value) = declared.budgets.blob {
        budgets.blob = value;
    }
    if let Some(value) = declared.budgets.transport {
        budgets.transport = value;
    }
    if let Some(value) = declared.budgets.staged {
        budgets.staged = value;
    }
    budgets
}

fn message(name: &str, verified: &[crate::channel::Verified], branch: &str) -> String {
    let mut text = format!("channel {name}: {} package(s)", verified.len());
    for entry in verified {
        text.push_str(&format!(
            "\n{} {} {}",
            entry.fragment.subdir, entry.fragment.package, entry.fragment.version
        ));
    }
    text.push_str(&format!("\n\nbranch: {branch}"));
    text
}

fn check(
    transport: &GitTransport,
    tree: &ChannelTree,
    branch: &str,
    json_output: bool,
) -> CommandOutput {
    let current = match transport.current() {
        Ok(current) => current,
        Err(error) => {
            return failure(
                FINDINGS,
                &[Diagnostic::new(
                    Severity::Error,
                    "publish.check.failed",
                    error.to_string(),
                )],
            );
        }
    };
    let Some(identity) = current else {
        return failure(
            FINDINGS,
            &[Diagnostic::new(
                Severity::Error,
                "publish.check.absent",
                format!("branch {branch} does not exist yet; the channel has never been published"),
            )],
        );
    };
    let staged = match transport.staged_tree(tree) {
        Ok(staged) => staged,
        Err(error) => {
            return failure(
                FINDINGS,
                &[Diagnostic::new(
                    Severity::Error,
                    "publish.check.failed",
                    error.to_string(),
                )],
            );
        }
    };
    if identity.tree != staged {
        return failure(
            FINDINGS,
            &[Diagnostic::new(
                Severity::Error,
                "publish.check.drift",
                format!(
                    "branch {branch} holds tree {} but the fragments render to {} (content digest {}); run `universe publish` to republish",
                    identity.tree,
                    staged,
                    tree.digest()
                ),
            )],
        );
    }
    if json_output {
        return json_success(
            "publish",
            json!({"check": "clean", "branch": branch, "commit": identity.commit, "tree": identity.tree}),
            Vec::new(),
            Vec::new(),
        );
    }
    human(format!(
        "branch {branch} already holds this channel ({} files, commit {})\n",
        tree.len(),
        identity.commit
    ))
}

fn describe(
    tree: &ChannelTree,
    branch: &str,
    json_output: bool,
    budgets: Budgets,
) -> CommandOutput {
    if json_output {
        return json_success(
            "publish",
            json!({
                "dry_run": true,
                "branch": branch,
                "files": tree.paths().cloned().collect::<Vec<String>>(),
                "bytes": tree.total_bytes(),
                "largest": tree.largest(),
                "budgets": {
                    "blob": budgets.blob,
                    "transport": budgets.transport,
                    "staged": budgets.staged,
                },
            }),
            Vec::new(),
            Vec::new(),
        );
    }
    let mut body = format!(
        "would publish {} file(s), {} byte(s) to branch {branch}\n",
        tree.len(),
        tree.total_bytes()
    );
    for path in tree.paths() {
        body.push_str(&format!("  {path}\n"));
    }
    human(body)
}

fn report_unchanged(tree: &ChannelTree, branch: &str, json_output: bool) -> CommandOutput {
    if json_output {
        return json_success(
            "publish",
            json!({"branch": branch, "published": false, "reason": "unchanged", "files": tree.len()}),
            Vec::new(),
            Vec::new(),
        );
    }
    human(format!(
        "branch {branch} already holds these {} file(s); nothing pushed\n",
        tree.len()
    ))
}

fn report_published(
    tree: &ChannelTree,
    branch: &str,
    commit: &str,
    json_output: bool,
) -> CommandOutput {
    if json_output {
        return json_success(
            "publish",
            json!({
                "branch": branch,
                "published": true,
                "commit": commit,
                "files": tree.len(),
                "bytes": tree.total_bytes(),
            }),
            Vec::new(),
            Vec::new(),
        );
    }
    human(format!(
        "published {} file(s), {} byte(s) to branch {branch} at {commit}\n",
        tree.len(),
        tree.total_bytes()
    ))
}

/// Where a publish stages its scratch repository.
pub(crate) fn scratch_directory(root: &Path) -> PathBuf {
    root.join("output/publish")
}
