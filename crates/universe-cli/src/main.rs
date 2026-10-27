use std::ffi::OsString;
use std::process::{self, Command};

/// Select the component executable and arguments for the public `universe` facade.
///
/// The facade owns no product behavior: producer commands go to `universe-kit`,
/// and the explicit `airlock` namespace goes to `universe-airlock`.
fn route(args: &[OsString]) -> (&'static str, &[OsString]) {
    match args.first().and_then(|arg| arg.to_str()) {
        Some("airlock") => ("universe-airlock", &args[1..]),
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
