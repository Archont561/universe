//! `universe audit`: what the build actually left behind, measured rather than assumed.
//!
//! Two passes, and the second is the one that catches a release defect. The first measures
//! identity and integrity: size, digest, and whether the file is even named after the package
//! that claims it. The second opens the archive and reads what it says about itself — the
//! index record a solver will act on, and the licence a consumer is entitled to rely on.
//!
//! Native binary inspection (ELF/PE/Mach-O headers, runpaths, build-root leaks) lives in
//! `universe-conda` and is applied by the rules declared in `universe.toml`'s `[audit]`
//! section. The receipt this pass emits is the deterministic fragment a catalogue is built
//! from, which is why `--emit-fragment` writes it rather than leaving it to a second tool.

use std::path::Path;

use serde_json::{Map, Value, json};
use universe_conda::{LicenceRules, open_package};
use universe_core::diagnostic::{Diagnostic, Severity};
use universe_core::manifest::{Package, PackageKind};
use universe_core::{Artifact, ArtifactId, CommandOutput, Sha256Digest};

use crate::options::{Options, VerbSpec};
use crate::output::{FINDINGS, USAGE, failure, human, json_success};
use crate::project::Project;

/// Run `universe audit`.
pub(crate) fn run(args: &[String]) -> CommandOutput {
    let options = match Options::parse(args, VerbSpec::audit()) {
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
            continue;
        }
        for path in &found {
            match inspect(&options.root, path, package, &project) {
                Ok((artifact, receipt)) => {
                    artifacts.push(artifact);
                    receipts.push(receipt);
                }
                Err(finding) => diagnostics.push(finding),
            }
        }
    }

    if !diagnostics.is_empty() {
        return failure(FINDINGS, &diagnostics);
    }

    if let Some(directory) = &options.fragment_directory
        && let Err(error) = write_fragments(directory, &receipts)
    {
        return failure(
            USAGE,
            &[Diagnostic::new(
                Severity::Error,
                "audit.fragment.write_failed",
                error,
            )],
        );
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

/// Measure one artifact, and return the receipt a catalogue is built from.
fn inspect(
    root: &Path,
    path: &Path,
    package: &Package,
    project: &Project,
) -> Result<(Artifact, Value), Diagnostic> {
    let relative = relative(root, path);
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
        .with_path(relative.clone()));
    }

    let mut receipt = Map::new();
    receipt.insert("path".to_owned(), json!(relative));
    receipt.insert("package".to_owned(), json!(package.name));

    match package.kind {
        PackageKind::Conda if file_name.ends_with(".conda") => {
            audit_conda(path, package, project, &relative, &mut receipt)?
        }
        _ => {
            let bytes = std::fs::read(path).map_err(|error| unreadable(&relative, &error))?;
            seal(&bytes, package, &file_name, &relative, &mut receipt)?;
        }
    }

    let digest = receipt
        .get("digest")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_owned();
    let size_bytes = receipt
        .get("size_bytes")
        .and_then(Value::as_u64)
        .unwrap_or_default();
    let version = receipt
        .get("version")
        .and_then(Value::as_str)
        .unwrap_or("unknown")
        .to_owned();

    let artifact = Artifact {
        id: ArtifactId::new(package.name.clone(), version)
            .map_err(|diagnostic| diagnostic.with_path(relative.clone()))?,
        digest: Sha256Digest::parse(&digest).map_err(|error| {
            Diagnostic::new(Severity::Error, "audit.digest.invalid", error.to_string())
        })?,
        size_bytes,
        media_type: None,
    };
    Ok((artifact, Value::Object(receipt)))
}

/// The Conda pass: the archive's own index record, and the licence it ships.
fn audit_conda(
    path: &Path,
    package: &Package,
    project: &Project,
    relative: &str,
    receipt: &mut Map<String, Value>,
) -> Result<(), Diagnostic> {
    let opened = open_package(path).map_err(|error| {
        Diagnostic::new(
            Severity::Error,
            "audit.package.unreadable",
            format!("{relative}: {error}"),
        )
        .with_path(relative.to_owned())
    })?;

    let index = opened.index();
    if index.name != package.name {
        return Err(Diagnostic::new(
            Severity::Error,
            "audit.package.identity",
            format!(
                "{relative} is package '{}' inside the archive but '{}' in the manifest",
                index.name, package.name
            ),
        )
        .with_path(relative.to_owned()));
    }

    // The digest and size come from the archive rather than a second read of the file.
    let artifact = opened.artifact();
    receipt.insert("version".to_owned(), json!(index.version));
    receipt.insert("build".to_owned(), json!(index.build));
    receipt.insert("build_number".to_owned(), json!(index.build_number));
    receipt.insert("subdir".to_owned(), json!(index.subdir));
    receipt.insert("size_bytes".to_owned(), json!(artifact.size_bytes));
    receipt.insert("digest".to_owned(), json!(artifact.digest.as_str()));
    receipt.insert("depends".to_owned(), json!(sorted(&index.depends)));

    let licence = universe_conda::audit_licence(
        &opened,
        LicenceRules {
            enforce_license_file: project.manifest.audit.enforce_license_file,
            require_valid_spdx: project.manifest.audit.require_valid_spdx,
        },
    )
    .map_err(|error| {
        Diagnostic::new(
            Severity::Error,
            "audit.licence.unreadable",
            format!("{relative}: {error}"),
        )
        .with_path(relative.to_owned())
    })?;

    receipt.insert(
        "license".to_owned(),
        json!(licence.expression().unwrap_or_default()),
    );
    receipt.insert("license_files".to_owned(), json!(licence.files()));

    if let Some(finding) = licence.violations().first() {
        return Err(Diagnostic::new(
            finding.severity(),
            finding.code(),
            format!("{relative}: {}", finding.message()),
        )
        .with_path(relative.to_owned()));
    }
    Ok(())
}

/// The generic pass for an artifact this build cannot open.
fn seal(
    bytes: &[u8],
    package: &Package,
    file_name: &str,
    relative: &str,
    receipt: &mut Map<String, Value>,
) -> Result<(), Diagnostic> {
    if bytes.is_empty() {
        return Err(Diagnostic::new(
            Severity::Error,
            "audit.artifact.empty",
            format!("{relative} is empty"),
        )
        .with_path(relative.to_owned()));
    }
    receipt.insert(
        "version".to_owned(),
        json!(version_from(file_name, &package.name)),
    );
    receipt.insert("size_bytes".to_owned(), json!(bytes.len()));
    receipt.insert(
        "digest".to_owned(),
        json!(universe_workflow::sha256::sha256_hex(bytes)),
    );
    Ok(())
}

fn unreadable(relative: &str, error: &std::io::Error) -> Diagnostic {
    Diagnostic::new(
        Severity::Error,
        "audit.artifact.unreadable",
        format!("cannot read {relative}: {error}"),
    )
    .with_path(relative.to_owned())
}

/// Write the deterministic fragment a catalogue reads, one file per artifact.
///
/// The layout is `<directory>/<subdir>/<file name>.json`, so the subdirectory is a property of
/// the path rather than of the document, and two builds of the same package overwrite rather
/// than accumulate.
fn write_fragments(directory: &Path, receipts: &[Value]) -> Result<(), String> {
    for receipt in receipts {
        let segment = receipt
            .get("subdir")
            .and_then(Value::as_str)
            .unwrap_or("noarch");
        let file_name = receipt
            .get("path")
            .and_then(Value::as_str)
            .and_then(|path| path.rsplit('/').next())
            .unwrap_or("artifact");
        let target = directory.join(segment);
        std::fs::create_dir_all(&target)
            .map_err(|error| format!("cannot create {}: {error}", target.display()))?;
        let document = serde_json::to_string_pretty(receipt)
            .map_err(|error| format!("cannot serialize a fragment: {error}"))?;
        let path = target.join(format!("{file_name}.json"));
        std::fs::write(&path, format!("{document}\n"))
            .map_err(|error| format!("cannot write {}: {error}", path.display()))?;
    }
    Ok(())
}

fn sorted(values: &[String]) -> Vec<String> {
    let mut sorted = values.to_vec();
    sorted.sort();
    sorted
}

fn relative(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .into_owned()
}

/// The version segment of a native artifact file name, `name-version-…`.
fn version_from(file_name: &str, package: &str) -> String {
    // The extension is dropped first: splitting on `-` alone would leave `.crate` on the end
    // of a version that has no build string of its own.
    let stem = file_name
        .rsplit_once('.')
        .map_or(file_name, |(stem, _)| stem);
    stem.strip_prefix(&format!("{package}-"))
        .and_then(|rest| rest.split('-').next())
        .filter(|version| !version.is_empty() && version.chars().any(|c| c.is_ascii_digit()))
        .unwrap_or("unknown")
        .to_owned()
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
