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
