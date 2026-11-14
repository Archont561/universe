//! Library surface for the `universe` command facade.
//!
//! The binary in `src/main.rs` does nothing but hand its arguments to [`route`] and exec the
//! component it names. Routing lives here so the rule "which component owns this verb" is a
//! behaviour with a name, rather than a `match` a test can only reach through `main`.

use std::ffi::OsString;
use std::path::{Path, PathBuf};

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

/// Where the facade looks for a component, in order.
///
/// The three fields are the three places, and keeping them as data rather than reading the
/// environment inside the search is what makes the order testable without a workspace build.
/// The order itself is the point: a component **beside the running executable** comes before
/// `PATH`, because a prefix install puts the whole set in one directory and a stray binary of
/// the same name earlier on `PATH` must not be what the user gets instead (GitHub issue 125).
#[derive(Clone, Debug, Default)]
pub struct Search {
    /// An explicit directory, from [`COMPONENT_DIR`]. First because it is a deliberate
    /// statement about this invocation, and a testing or packaging harness needs to beat both
    /// of the others.
    pub override_dir: Option<PathBuf>,
    /// The directory the running facade is in.
    pub exe_dir: Option<PathBuf>,
    /// The `PATH` value, searched last.
    pub path: Option<OsString>,
}

impl Search {
    /// The places this process can actually look.
    #[must_use]
    pub fn ambient() -> Self {
        Self {
            override_dir: std::env::var_os(COMPONENT_DIR).map(PathBuf::from),
            exe_dir: std::env::current_exe()
                .ok()
                .and_then(|exe| exe.parent().map(Path::to_path_buf)),
            path: std::env::var_os("PATH"),
        }
    }
}

/// Environment variable naming a directory to search before any other.
pub const COMPONENT_DIR: &str = "UNIVERSE_COMPONENT_DIR";

/// The file name a component has on this platform.
fn file_name(component: &str) -> String {
    format!("{component}{}", std::env::consts::EXE_SUFFIX)
}

/// Whether `path` names something this process could run.
///
/// Existence is not enough: a directory named `universe-kit`, or a file without the executable
/// bit, would be found and then fail at `exec` with a message that says nothing about the
/// search. On platforms without a mode, existing is the whole test.
fn is_runnable(path: &Path) -> bool {
    let Ok(metadata) = std::fs::metadata(path) else {
        return false;
    };
    if !metadata.is_file() {
        return false;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        metadata.permissions().mode() & 0o111 != 0
    }
    #[cfg(not(unix))]
    {
        true
    }
}

/// Find a component executable.
///
/// # Errors
///
/// Returns [`MissingComponent`] when no directory holds a runnable file of that name; the
/// error carries every place that was looked, so the message can name them.
pub fn resolve(component: &str, search: &Search) -> Result<PathBuf, MissingComponent> {
    let name = file_name(component);
    let mut directories: Vec<PathBuf> = Vec::new();
    if let Some(directory) = &search.override_dir {
        directories.push(directory.clone());
    }
    if let Some(directory) = &search.exe_dir {
        directories.push(directory.clone());
    }
    if let Some(path) = &search.path {
        directories.extend(std::env::split_paths(path));
    }
    let mut looked: Vec<PathBuf> = Vec::new();
    for directory in directories {
        if is_runnable(&directory.join(&name)) {
            return Ok(directory.join(&name));
        }
        if !looked.contains(&directory) {
            looked.push(directory);
        }
    }
    Err(MissingComponent {
        component: component.to_owned(),
        looked,
    })
}

/// A component that could not be found anywhere the facade looks.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MissingComponent {
    /// The component's name, as the facade refers to it.
    pub component: String,
    /// Every directory that was searched, in order and without repeats.
    pub looked: Vec<PathBuf>,
}

impl std::fmt::Display for MissingComponent {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // A remediation, not just a complaint: the three ways to fix this are the three places
        // the search just visited, so they are named in the order a person would try them.
        let looked = if self.looked.is_empty() {
            "nothing was searched".to_owned()
        } else {
            self.looked
                .iter()
                .map(|directory| directory.display().to_string())
                .collect::<Vec<_>>()
                .join(", ")
        };
        write!(
            formatter,
            "cannot find {component}; looked in {looked}. Install {component} in the same \
             directory as this binary, add that directory to PATH, or set {COMPONENT_DIR} to \
             the directory that holds it",
            component = self.component,
        )
    }
}

impl std::error::Error for MissingComponent {}

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

    /// A directory holding a fake component.
    ///
    /// The facade resolves a path; it does not check what the path runs. A shell script with
    /// the executable bit set is the cheapest honest stand-in, and it keeps these tests free of
    /// a workspace build — and of a dev-dependency, since the lockfile is workflow-owned.
    fn component_in(directory: &std::path::Path, name: &str) -> std::path::PathBuf {
        std::fs::create_dir_all(directory).expect("component directory");
        let path = directory.join(name);
        std::fs::write(&path, "#!/bin/sh\nexit 0\n").expect("component");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755))
                .expect("executable");
        }
        path
    }

    /// A directory of this test's own, removed when it goes out of scope.
    ///
    /// `tempfile` is not a dependency of this crate and adding one would change `Cargo.lock`,
    /// which the relock workflow owns. This is the whole of what the tests use.
    struct Scratch(std::path::PathBuf);

    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    impl std::ops::Deref for Scratch {
        type Target = std::path::Path;

        fn deref(&self) -> &Self::Target {
            &self.0
        }
    }

    impl Scratch {
        fn path(&self) -> &std::path::Path {
            &self.0
        }
    }

    fn scratch(label: &str) -> Scratch {
        use std::sync::atomic::{AtomicUsize, Ordering};
        static COUNTER: AtomicUsize = AtomicUsize::new(0);
        let unique = format!(
            "universe-cli-{label}-{}-{}",
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::Relaxed)
        );
        let path = std::env::temp_dir().join(unique);
        std::fs::create_dir_all(&path).expect("scratch directory");
        Scratch(path)
    }

    #[test]
    fn a_component_beside_the_facade_is_found_without_any_path() {
        // GitHub issue 125: the facade was installed by absolute path into a prefix whose bin
        // directory was not on `PATH`, and it reported the component missing although the
        // component was in the very same directory.
        let facade = crate::tests::scratch("beside");
        let beside = component_in(facade.path(), "universe-kit");
        let search = super::Search {
            override_dir: None,
            exe_dir: Some(facade.path().to_path_buf()),
            path: None,
        };

        let resolved = super::resolve("universe-kit", &search).expect("found beside the facade");

        assert_eq!(resolved, beside);
    }

    #[test]
    fn the_override_directory_beats_both_of_the_others() {
        // A harness that stages a component somewhere has to be able to say so and be obeyed,
        // even when an older copy sits beside the facade and another sits on PATH.
        let elsewhere = scratch("override");
        let staged = component_in(elsewhere.path(), "universe-kit");
        let facade = scratch("override-facade");
        component_in(facade.path(), "universe-kit");
        let path = scratch("override-path");
        component_in(path.path(), "universe-kit");

        let search = super::Search {
            override_dir: Some(elsewhere.path().to_path_buf()),
            exe_dir: Some(facade.path().to_path_buf()),
            path: Some(path.path().as_os_str().to_owned()),
        };

        assert_eq!(
            super::resolve("universe-kit", &search).expect("resolved"),
            staged
        );
    }

    #[test]
    fn a_condition_on_path_cannot_shadow_the_component_beside_the_facade() {
        // The defect the order exists to prevent: an unrelated `universe-kit` earlier on PATH
        // being run instead of the one installed with this facade.
        let facade = scratch("shadow-facade");
        let installed = component_in(facade.path(), "universe-kit");
        let stray = scratch("shadow-stray");
        component_in(stray.path(), "universe-kit");

        let search = super::Search {
            override_dir: None,
            exe_dir: Some(facade.path().to_path_buf()),
            path: Some(stray.path().as_os_str().to_owned()),
        };

        assert_eq!(
            super::resolve("universe-kit", &search).expect("resolved"),
            installed,
            "a stray binary on PATH won over the one installed beside the facade"
        );
    }

    #[test]
    fn path_is_the_last_place_looked() {
        // A developer build has no component beside the facade; PATH is what makes the
        // workspace usable, so it has to work when nothing else does.
        let facade = scratch("path-facade");
        let on_path = scratch("path-dir");
        let found = component_in(on_path.path(), "universe-airlock");

        let search = super::Search {
            override_dir: None,
            exe_dir: Some(facade.path().to_path_buf()),
            path: Some(on_path.path().as_os_str().to_owned()),
        };

        assert_eq!(
            super::resolve("universe-airlock", &search).expect("resolved"),
            found
        );
    }

    #[test]
    fn a_missing_component_names_itself_where_it_looked_and_what_to_do() {
        // Exit 127 with "No such file or directory" is what the issue reported, and it names
        // neither the places searched nor a remedy.
        let facade = scratch("missing-facade");
        let search = super::Search {
            override_dir: None,
            exe_dir: Some(facade.path().to_path_buf()),
            path: Some(std::ffi::OsString::from("/nonexistent")),
        };

        let error = super::resolve("universe-kit", &search).expect_err("not found");
        let message = error.to_string();

        assert!(message.contains("universe-kit"), "{message}");
        assert!(
            message.contains(&facade.path().display().to_string()),
            "{message}"
        );
        assert!(message.contains("/nonexistent"), "{message}");
        assert!(message.contains(super::COMPONENT_DIR), "{message}");
        assert_eq!(error.component, "universe-kit");
    }

    #[test]
    fn something_that_cannot_be_run_is_not_taken_for_the_component() {
        // A directory with the component's name, or a file without the executable bit, would
        // be found here and fail at `exec` with a message about the file rather than the
        // search. Neither counts as found.
        let facade = scratch("unrunnable");
        std::fs::create_dir_all(facade.path().join("universe-kit")).expect("a directory");
        let search = super::Search {
            override_dir: None,
            exe_dir: Some(facade.path().to_path_buf()),
            path: None,
        };

        assert!(super::resolve("universe-kit", &search).is_err());

        #[cfg(unix)]
        {
            // The other shape: a real file that the filesystem will not run.
            let without_mode = scratch("unrunnable-file");
            let path = without_mode.path().join("universe-kit");
            std::fs::write(&path, "#!/bin/sh\n").expect("file");
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644)).expect("mode");
            let search = super::Search {
                override_dir: None,
                exe_dir: Some(without_mode.path().to_path_buf()),
                path: None,
            };
            assert!(
                super::resolve("universe-kit", &search).is_err(),
                "a file with no executable bit was taken for the component"
            );
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
