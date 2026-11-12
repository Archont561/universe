//! The options every producer verb shares.
//!
//! One parser rather than one per verb: `--root` meaning the project root, `--json` meaning
//! the envelope, and an unknown flag being a usage error are the same promises everywhere, and
//! four parsers would let them drift. What differs between verbs is *which* flags exist, and
//! that difference is a value rather than a parameter list, so a verb cannot grow a flag
//! nobody declared.

use std::path::PathBuf;

/// The flags one verb accepts.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct VerbSpec {
    /// The verb's name, as a caller types it and as its usage line reads.
    pub(crate) verb: &'static str,
    /// `--platform <subdir>`.
    pub(crate) platform: bool,
    /// `--emit-fragment <dir>`.
    pub(crate) fragment: bool,
    /// `--changed-only` and `--base-ref <ref>`.
    pub(crate) change_scope: bool,
    /// `--timeout <seconds>` and `--receipts <dir>`.
    pub(crate) build_controls: bool,
    /// `--fragments <dir>` and `--output <dir>`.
    pub(crate) channel_inputs: bool,
    /// `--base-url <url>` and `--package <name>` (repeatable).
    pub(crate) channel_shape: bool,
    /// `--remote <name>`, `--branch <name>`, `--keep <n>`, `--check`, and `--dry-run`.
    pub(crate) transport: bool,
    /// `--force`, and the `--store`/`--dry-run` an upload needs.
    pub(crate) upload: bool,
}

impl VerbSpec {
    /// What `universe plan` accepts.
    pub(crate) const fn plan() -> Self {
        Self {
            verb: "plan",
            platform: true,
            fragment: false,
            change_scope: true,
            build_controls: false,
            channel_inputs: false,
            channel_shape: false,
            transport: false,
            upload: false,
        }
    }

    /// What `universe lint` accepts.
    pub(crate) const fn lint() -> Self {
        Self {
            verb: "lint",
            platform: false,
            fragment: false,
            change_scope: false,
            build_controls: false,
            channel_inputs: false,
            channel_shape: false,
            transport: false,
            upload: false,
        }
    }

    /// What `universe build` accepts.
    pub(crate) const fn build() -> Self {
        Self {
            verb: "build",
            platform: true,
            fragment: false,
            change_scope: false,
            build_controls: true,
            channel_inputs: false,
            channel_shape: false,
            transport: false,
            upload: false,
        }
    }

    /// What `universe audit` accepts.
    pub(crate) const fn audit() -> Self {
        Self {
            verb: "audit",
            platform: false,
            fragment: true,
            change_scope: false,
            build_controls: false,
            channel_inputs: false,
            channel_shape: false,
            transport: false,
            upload: false,
        }
    }

    /// What `universe index` accepts.
    pub(crate) const fn index() -> Self {
        Self {
            verb: "index",
            platform: false,
            fragment: false,
            change_scope: false,
            build_controls: false,
            channel_inputs: true,
            channel_shape: true,
            transport: false,
            upload: false,
        }
    }

    /// What `universe publish` accepts.
    pub(crate) const fn publish() -> Self {
        Self {
            verb: "publish",
            platform: false,
            fragment: false,
            change_scope: false,
            build_controls: false,
            channel_inputs: true,
            channel_shape: true,
            transport: true,
            upload: false,
        }
    }

    /// What `universe publish-artifacts` accepts.
    pub(crate) const fn publish_artifacts() -> Self {
        Self {
            verb: "publish-artifacts",
            platform: false,
            fragment: false,
            change_scope: false,
            build_controls: false,
            channel_inputs: true,
            channel_shape: true,
            transport: false,
            upload: true,
        }
    }

    fn usage(&self, argument: &str) -> String {
        let mut usage = format!("usage: universe {} [--root <dir>]", self.verb);
        if self.platform {
            usage.push_str(" [--platform <subdir>]");
        }
        if self.change_scope {
            usage.push_str(" [--changed-only [--base-ref <ref>]]");
        }
        if self.build_controls {
            usage.push_str(" [--timeout <seconds>] [--receipts <dir>]");
        }
        if self.fragment {
            usage.push_str(" [--emit-fragment <dir>]");
        }
        if self.channel_inputs {
            usage.push_str(" [--fragments <dir>] [--output <dir>]");
        }
        if self.channel_shape {
            usage.push_str(" [--base-url <url>] [--package <name>]...");
        }
        if self.transport {
            usage.push_str(
                " [--remote <name>] [--branch <name>] [--store <name>] [--keep <n>] [--check] \
                 [--dry-run]",
            );
        }
        if self.upload {
            usage.push_str(" [--store <name>] [--force] [--dry-run] [--notes-file <path>]");
        }
        usage.push_str(" [--json]");
        format!("unknown argument: {argument}\n{usage}")
    }
}

/// Parsed options for one verb invocation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct Options {
    /// Project root the manifest is read from.
    pub(crate) root: PathBuf,
    /// Whether to emit the machine-readable envelope.
    pub(crate) json: bool,
    /// Platform filter, when the verb can be narrowed to one subdir.
    pub(crate) platform: Option<String>,
    /// Directory the `audit` verb writes catalogue fragments to.
    pub(crate) fragment_directory: Option<PathBuf>,
    /// Whether `plan` should keep only what a change can affect.
    pub(crate) changed_only: bool,
    /// Git ref a change is measured against.
    pub(crate) base_ref: String,
    /// Wall-clock limit for one build command.
    pub(crate) timeout_seconds: u64,
    /// Directory the `build` verb writes receipts to.
    pub(crate) receipts: Option<PathBuf>,
    /// Directory the channel's fragments are read from.
    pub(crate) fragments: Option<PathBuf>,
    /// Directory a rendered channel tree is written to.
    pub(crate) output: Option<PathBuf>,
    /// Public URL the channel will be served from, recorded in its metadata.
    pub(crate) base_url: Option<String>,
    /// Packages a publish is narrowed to; empty means all of them.
    pub(crate) packages: Vec<String>,
    /// Git remote the channel branch lives on.
    pub(crate) remote: Option<String>,
    /// Branch the channel is published to.
    pub(crate) branch: Option<String>,
    /// Store whose budgets and channel the publish is measured against.
    pub(crate) store: Option<String>,
    /// How many builds of one version the snapshot keeps; `None` means the store's own rule,
    /// and no rule there means every build.
    pub(crate) keep: Option<usize>,
    /// Whether to only compare the branch with what the fragments render to.
    pub(crate) check: bool,
    /// Whether to describe the publish without performing it.
    pub(crate) dry_run: bool,
    /// Whether an upload may replace an artifact whose digest differs.
    pub(crate) force: bool,
    /// Prose published as a release body, when the caller names a file.
    pub(crate) notes_file: Option<PathBuf>,
}

impl Options {
    /// Parse the trailing arguments of a verb.
    ///
    /// # Errors
    ///
    /// Returns the usage text when an argument is unknown, or when a flag that needs a value
    /// is the last one.
    pub(crate) fn parse(args: &[String], spec: VerbSpec) -> Result<Self, String> {
        let mut options = Self {
            root: PathBuf::from("."),
            json: false,
            platform: None,
            fragment_directory: None,
            changed_only: false,
            base_ref: "origin/main".to_owned(),
            timeout_seconds: 1800,
            receipts: None,
            fragments: None,
            output: None,
            base_url: None,
            packages: Vec::new(),
            remote: None,
            branch: None,
            store: None,
            keep: None,
            check: false,
            dry_run: false,
            force: false,
            notes_file: None,
        };
        let mut rest = args.iter();
        while let Some(argument) = rest.next() {
            match argument.as_str() {
                "--json" => options.json = true,
                "--root" => options.root = PathBuf::from(value(rest.next(), "--root")?),
                "--platform" if spec.platform => {
                    options.platform = Some(value(rest.next(), "--platform")?.clone());
                }
                "--emit-fragment" if spec.fragment => {
                    options.fragment_directory =
                        Some(PathBuf::from(value(rest.next(), "--emit-fragment")?));
                }
                "--changed-only" if spec.change_scope => options.changed_only = true,
                "--base-ref" if spec.change_scope => {
                    options.base_ref = value(rest.next(), "--base-ref")?.clone();
                }
                "--timeout" if spec.build_controls => {
                    let raw = value(rest.next(), "--timeout")?;
                    options.timeout_seconds = raw
                        .parse()
                        .map_err(|_| format!("--timeout takes whole seconds, not '{raw}'"))?;
                }
                "--fragments" if spec.channel_inputs => {
                    options.fragments = Some(PathBuf::from(value(rest.next(), "--fragments")?));
                }
                "--output" if spec.channel_inputs => {
                    options.output = Some(PathBuf::from(value(rest.next(), "--output")?));
                }
                "--base-url" if spec.channel_shape => {
                    options.base_url = Some(value(rest.next(), "--base-url")?.clone());
                }
                "--package" if spec.channel_shape => {
                    options
                        .packages
                        .push(value(rest.next(), "--package")?.clone());
                }
                "--remote" if spec.transport => {
                    options.remote = Some(value(rest.next(), "--remote")?.clone());
                }
                "--store" if spec.transport || spec.upload => {
                    options.store = Some(value(rest.next(), "--store")?.clone());
                }
                "--force" if spec.upload => options.force = true,
                "--notes-file" if spec.upload => {
                    options.notes_file = Some(PathBuf::from(value(rest.next(), "--notes-file")?));
                }
                "--branch" if spec.transport => {
                    options.branch = Some(value(rest.next(), "--branch")?.clone());
                }
                "--keep" if spec.transport => {
                    let raw = value(rest.next(), "--keep")?;
                    options.keep = Some(raw.parse().map_err(|_| {
                        format!("--keep takes a whole number of builds, not '{raw}'")
                    })?);
                }
                "--check" if spec.transport => options.check = true,
                "--dry-run" if spec.transport || spec.upload => options.dry_run = true,
                "--receipts" if spec.build_controls => {
                    options.receipts = Some(PathBuf::from(value(rest.next(), "--receipts")?));
                }
                other => return Err(spec.usage(other)),
            }
        }
        Ok(options)
    }
}

fn value<'a>(next: Option<&'a String>, flag: &str) -> Result<&'a String, String> {
    next.ok_or_else(|| format!("{flag} needs a value"))
}

#[cfg(test)]
mod tests {
    use super::{Options, VerbSpec};

    fn parse(args: &[&str], spec: VerbSpec) -> Result<Options, String> {
        let owned: Vec<String> = args.iter().map(|argument| (*argument).to_owned()).collect();
        Options::parse(&owned, spec)
    }

    #[test]
    fn publish_accepts_every_flag_it_documents() {
        let options = parse(
            &[
                "--remote",
                "origin",
                "--branch",
                "channel",
                "--store",
                "conda-branch",
                "--fragments",
                "output/fragments",
                "--output",
                "output/publish",
                "--base-url",
                "https://example.invalid/channel",
                "--package",
                "a",
                "--package",
                "b",
                "--keep",
                "2",
                "--check",
                "--dry-run",
                "--json",
            ],
            VerbSpec::publish(),
        )
        .expect("flags are accepted");

        assert_eq!(options.remote.as_deref(), Some("origin"));
        assert_eq!(options.branch.as_deref(), Some("channel"));
        assert_eq!(options.store.as_deref(), Some("conda-branch"));
        assert_eq!(options.packages, ["a", "b"]);
        assert_eq!(options.keep, Some(2));
        assert!(options.check && options.dry_run && options.json);
    }

    #[test]
    fn index_refuses_a_transport_flag_because_it_publishes_nothing() {
        // A flag that silently did nothing would make a script believe it had published.
        let error = parse(&["--remote", "origin"], VerbSpec::index()).expect_err("refused");
        assert!(error.contains("usage: universe index"), "{error}");
    }

    #[test]
    fn a_keep_that_is_not_a_number_is_refused_with_the_flag_named() {
        let error = parse(&["--keep", "some"], VerbSpec::publish()).expect_err("refused");
        assert!(error.contains("--keep takes a whole number"), "{error}");
        // `index` renders a channel but pushes nothing, so retention is not its business.
        assert!(parse(&["--keep", "2"], VerbSpec::index()).is_err());
    }

    #[test]
    fn a_channel_verb_documents_its_own_flags_in_the_usage_line() {
        let error = parse(&["--nonsense"], VerbSpec::publish()).expect_err("refused");
        assert!(error.contains("--remote <name>"), "{error}");
        assert!(error.contains("--check"), "{error}");
        let error = parse(&["--nonsense"], VerbSpec::index()).expect_err("refused");
        assert!(error.contains("--output <dir>"), "{error}");
        assert!(!error.contains("--remote"), "{error}");
    }

    #[test]
    fn publish_artifacts_takes_the_file_its_release_body_is_read_from() {
        // A path, not the prose: the body is Markdown with headings, links and code blocks, and
        // a shell argument holding it is one quoting mistake away from publishing a fragment.
        let options = parse(
            &["--notes-file", "docs/releases/0.1.0.md"],
            VerbSpec::publish_artifacts(),
        )
        .expect("the flag is accepted");
        assert_eq!(
            options.notes_file.as_deref(),
            Some(std::path::Path::new("docs/releases/0.1.0.md"))
        );
        let error = parse(&["--notes-file"], VerbSpec::publish_artifacts()).expect_err("refused");
        assert!(error.contains("--notes-file needs a value"), "{error}");
    }

    #[test]
    fn a_verb_that_publishes_no_release_refuses_the_notes_file() {
        // `index` renders a channel and uploads nothing, so a body it accepted would be a body
        // it dropped.
        assert!(
            parse(&["--notes-file", "notes.md"], VerbSpec::index()).is_err(),
            "index accepted a release body"
        );
    }

    #[test]
    fn a_flag_without_its_value_says_which_flag() {
        let error = parse(&["--remote"], VerbSpec::publish()).expect_err("refused");
        assert!(error.contains("--remote needs a value"), "{error}");
    }
}
