//! A static Conda channel, rendered from fragments.
//!
//! The tree this produces is what a solver reads: one `repodata.json` per subdir, the
//! `channeldata.json` that names the subdirs, `run_exports.json`, and the marker files a
//! static host wants. Everything is a pure function of the fragment set, which is why two
//! renders of the same artifacts are byte-identical and a republish can be proven to be a
//! no-op rather than hoped to be.
//!
//! Two deliberate omissions:
//!
//! - **No `timestamp`.** Conda's repodata usually carries a build timestamp, and copying one
//!   would make every publish differ from the last. The channel's identity is its content.
//! - **No `md5`.** The auditor measures SHA-256; writing a second digest that nobody computed
//!   here would be a claim this code cannot stand behind.

use std::collections::BTreeMap;

use serde_json::{Value, json};

use crate::fragment::Fragment;

/// How many bytes a blob may be before a transport splits it.
///
/// 95 MiB, not 100: GitHub refuses a blob above 100 MiB outright, and the margin is what
/// keeps a split decision from being a push failure.
pub const BLOB_CEILING: u64 = 95 * 1024 * 1024;

/// Total bytes one publish may move.
pub const TRANSPORT_CEILING: u64 = 2048 * 1024 * 1024;

/// Bytes one staged push may hold.
pub const STAGED_CEILING: u64 = 2048 * 1024 * 1024;

/// What a channel is called and which subdirs it serves.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ChannelSpec {
    /// Channel name, as a consumer pastes it into their manifest.
    pub name: String,
    /// Subdirs the channel advertises. Every one gets metadata, even when empty, because a
    /// solver that asks for `noarch` on a channel that has none gets a 404 rather than an
    /// empty answer.
    pub subdirs: Vec<String>,
    /// Absolute URL the channel is served from, when one exists. It is written into
    /// `repodata.json` as `base_url` so a solver can fetch packages from a mirror.
    pub base_url: Option<String>,
}

impl ChannelSpec {
    /// A channel serving one subdir.
    #[must_use]
    pub fn new(name: impl Into<String>, subdirs: Vec<String>) -> Self {
        Self {
            name: name.into(),
            subdirs,
            base_url: None,
        }
    }
}

/// One file of the rendered channel.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ChannelFile {
    /// Path inside the channel, slash-separated.
    pub path: String,
    /// The bytes.
    pub bytes: Vec<u8>,
}

/// Why a channel could not be rendered.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum ChannelError {
    /// A fragment does not describe something a channel can publish.
    #[error("fragment for {package} is unusable: {}", problems.join("; "))]
    InvalidFragment {
        /// The package the fragment names.
        package: String,
        /// Every problem with it, in field order.
        problems: Vec<String>,
    },

    /// Two fragments claim the same channel path.
    #[error("two fragments claim {path}, so one of them would be lost")]
    Duplicate {
        /// The contested path.
        path: String,
    },

    /// A fragment belongs to a subdir the channel does not serve.
    #[error("fragment for {package} is for subdir '{subdir}', which the channel does not serve (serving: {})", served.join(", "))]
    UndeclaredSubdir {
        /// The package the fragment names.
        package: String,
        /// The subdir it claims.
        subdir: String,
        /// The subdirs the channel serves.
        served: Vec<String>,
    },
}

/// Render the channel's own files — metadata, markers, and the consumer's README.
///
/// The package bytes themselves are not here: they live wherever the audit found them, and
/// copying them is the publisher's job, because a pure function has no business reading disk.
///
/// # Errors
///
/// Returns [`ChannelError`] for an unusable fragment, two fragments claiming one path, or a
/// fragment for a subdir the channel does not serve.
pub fn render(
    fragments: &[Fragment],
    spec: &ChannelSpec,
) -> Result<Vec<ChannelFile>, ChannelError> {
    let mut sorted: Vec<&Fragment> = fragments.iter().collect();
    // Sorted by channel path, so the tree never depends on the order the fragments were read.
    sorted.sort_by_key(|fragment| fragment.channel_path());

    let mut seen: BTreeMap<String, &Fragment> = BTreeMap::new();
    for fragment in &sorted {
        let problems = fragment.problems();
        if !problems.is_empty() {
            return Err(ChannelError::InvalidFragment {
                package: fragment.package.clone(),
                problems,
            });
        }
        if !spec.subdirs.contains(&fragment.subdir) {
            return Err(ChannelError::UndeclaredSubdir {
                package: fragment.package.clone(),
                subdir: fragment.subdir.clone(),
                served: spec.subdirs.clone(),
            });
        }
        if let Some(existing) = seen.insert(fragment.channel_path(), fragment)
            && existing.digest != fragment.digest
        {
            // The same file name with different bytes is the one duplication a channel must
            // never resolve silently: the solver would take whichever came last.
            return Err(ChannelError::Duplicate {
                path: fragment.channel_path(),
            });
        }
    }

    let mut files = Vec::new();
    for subdir in &spec.subdirs {
        let entries: Vec<&Fragment> = sorted
            .iter()
            .copied()
            .filter(|fragment| &fragment.subdir == subdir)
            .collect();
        files.push(ChannelFile {
            path: format!("{subdir}/repodata.json"),
            bytes: json_bytes(&repodata(&entries, subdir, spec.base_url.as_deref())),
        });
    }

    files.push(ChannelFile {
        path: "channeldata.json".to_owned(),
        bytes: json_bytes(&channeldata(&sorted, spec)),
    });
    files.push(ChannelFile {
        path: "run_exports.json".to_owned(),
        bytes: json_bytes(&json!({ "channeldata_version": 1, "packages": {} })),
    });
    // A static host that runs Jekyll would drop every path starting with an underscore; the
    // marker is cheaper than remembering that at deploy time.
    files.push(ChannelFile {
        path: ".nojekyll".to_owned(),
        bytes: Vec::new(),
    });
    files.push(ChannelFile {
        path: "README.md".to_owned(),
        bytes: crate::readme::render(&sorted, spec).into_bytes(),
    });

    files.sort_by(|left, right| left.path.cmp(&right.path));
    Ok(files)
}

/// CEP-15 `repodata.json` for one subdir.
fn repodata(fragments: &[&Fragment], subdir: &str, base_url: Option<&str>) -> Value {
    let mut packages = serde_json::Map::new();
    for fragment in fragments {
        packages.insert(
            fragment.file_name(),
            json!({
                "name": fragment.package,
                "version": fragment.version,
                "build": fragment.build,
                "build_number": fragment.build_number,
                "subdir": fragment.subdir,
                "depends": fragment.depends,
                "license": fragment.license,
                "sha256": fragment.digest,
                "size": fragment.size_bytes,
            }),
        );
    }

    let mut info = serde_json::Map::new();
    info.insert("subdir".to_owned(), json!(subdir));
    if let Some(base_url) = base_url {
        info.insert("base_url".to_owned(), json!(base_url));
    }

    json!({
        "info": Value::Object(info),
        "packages": {},
        "packages.conda": Value::Object(packages),
        "removed": [],
        "repodata_version": 1,
    })
}

/// `channeldata.json`, the record that names what the channel serves.
fn channeldata(fragments: &[&Fragment], spec: &ChannelSpec) -> Value {
    let mut packages = serde_json::Map::new();
    for fragment in fragments {
        let entry = packages.entry(fragment.package.clone()).or_insert_with(|| {
            json!({
                "subdirs": [],
                "version": fragment.version,
                "build": fragment.build,
                "depends": fragment.depends,
                "license": fragment.license,
                "name": fragment.package,
            })
        });
        if let Some(subdirs) = entry.get_mut("subdirs").and_then(Value::as_array_mut)
            && !subdirs
                .iter()
                .any(|subdir| subdir == &json!(fragment.subdir))
        {
            subdirs.push(json!(fragment.subdir));
        }
    }

    json!({
        "channeldata_version": 1,
        "name": spec.name,
        "subdirs": spec.subdirs,
        "packages": Value::Object(packages),
        "run_exports": {},
    })
}

fn json_bytes(value: &Value) -> Vec<u8> {
    // Two-space indentation and a trailing newline: a channel is read by tools, but it is
    // also reviewed by people in a diff, and a single line per megabyte of repodata is not
    // reviewable. serde_json writes map keys in insertion order, and every map here is built
    // from sorted input.
    let mut text = serde_json::to_string_pretty(value).expect("a JSON value always serializes");
    text.push('\n');
    text.into_bytes()
}

#[cfg(test)]
mod tests {
    use super::{ChannelError, ChannelSpec, render};
    use crate::fragment::Fragment;

    fn fragment(package: &str, subdir: &str, digest: &str) -> Fragment {
        Fragment {
            package: package.to_owned(),
            version: "0.1.0".to_owned(),
            build: "h1234567_0".to_owned(),
            build_number: 0,
            subdir: subdir.to_owned(),
            digest: digest.to_owned(),
            size_bytes: 7,
            depends: Vec::new(),
            license: Some("MIT".to_owned()),
            license_files: Vec::new(),
            path: String::new(),
        }
    }

    const DIGEST: &str = "9f2c1b0a3d4e5f60718293a4b5c6d7e8f90a1b2c3d4e5f60718293a4b5c6d7e8";

    fn spec() -> ChannelSpec {
        ChannelSpec::new(
            "archont561",
            vec!["linux-64".to_owned(), "noarch".to_owned()],
        )
    }

    #[test]
    fn every_served_subdir_gets_metadata_even_when_it_is_empty() {
        let files = render(&[fragment("p", "linux-64", DIGEST)], &spec()).expect("renders");
        let paths: Vec<&str> = files.iter().map(|file| file.path.as_str()).collect();
        assert!(paths.contains(&"noarch/repodata.json"));
        assert!(paths.contains(&"channeldata.json"));
        assert!(paths.contains(&".nojekyll"));
    }

    #[test]
    fn a_duplicate_path_with_different_bytes_is_refused() {
        let other = "1111111111111111111111111111111111111111111111111111111111111111";
        let error = render(
            &[
                fragment("p", "linux-64", DIGEST),
                fragment("p", "linux-64", other),
            ],
            &spec(),
        )
        .expect_err("refused");
        assert_eq!(
            error,
            ChannelError::Duplicate {
                path: "linux-64/p-0.1.0-h1234567_0.conda".to_owned()
            }
        );
    }

    #[test]
    fn the_same_fragment_twice_is_not_a_duplicate() {
        assert!(
            render(
                &[
                    fragment("p", "linux-64", DIGEST),
                    fragment("p", "linux-64", DIGEST)
                ],
                &spec()
            )
            .is_ok()
        );
    }

    #[test]
    fn a_subdir_the_channel_does_not_serve_is_refused_by_name() {
        let error = render(&[fragment("p", "osx-arm64", DIGEST)], &spec()).expect_err("refused");
        assert!(matches!(error, ChannelError::UndeclaredSubdir { .. }));
    }
}

#[cfg(test)]
mod permutation_tests {
    use super::{ChannelSpec, render};
    use crate::fragment::Fragment;

    fn fragment(package: &str, subdir: &str) -> Fragment {
        Fragment {
            package: package.to_owned(),
            version: "1.0.0".to_owned(),
            build: "0".to_owned(),
            build_number: 0,
            subdir: subdir.to_owned(),
            digest: format!("{:064x}", package.bytes().next().unwrap_or(0) as u64),
            size_bytes: 11,
            depends: Vec::new(),
            license: Some("MIT".to_owned()),
            license_files: Vec::new(),
            path: format!("output/{package}.conda"),
        }
    }

    proptest::proptest! {
        // A channel is a pure function of the fragments it describes, so the order the audit
        // happened to find them in must not reach the bytes a solver downloads.
        #[test]
        fn any_order_of_the_same_fragments_renders_the_same_tree(order in proptest::sample::subsequence(
            vec![0_usize, 1, 2, 3], 4
        ), rotation in 0_usize..4) {
            let packages = ["a", "b", "c", "d"];
            let mut fragments: Vec<Fragment> = order
                .iter()
                .map(|index| fragment(packages[*index], "linux-64"))
                .collect();
            if fragments.len() < 2 {
                return Ok(());
            }
            let shift = rotation % fragments.len();
            fragments.rotate_left(shift);
            let spec = ChannelSpec::new("fixture", vec!["linux-64".to_owned()]);
            let first = render(&fragments, &spec).expect("renders");

            fragments.reverse();
            let second = render(&fragments, &spec).expect("renders");

            proptest::prop_assert_eq!(first, second);
        }
    }
}
