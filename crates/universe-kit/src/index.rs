//! `universe index`: render a channel tree into a directory, writing nothing else.
//!
//! This is publication minus the transport. It exists because the thing most worth checking
//! about a channel — that a solver can read it — should be checkable without a remote, a
//! branch, or a credential, and because a static host that is not Git needs the same tree.

use std::path::Path;

use serde_json::json;
use universe_channel::transport::ChannelTree;
use universe_core::CommandOutput;
use universe_core::diagnostic::{Diagnostic, Severity};

use crate::channel::{FRAGMENT_DIRECTORY, build_tree, read_fragments, select, spec_for, verify};
use crate::options::{Options, VerbSpec};
use crate::output::{FINDINGS, USAGE, failure, human, json_success};
use crate::project::Project;

/// Run `universe index`.
pub(crate) fn run(args: &[String]) -> CommandOutput {
    let options = match Options::parse(args, VerbSpec::index()) {
        Ok(options) => options,
        Err(message) => {
            return failure(
                USAGE,
                &[Diagnostic::new(
                    Severity::Error,
                    "index.usage.invalid",
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

    let Some(directory) = options.output.clone() else {
        return failure(
            USAGE,
            &[Diagnostic::new(
                Severity::Error,
                "index.output.required",
                "--output names the directory the channel is written to",
            )],
        );
    };

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

    let written = write_tree(&directory, &tree);
    let written = match written {
        Ok(paths) => paths,
        Err(error) => {
            return failure(
                USAGE,
                &[Diagnostic::new(
                    Severity::Error,
                    "index.write.failed",
                    error,
                )],
            );
        }
    };

    if options.json {
        return json_success(
            "index",
            json!({
                "output": directory.display().to_string(),
                "files": written,
                "packages": verified.len(),
            }),
            Vec::new(),
            Vec::new(),
        );
    }
    let mut body = format!(
        "{} file(s) written to {} for {} package(s)\n",
        written.len(),
        directory.display(),
        verified.len()
    );
    for path in &written {
        body.push_str(&format!("  {path}\n"));
    }
    human(body)
}

fn write_tree(directory: &Path, tree: &ChannelTree) -> Result<Vec<String>, String> {
    let mut written = Vec::new();
    for (path, bytes) in tree.files() {
        let target = directory.join(path);
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|error| format!("cannot create {}: {error}", parent.display()))?;
        }
        std::fs::write(&target, bytes)
            .map_err(|error| format!("cannot write {}: {error}", target.display()))?;
        written.push(path.clone());
    }
    Ok(written)
}
