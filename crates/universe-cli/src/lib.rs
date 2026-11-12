//! Library surface for the `universe` command facade.
//!
//! The binary in `src/main.rs` does nothing but hand its arguments to [`route`] and exec the
//! component it names. Routing lives here so the rule "which component owns this verb" is a
//! behaviour with a name, rather than a `match` a test can only reach through `main`.

use std::ffi::OsString;

/// Facade version, read by the compiler from this crate's manifest.
///
/// The manifest inherits `[workspace.package] version`, so a release promotes one literal in
/// the root `Cargo.toml`. The `VERSIONS` file beside this crate is a packaging mirror, and a
/// test holds it equal to this constant.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Select the component executable and arguments for the public `universe` facade.
///
/// The facade owns no product behavior: producer commands go to `universe-kit`,
/// and the explicit `airlock` namespace goes to `universe-airlock`.
///
/// Four verbs in that namespace are the exceptions, and they are exceptions by direction rather
/// than by taste: rendering reads `universe.toml` and writes committed files, publishing pushes
/// the ref the entrypoint clones to a remote, packing writes the payload a restore consumes, and
/// mirroring fetches a lock's artifacts and publishes them to a branch. All four are production,
/// so all four belong to the producer, whose rendered workflow calls them. `airlock exec`,
/// `verify` and `restore` run on a target whose only installation is the consumer, and they stay
/// where they are.
#[must_use]
pub fn route(args: &[OsString]) -> (&'static str, &[OsString]) {
    match args.first().and_then(|arg| arg.to_str()) {
        Some("airlock") => {
            let verb = args.get(1).and_then(|arg| arg.to_str());
            if matches!(
                verb,
                Some("render" | "publish-bootstrap" | "pack" | "mirror")
            ) {
                ("universe-kit", args)
            } else {
                ("universe-airlock", &args[1..])
            }
        }
        Some("kit") => ("universe-kit", &args[1..]),
        _ => ("universe-kit", args),
    }
}

#[cfg(test)]
mod tests {
    use super::{VERSION, route};
    use std::ffi::OsString;

    fn args(values: &[&str]) -> Vec<OsString> {
        values.iter().map(OsString::from).collect()
    }

    #[test]
    fn version_matches_the_package_manifest() {
        assert_eq!(VERSION, env!("CARGO_PKG_VERSION"));
    }

    #[test]
    fn version_matches_the_versions_file() {
        assert_eq!(VERSION, include_str!("../VERSIONS").trim());
    }

    #[test]
    fn defaults_to_kit_without_rewriting_arguments() {
        let arguments = args(&["pages", "build", "--offline"]);
        let (component, forwarded) = route(&arguments);
        assert_eq!(component, "universe-kit");
        assert_eq!(forwarded, arguments);
    }

    #[test]
    fn explicit_kit_namespace_is_removed() {
        let arguments = args(&["kit", "pages", "build"]);
        let (component, forwarded) = route(&arguments);
        assert_eq!(component, "universe-kit");
        assert_eq!(forwarded, args(&["pages", "build"]));
    }

    #[test]
    fn airlock_namespace_is_routed_without_other_changes() {
        let arguments = args(&["airlock", "exec", "--", "cargo", "test"]);
        let (component, forwarded) = route(&arguments);
        assert_eq!(component, "universe-airlock");
        assert_eq!(forwarded, args(&["exec", "--", "cargo", "test"]));
    }

    #[test]
    fn rendering_an_entrypoint_is_producer_work_and_reaches_kit() {
        let arguments = args(&["airlock", "render", "--check"]);
        let (component, forwarded) = route(&arguments);
        assert_eq!(component, "universe-kit");
        // The namespace is not rewritten: `kit` reads the same two tokens it is handed.
        assert_eq!(forwarded, arguments);

        // Publishing the ref the entrypoint clones is production too: it writes to a remote, and
        // a consumer install has no business pushing anywhere.
        let publishing = args(&["airlock", "publish-bootstrap", "--binary", "boot"]);
        let (component, forwarded) = route(&publishing);
        assert_eq!(component, "universe-kit");
        assert_eq!(forwarded, publishing);

        // Packing writes the payload a restore consumes, from artifacts this side built.
        let packing = args(&["airlock", "pack"]);
        let (component, forwarded) = route(&packing);
        assert_eq!(component, "universe-kit");
        assert_eq!(forwarded, packing);

        // Mirroring fetches a lock's artifacts and publishes them: the rendered workflow calls
        // `universe airlock mirror`, and a consumer install has no network to mirror over.
        let mirroring = args(&["airlock", "mirror", "--adapter", "pixi", "--json"]);
        let (component, forwarded) = route(&mirroring);
        assert_eq!(component, "universe-kit");
        assert_eq!(forwarded, mirroring);

        // Everything else in the namespace is the consumer's.
        // A sandbox runs a command the user built, on their own machine: the consumer's.
        for verb in ["exec", "verify", "restore", "sandbox"] {
            let arguments = args(&["airlock", verb]);
            let (component, forwarded) = route(&arguments);
            assert_eq!(component, "universe-airlock", "{verb}");
            assert_eq!(forwarded, args(&[verb]));
        }
    }

    #[cfg(unix)]
    #[test]
    fn non_utf8_argument_is_preserved() {
        use std::os::unix::ffi::OsStringExt;

        let arguments = vec![OsString::from("airlock"), OsString::from_vec(vec![0xff])];
        let (component, forwarded) = route(&arguments);
        assert_eq!(component, "universe-airlock");
        assert_eq!(forwarded[0].as_encoded_bytes(), &[0xff]);
    }
}
