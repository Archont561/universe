//! The options every producer verb shares.
//!
//! One parser rather than one per verb: `--root` meaning the project root, `--json` meaning
//! the envelope, and an unknown flag being a usage error are the same promises everywhere,
//! and three parsers would let them drift.

use std::path::PathBuf;

/// Parsed options for one verb invocation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct Options {
    /// Project root the manifest is read from.
    pub(crate) root: PathBuf,
    /// Whether to emit the machine-readable envelope.
    pub(crate) json: bool,
    /// Platform filter, when the verb can be narrowed to one subdir.
    pub(crate) platform: Option<String>,
}

impl Options {
    /// Parse the trailing arguments of a verb.
    ///
    /// # Errors
    ///
    /// Returns the usage text when an argument is unknown, when a flag that needs a value is
    /// the last one, or when `allow_platform` is false and `--platform` is given.
    pub(crate) fn parse(args: &[String], verb: &str, allow_platform: bool) -> Result<Self, String> {
        let mut options = Self {
            root: PathBuf::from("."),
            json: false,
            platform: None,
        };
        let mut rest = args.iter();
        while let Some(argument) = rest.next() {
            match argument.as_str() {
                "--json" => options.json = true,
                "--root" => {
                    options.root = PathBuf::from(
                        rest.next()
                            .ok_or_else(|| "--root needs a directory".to_owned())?,
                    );
                }
                "--platform" if allow_platform => {
                    options.platform = Some(
                        rest.next()
                            .ok_or_else(|| "--platform needs a subdir".to_owned())?
                            .clone(),
                    );
                }
                other => return Err(usage(verb, allow_platform, other)),
            }
        }
        Ok(options)
    }
}

fn usage(verb: &str, allow_platform: bool, argument: &str) -> String {
    let platform = if allow_platform {
        " [--platform <subdir>]"
    } else {
        ""
    };
    format!(
        "unknown argument: {argument}\nusage: universe {verb} [--root <dir>]{platform} [--json]"
    )
}
