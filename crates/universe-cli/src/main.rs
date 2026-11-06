use std::ffi::OsString;
use std::process::{self, Command};

/// Select the component executable and arguments for the public `universe` facade.
///
/// The facade owns no product behavior: producer commands go to `universe-kit`,
/// and the explicit `airlock` namespace goes to `universe-airlock`.
///
/// `airlock render` is the one exception, and it is an exception by direction rather than by
/// taste: it reads `universe.toml` and writes committed files, which is production, so it
/// belongs to the producer. `airlock exec`, `verify` and `restore` run on a target whose only
/// installation is the consumer, and they stay where they are.
fn route(args: &[OsString]) -> (&'static str, &[OsString]) {
    match args.first().and_then(|arg| arg.to_str()) {
        Some("airlock") => {
            let verb = args.get(1).and_then(|arg| arg.to_str());
            if verb == Some("render") {
                ("universe-kit", args)
            } else {
                ("universe-airlock", &args[1..])
            }
        }
        Some("kit") => ("universe-kit", &args[1..]),
        _ => ("universe-kit", args),
    }
}

fn main() {
    let args: Vec<OsString> = std::env::args_os().skip(1).collect();
    let (component, forwarded_args) = route(&args);
    let status = match Command::new(component).args(forwarded_args).status() {
        Ok(status) => status,
        Err(error) => {
            eprintln!("universe: cannot run {component}: {error}");
            process::exit(127);
        }
    };

    process::exit(status.code().unwrap_or(1));
}

#[cfg(test)]
mod tests {
    use super::route;
    use std::ffi::OsString;

    fn args(values: &[&str]) -> Vec<OsString> {
        values.iter().map(OsString::from).collect()
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

        // Everything else in the namespace is the consumer's.
        for verb in ["exec", "verify", "restore"] {
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
