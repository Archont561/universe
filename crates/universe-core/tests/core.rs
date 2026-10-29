use universe_core::{
    CliError, EntryKind, FIRST_CLASS_ADAPTER_BUDGET, Product, STDOUT_SCHEMA_VERSION, VERSION,
    execute_version, parse_version_args, render_version, run_main,
};

#[test]
fn product_names_cover_all_artifacts() {
    let actual: Vec<(&str, &str, String)> = Product::ALL
        .into_iter()
        .map(|product| {
            (
                product.display_name(),
                product.binary_name(),
                product.to_string(),
            )
        })
        .collect();

    assert_eq!(
        actual,
        vec![
            ("Universe Kit", "universe-kit", "Universe Kit".to_owned()),
            (
                "Universe Airlock",
                "universe-airlock",
                "Universe Airlock".to_owned()
            ),
            (
                "Universe Airlock Bootstrap",
                "universe-airlock-bootstrap",
                "Universe Airlock Bootstrap".to_owned()
            ),
        ]
    );
}

#[test]
fn entry_kind_slugs_are_stable() {
    let actual: Vec<&str> = EntryKind::ALL.into_iter().map(EntryKind::slug).collect();

    assert_eq!(actual, ["software", "data", "model", "skill"]);
    assert_eq!(FIRST_CLASS_ADAPTER_BUDGET, 5);
    assert_eq!(STDOUT_SCHEMA_VERSION.get(), 2);
}

#[test]
fn version_args_accept_default_json_and_all_aliases() {
    for args in [
        Vec::<&str>::new(),
        vec!["version"],
        vec!["--version"],
        vec!["-V"],
    ] {
        let options = parse_version_args(args).expect("valid version invocation");
        assert!(!options.json());
    }

    for args in [
        vec!["--json"],
        vec!["--json", "version"],
        vec!["version", "--json"],
    ] {
        let options = parse_version_args(args).expect("valid JSON version invocation");
        assert!(options.json());
    }
}

#[test]
fn version_args_reject_duplicate_and_unknown_tokens() {
    assert_eq!(
        parse_version_args(["version", "--version"]),
        Err(CliError::DuplicateVersion)
    );
    assert_eq!(
        parse_version_args(["restore"]),
        Err(CliError::UnknownArgument("restore".to_owned()))
    );
    assert_eq!(
        CliError::DuplicateVersion.to_string(),
        "version may be requested only once"
    );
    assert_eq!(
        CliError::UnknownArgument("restore".to_owned()).to_string(),
        "unknown argument: restore"
    );
}

#[test]
fn renders_plain_and_json_versions() {
    let plain = render_version(
        Product::Airlock,
        parse_version_args(["version"]).expect("plain options"),
    );
    assert_eq!(plain, format!("universe-airlock {VERSION}"));

    let json = render_version(
        Product::Bootstrap,
        parse_version_args(["--json"]).expect("JSON options"),
    );
    assert_eq!(
        json,
        format!(
            "{{\"schema_version\":2,\"command\":\"version\",\"ok\":true,\"result\":{{\"product\":\"Universe Airlock Bootstrap\",\"binary\":\"universe-airlock-bootstrap\",\"version\":\"{VERSION}\"}},\"diagnostics\":[],\"artifacts\":[]}}"
        )
    );
}

#[test]
fn execute_version_splits_streams_and_exit_codes() {
    let ok = execute_version(Product::Kit, ["--json", "version"]);
    assert_eq!(ok.exit_code(), 0);
    assert!(ok.stdout().contains("\"ok\":true"));
    assert!(ok.stderr().is_empty());

    let error = execute_version(Product::Kit, ["publish"]);
    assert_eq!(error.exit_code(), 2);
    assert!(error.stdout().is_empty());
    assert!(error.stderr().contains("unknown argument: publish"));
    assert!(error.stderr().contains("usage: universe-kit"));
}

#[test]
fn run_main_returns_success_and_usage_codes() {
    assert_eq!(
        format!("{:?}", run_main(Product::Kit, ["version"])),
        format!("{:?}", std::process::ExitCode::SUCCESS)
    );
    assert_eq!(
        format!("{:?}", run_main(Product::Kit, ["bogus"])),
        format!("{:?}", std::process::ExitCode::from(2))
    );
}
