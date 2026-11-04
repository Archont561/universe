//! Reading fragments, proving them against their bytes, and assembling the tree.
//!
//! This is the part `index` and `publish` share, and it is written once so that the publisher
//! cannot verify less than the checker. Every fragment is measured against the artifact it
//! names before it reaches a channel: a digest that disagrees with the bytes beside it stops
//! publication instead of writing a channel a solver will choke on later.

use std::path::{Path, PathBuf};

use universe_channel::channel::{ChannelSpec, render};
use universe_channel::fragment::{Fragment, parse};
use universe_channel::transport::ChannelTree;
use universe_core::diagnostic::{Diagnostic, Severity};
use universe_core::manifest::Manifest;

/// Default directory the audit verb writes fragments to.
pub(crate) const FRAGMENT_DIRECTORY: &str = "output/fragments";

/// A fragment plus the bytes it describes.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct Verified {
    /// The fragment.
    pub(crate) fragment: Fragment,
    /// Its bytes, read from the project root.
    pub(crate) bytes: Vec<u8>,
}

/// Read every fragment under `directory`, recursively, in a stable order.
pub(crate) fn read_fragments(directory: &Path) -> Result<Vec<Fragment>, Diagnostic> {
    let mut paths = Vec::new();
    collect(directory, &mut paths);
    paths.sort();

    let mut fragments = Vec::new();
    for path in paths {
        let text = std::fs::read_to_string(&path).map_err(|error| {
            Diagnostic::new(
                Severity::Error,
                "channel.fragment.unreadable",
                format!("cannot read {}: {error}", path.display()),
            )
        })?;
        let fragment = parse(&text).map_err(|error| {
            Diagnostic::new(
                Severity::Error,
                "channel.fragment.invalid",
                format!("{}: {error}", path.display()),
            )
        })?;
        fragments.push(fragment);
    }
    if fragments.is_empty() {
        return Err(Diagnostic::new(
            Severity::Error,
            "channel.fragment.none",
            format!(
                "no fragments under {}; run `universe audit --emit-fragment` first",
                directory.display()
            ),
        ));
    }
    Ok(fragments)
}

fn collect(directory: &Path, found: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(directory) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect(&path, found);
        } else if path
            .extension()
            .is_some_and(|extension| extension == "json")
        {
            found.push(path);
        }
    }
}

/// Prove every fragment against the bytes it names.
///
/// # Errors
///
/// Returns the first fragment whose artifact is missing, whose size differs, or whose digest
/// differs — naming the file, because "a digest mismatched" without a name is a search.
pub(crate) fn verify(root: &Path, fragments: &[Fragment]) -> Result<Vec<Verified>, Diagnostic> {
    let mut verified = Vec::new();
    for fragment in fragments {
        let problems = fragment.problems();
        if !problems.is_empty() {
            return Err(Diagnostic::new(
                Severity::Error,
                "channel.fragment.unusable",
                format!(
                    "the fragment for {} cannot describe a channel entry: {}",
                    fragment.package,
                    problems.join("; ")
                ),
            ));
        }
        if fragment.path.trim().is_empty() {
            return Err(Diagnostic::new(
                Severity::Error,
                "channel.fragment.path",
                format!(
                    "the fragment for {} does not say where its bytes are",
                    fragment.package
                ),
            ));
        }
        let path = root.join(&fragment.path);
        let bytes = std::fs::read(&path).map_err(|error| {
            Diagnostic::new(
                Severity::Error,
                "channel.artifact.missing",
                format!(
                    "{} is named by the fragment for {} but cannot be read: {error}",
                    fragment.path, fragment.package
                ),
            )
        })?;
        if bytes.len() as u64 != fragment.size_bytes {
            return Err(Diagnostic::new(
                Severity::Error,
                "channel.artifact.size",
                format!(
                    "{} is {} bytes but its fragment says {}",
                    fragment.path,
                    bytes.len(),
                    fragment.size_bytes
                ),
            ));
        }
        let digest = universe_workflow::sha256::sha256_hex(&bytes);
        if digest != fragment.digest {
            return Err(Diagnostic::new(
                Severity::Error,
                "channel.artifact.digest",
                format!(
                    "{} hashes to {digest} but its fragment says {}",
                    fragment.path, fragment.digest
                ),
            ));
        }
        verified.push(Verified {
            fragment: fragment.clone(),
            bytes,
        });
    }
    Ok(verified)
}

/// The channel a project's manifest describes.
pub(crate) fn spec_for(manifest: &Manifest, base_url: Option<&str>) -> ChannelSpec {
    let mut subdirs = manifest.project.platforms.clone();
    // `noarch` is always served: a consumer's solve asks for it whether or not this project
    // has ever built one, and a 404 is not an answer a solver handles gracefully.
    if !subdirs.iter().any(|subdir| subdir == "noarch") {
        subdirs.push("noarch".to_owned());
    }
    ChannelSpec {
        name: manifest.project.name.clone(),
        subdirs,
        base_url: base_url.map(str::to_owned),
    }
}

/// Assemble the tree: metadata, package bytes, and the README.
///
/// # Errors
///
/// Returns the renderer's finding when a fragment cannot describe a channel entry.
pub(crate) fn build_tree(
    verified: &[Verified],
    spec: &ChannelSpec,
) -> Result<ChannelTree, Diagnostic> {
    let fragments: Vec<Fragment> = verified
        .iter()
        .map(|entry| entry.fragment.clone())
        .collect();
    let rendered = render(&fragments, spec).map_err(|error| {
        Diagnostic::new(Severity::Error, "channel.render.failed", error.to_string())
    })?;

    let mut tree = ChannelTree::default();
    for file in rendered {
        tree.insert(file.path, file.bytes);
    }
    for entry in verified {
        // The bytes go to the canonical subdir path, so a native client can download what the
        // metadata advertises without knowing anything about this repository.
        tree.insert(entry.fragment.channel_path(), entry.bytes.clone());
    }
    Ok(tree)
}

/// The fragment set a publish would ship: every fragment if `packages` is empty, otherwise
/// only those for the named packages.
pub(crate) fn select(fragments: &[Fragment], packages: &[String]) -> Vec<Fragment> {
    if packages.is_empty() {
        return fragments.to_vec();
    }
    fragments
        .iter()
        .filter(|fragment| packages.contains(&fragment.package))
        .cloned()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{build_tree, read_fragments, select, spec_for, verify};
    use universe_channel::channel::ChannelSpec;
    use universe_channel::fragment::Fragment;
    use universe_core::manifest::Manifest;

    const MANIFEST: &str = r#"
schema = 1

[project]
name = "fixture-channel"
platforms = ["linux-64"]

[[packages]]
name = "example-airlock"
kind = "conda"
command = ["cargo", "build"]
outputs = ["output/**/*.conda"]
smoke = ["example-airlock --version"]
publish = ["channel-branch"]

[[stores]]
name = "channel-branch"
kind = "branch"
url = "https://example.invalid/universe.git"
writable = true
"#;

    fn fragment(digest: &str, size: u64, path: &str) -> Fragment {
        Fragment {
            package: "example-airlock".to_owned(),
            version: "1.0.0".to_owned(),
            build: "0".to_owned(),
            build_number: 0,
            subdir: "linux-64".to_owned(),
            digest: digest.to_owned(),
            size_bytes: size,
            depends: Vec::new(),
            license: Some("MIT".to_owned()),
            license_files: Vec::new(),
            path: path.to_owned(),
        }
    }

    /// A root holding the artifact the fragment names.
    fn root_with(bytes: &[u8]) -> tempfile::TempDir {
        let directory = tempfile::tempdir().expect("temp dir");
        std::fs::create_dir_all(directory.path().join("output")).expect("output");
        std::fs::write(directory.path().join("output/example.conda"), bytes).expect("artifact");
        directory
    }

    fn code(result: Result<Vec<super::Verified>, universe_core::diagnostic::Diagnostic>) -> String {
        result.expect_err("verification fails").code().to_owned()
    }

    #[test]
    fn a_fragment_whose_size_disagrees_with_its_file_is_refused() {
        let root = root_with(b"conda bytes");
        let digest = universe_workflow::sha256::sha256_hex(b"conda bytes");
        assert_eq!(
            code(verify(
                root.path(),
                &[fragment(&digest, 3, "output/example.conda")]
            )),
            "channel.artifact.size"
        );
    }

    #[test]
    fn a_fragment_that_names_no_path_is_refused() {
        let root = root_with(b"conda bytes");
        let digest = universe_workflow::sha256::sha256_hex(b"conda bytes");
        assert_eq!(
            code(verify(root.path(), &[fragment(&digest, 11, " ")])),
            "channel.fragment.path"
        );
    }

    #[test]
    fn a_fragment_that_could_not_describe_a_package_is_refused_before_reading_disk() {
        let root = root_with(b"conda bytes");
        // A zero size is not a package a solver can fetch, whatever is on disk.
        assert_eq!(
            code(verify(
                root.path(),
                &[fragment(&"a".repeat(64), 0, "output/example.conda")]
            )),
            "channel.fragment.unusable"
        );
    }

    #[test]
    fn reading_a_directory_that_holds_no_fragments_is_an_error_that_says_what_to_run() {
        let directory = tempfile::tempdir().expect("temp dir");
        let diagnostic = read_fragments(directory.path()).expect_err("no fragments");
        assert_eq!(diagnostic.code(), "channel.fragment.none");
        assert!(diagnostic.message().contains("audit --emit-fragment"));
    }

    #[test]
    fn a_fragment_that_is_not_json_names_the_file_it_came_from() {
        let directory = tempfile::tempdir().expect("temp dir");
        std::fs::write(directory.path().join("broken.json"), "not json").expect("fragment");
        let diagnostic = read_fragments(directory.path()).expect_err("invalid fragment");
        assert_eq!(diagnostic.code(), "channel.fragment.invalid");
        assert!(diagnostic.message().contains("broken.json"));
    }

    #[test]
    fn the_channel_always_serves_noarch_and_every_declared_platform() {
        let manifest = Manifest::from_toml(MANIFEST).expect("manifest parses");
        let spec = spec_for(&manifest, Some("https://example.invalid/channel"));
        assert_eq!(spec.subdirs, ["linux-64", "noarch"]);
        assert_eq!(spec.name, "fixture-channel");
        assert_eq!(
            spec.base_url.as_deref(),
            Some("https://example.invalid/channel")
        );
    }

    #[test]
    fn a_fragment_for_a_subdir_the_channel_does_not_serve_stops_the_render() {
        let root = root_with(b"conda bytes");
        let digest = universe_workflow::sha256::sha256_hex(b"conda bytes");
        let verified = verify(
            root.path(),
            &[fragment(&digest, 11, "output/example.conda")],
        )
        .expect("verification passes");
        // `osx-arm64` is not one of the manifest's platforms, so serving it would advertise a
        // channel that half exists.
        let spec = ChannelSpec::new("fixture-channel", vec!["osx-arm64".to_owned()]);
        let diagnostic = build_tree(&verified, &spec).expect_err("undeclared subdir");
        assert_eq!(diagnostic.code(), "channel.render.failed");
        assert!(diagnostic.message().contains("linux-64"));
    }

    #[test]
    fn selecting_packages_keeps_only_the_named_ones_and_an_empty_selection_keeps_all() {
        let digest = "a".repeat(64);
        let fragments = vec![
            fragment(&digest, 1, "output/a.conda"),
            Fragment {
                package: "other-airlock".to_owned(),
                ..fragment(&digest, 1, "output/b.conda")
            },
        ];
        assert_eq!(select(&fragments, &[]).len(), 2);
        let narrowed = select(&fragments, &["other-airlock".to_owned()]);
        assert_eq!(narrowed.len(), 1);
        assert_eq!(narrowed[0].package, "other-airlock");
    }
}
