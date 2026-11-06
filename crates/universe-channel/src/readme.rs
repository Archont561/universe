//! The channel's own README, rendered from the same fragments as its metadata.
//!
//! A channel whose usage instructions each consumer re-derives has as many readings as it has
//! consumers. This one carries the table, the digests, and the exact stanza to paste, and it
//! is a pure function of the fragment set for the same reason the metadata is: a republish
//! that changes nothing must not produce a diff here either.

use crate::channel::ChannelSpec;
use crate::fragment::Fragment;

/// Render the channel README.
#[must_use]
pub fn render(fragments: &[&Fragment], spec: &ChannelSpec) -> String {
    let mut body = String::new();
    body.push_str(&format!("# {}\n\n", spec.name));
    body.push_str(
        "A static Conda channel, published by Universe. Every file here is generated: the\n\
         metadata from the audit fragments, the README from the same fragments. Edit the\n\
         project's `universe.toml`, not this branch.\n\n",
    );

    body.push_str("## Packages\n\n");
    if fragments.is_empty() {
        body.push_str("This channel currently serves no packages.\n\n");
    } else {
        body.push_str("| subdir | package | version | build | size | sha256 |\n");
        body.push_str("| --- | --- | --- | --- | --- | --- |\n");
        for fragment in fragments {
            body.push_str(&format!(
                "| `{}` | `{}` | `{}` | `{}` | {} | `{}` |\n",
                fragment.subdir,
                fragment.package,
                fragment.version,
                fragment.build,
                fragment.size_bytes,
                fragment.digest
            ));
        }
        body.push('\n');
    }

    body.push_str("## Using it\n\n");
    match &spec.base_url {
        Some(base_url) => {
            body.push_str(&format!(
                "The channel is served from `{base_url}`. A Pixi project consumes it by URL:\n\n"
            ));
            body.push_str("```toml\n[project]\nchannels = [\"");
            body.push_str(base_url);
            body.push_str("\"]\n```\n\n");
        }
        None => {
            body.push_str(
                "This branch is a channel tree, not a URL. Serve it over HTTP(S) — a static host,\n\
                 an object store, or `python -m http.server` from a checkout — and point a Pixi\n\
                 project at wherever it ends up:\n\n",
            );
            body.push_str("```toml\n[project]\nchannels = [\"https://<host>/<path>\"]\n```\n\n");
            body.push_str(
                "A checkout of this branch is enough for a local solve: point the URL at the\n\
                 directory you checked it out into.\n\n",
            );
        }
    }

    body.push_str(
        "The digests above are the ones the audit measured. A consumer that wants to verify a\n\
         download hashes the file and compares it with the row for that package.\n\n",
    );
    if !fragments.is_empty() {
        body.push_str(
            "Every artifact also has its audit receipt in this tree:\n\
             `meta/<subdir>/<package>-<version>-<build>.conda.json`, a kilobyte of metadata —\n\
             name, version, dependencies, licence, digests — for a client that wants to know\n\
             what a package contains without downloading it.\n",
        );
    }
    body
}

#[cfg(test)]
mod tests {
    use super::render;
    use crate::channel::ChannelSpec;
    use crate::fragment::Fragment;

    fn fragment(package: &str) -> Fragment {
        Fragment {
            package: package.to_owned(),
            version: "1.0.0".to_owned(),
            build: "0".to_owned(),
            build_number: 0,
            subdir: "linux-64".to_owned(),
            digest: "a".repeat(64),
            size_bytes: 11,
            depends: Vec::new(),
            license: Some("MIT".to_owned()),
            license_files: Vec::new(),
            path: "output/example.conda".to_owned(),
        }
    }

    #[test]
    fn a_channel_with_a_url_tells_the_consumer_to_paste_that_url() {
        let spec = ChannelSpec {
            name: "fixture".to_owned(),
            subdirs: vec!["linux-64".to_owned()],
            base_url: Some("https://example.invalid/channel".to_owned()),
        };
        let packages = [fragment("a"), fragment("b")];
        let readme = render(&packages.iter().collect::<Vec<_>>(), &spec);
        assert!(
            readme.contains("https://example.invalid/channel"),
            "{readme}"
        );
        assert!(readme.contains("| `linux-64` | `a` |"), "{readme}");
        assert!(readme.contains("[project]"), "{readme}");
    }

    #[test]
    fn a_channel_without_a_url_says_how_to_serve_the_tree_it_is_in() {
        // The branch case: there is no URL to paste, so the README must not invent one.
        let spec = ChannelSpec::new("fixture", vec!["linux-64".to_owned()]);
        let readme = render(&[], &spec);
        assert!(readme.contains("Serve it over HTTP"), "{readme}");
        assert!(readme.contains("serves no packages"), "{readme}");
        assert!(!readme.contains("https://example.invalid"), "{readme}");
    }

    #[test]
    fn the_readme_carries_the_digest_of_every_package_it_lists() {
        let spec = ChannelSpec::new("fixture", vec!["linux-64".to_owned()]);
        let packages = [fragment("a")];
        let readme = render(&packages.iter().collect::<Vec<_>>(), &spec);
        assert!(readme.contains(&"a".repeat(64)), "{readme}");
    }

    #[test]
    fn the_readme_says_where_the_receipts_are() {
        let mut one = fragment("example-airlock");
        one.digest = "a".repeat(64);
        let rendered = render(
            &[&one],
            &ChannelSpec::new("fixture", vec!["linux-64".to_owned(), "noarch".to_owned()]),
        );

        assert!(
            rendered.contains("meta/<subdir>/<package>-<version>-<build>.conda.json"),
            "{rendered}"
        );
        // A channel serving nothing has no receipts to describe.
        let empty = render(&[], &ChannelSpec::new("fixture", vec!["noarch".to_owned()]));
        assert!(!empty.contains("meta/<subdir>"), "{empty}");
    }
}
