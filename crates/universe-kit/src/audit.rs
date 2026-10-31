//! `universe audit`: what the build actually left behind, measured rather than assumed.
//!
//! The receipt a later publish step consumes is produced here: path, size, and digest of
//! every byte the manifest declared. A package that publishes somewhere and produced nothing
//! fails, because a publish job with no artifact is a job that would have published the
//! previous build.
//!
//! Binary-level inspection (ELF/PE/Mach-O headers, runpaths, build-root leaks) is `UNI-10`;
//! what this pass proves is identity and integrity, and it says so rather than implying more.

use std::path::Path;

use serde_json::json;
use universe_core::diagnostic::{Diagnostic, Severity};
use universe_core::{Artifact, ArtifactId, CommandOutput, Sha256Digest, manifest::Package};

use crate::options::Options;
use crate::output::{FINDINGS, USAGE, failure, human, json_success};
use crate::project::Project;

/// Run `universe audit`.
pub(crate) fn run(args: &[String]) -> CommandOutput {
    let options = match Options::parse(args, "audit", false) {
        Ok(options) => options,
        Err(message) => {
            return failure(
                USAGE,
                &[Diagnostic::new(
                    Severity::Error,
                    "audit.usage.invalid",
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

    let mut diagnostics = Vec::new();
    let mut artifacts = Vec::new();
    let mut receipts = Vec::new();

    for package in &project.manifest.packages {
        // A package that is not released and publishes nowhere has no artifact to account
        // for; auditing it would fail every repository that vendors one input.
        if !package.release && package.publish.is_empty() {
            continue;
        }
        let found = collect(&options.root, package);
        for path in &found {
            match inspect(path, package) {
                Ok(artifact) => {
                    receipts.push(json!({
                        "package": package.name,
                        "path": relative(&options.root, path),
                        "size_bytes": artifact.size_bytes,
                        "digest": artifact.digest.as_str(),
                    }));
                    artifacts.push(artifact);
                }
                Err(diagnostic) => diagnostics.push(diagnostic),
            }
        }
        if found.is_empty() {
            diagnostics.push(
                Diagnostic::new(
                    Severity::Error,
                    "audit.output.missing",
                    format!(
                        "package '{}' is released or published but produced none of {}",
                        package.name,
                        package.outputs.join(", ")
                    ),
                )
                .with_path(format!("packages[{}].outputs", package.name)),
            );
        }
    }

    if !diagnostics.is_empty() {
        return failure(FINDINGS, &diagnostics);
    }
    if options.json {
        return json_success(
            "audit",
            json!({ "artifacts": receipts }),
            Vec::new(),
            artifacts,
        );
    }
    if receipts.is_empty() {
        return human("nothing to audit\n".to_owned());
    }
    let mut body = String::new();
    for receipt in &receipts {
        body.push_str(&format!(
            "{}: {} ({} bytes)\n",
            receipt["path"].as_str().unwrap_or_default(),
            receipt["digest"].as_str().unwrap_or_default(),
            receipt["size_bytes"].as_u64().unwrap_or_default()
        ));
    }
    human(body)
}

fn collect(root: &Path, package: &Package) -> Vec<std::path::PathBuf> {
    package
        .outputs
        .iter()
        .flat_map(|pattern| crate::paths::collect(root, pattern))
        .collect()
}

fn inspect(path: &Path, package: &Package) -> Result<Artifact, Diagnostic> {
    let bytes = std::fs::read(path).map_err(|error| {
        Diagnostic::new(
            Severity::Error,
            "audit.artifact.unreadable",
            format!("cannot read {}: {error}", path.display()),
        )
    })?;
    if bytes.is_empty() {
        return Err(Diagnostic::new(
            Severity::Error,
            "audit.artifact.empty",
            format!("{} is empty", path.display()),
        ));
    }

    let file_name = path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default();
    // Every ecosystem spells its artifacts `<name>-<version>-…`; a file that does not carry
    // its own package name is an artifact this package did not produce.
    if !file_name.starts_with(&format!("{}-", package.name)) {
        return Err(Diagnostic::new(
            Severity::Error,
            "audit.artifact.identity",
            format!(
                "{} does not carry the name of package '{}'",
                file_name, package.name
            ),
        )
        .with_path(path.display().to_string()));
    }

    let id = ArtifactId::new(
        package.name.clone(),
        version_from(&file_name, &package.name),
    )
    .map_err(|diagnostic| diagnostic.with_path(path.display().to_string()))?;
    Ok(Artifact {
        id,
        digest: Sha256Digest::parse(&universe_workflow::sha256::sha256_hex(&bytes))
            .expect("a computed digest is always 64 lowercase hex characters"),
        size_bytes: bytes.len() as u64,
        media_type: None,
    })
}

/// The version segment of a native artifact file name, `name-version-…`.
fn version_from(file_name: &str, package: &str) -> String {
    file_name
        .strip_prefix(&format!("{package}-"))
        .and_then(|rest| rest.split('-').next())
        .filter(|version| !version.is_empty() && version.chars().any(|c| c.is_ascii_digit()))
        .unwrap_or("unknown")
        .to_owned()
}

fn relative(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .into_owned()
}

#[cfg(test)]
mod tests {
    #[test]
    fn the_version_segment_follows_the_package_name() {
        assert_eq!(
            super::version_from(
                "universe-airlock-0.1.0-h1b2c3d4_0.conda",
                "universe-airlock"
            ),
            "0.1.0"
        );
        assert_eq!(
            super::version_from("other-1.0-0.conda", "package"),
            "unknown"
        );
    }
}
