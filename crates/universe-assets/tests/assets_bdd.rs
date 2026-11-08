//! Scenarios for the assets a rendered workflow is built out of.
//!
//! The unit tests in `src/lib.rs` assert the same properties one at a time. These read them
//! as a consumer would: the portal is there without a checkout, and nothing the template can
//! emit is left floating on a tag that a maintainer upstream can move.

use std::path::{Path, PathBuf};

use rstest::fixture;
use rstest_bdd_macros::{given, scenario, then, when};
use universe_assets::{
    CHECKOUT, DOWNLOAD_ARTIFACT, PIXI_VERSION, RUST_CACHE, SETUP_PIXI, UPLOAD_ARTIFACT,
    WORKFLOW_TEMPLATE, portal_assets,
};

/// Whatever a scenario asked about, held as the strings it is about.
#[derive(Default)]
struct Assets {
    actions: Vec<&'static str>,
    template: Option<&'static str>,
}

/// The workspace root, one level above the crate that was compiled.
fn workspace() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("the crate lives two levels below the workspace root")
        .to_path_buf()
}

#[fixture]
fn assets() -> Assets {
    Assets::default()
}

#[given("the embedded portal bundle")]
fn the_portal_bundle(assets: &mut Assets) {
    assets.template = None;
}

#[given("the actions the template can emit")]
fn the_actions(assets: &mut Assets) {
    assets.actions = vec![
        CHECKOUT,
        SETUP_PIXI,
        RUST_CACHE,
        UPLOAD_ARTIFACT,
        DOWNLOAD_ARTIFACT,
    ];
}

#[given("the workflow template")]
fn the_template(assets: &mut Assets) {
    assets.template = Some(WORKFLOW_TEMPLATE);
}

#[given("the pinned Pixi version")]
fn the_pinned_pixi(assets: &mut Assets) {
    assets.template = None;
}

#[when("it is inspected")]
fn inspect_the_portal(_assets: &Assets) {
    // Nothing to do: the assertion below reads the same process-wide bundle a renderer does.
}

#[when("they are inspected")]
fn inspect_the_actions(_assets: &Assets) {
    // The assertion names what it checks; the list is already in scope.
}

#[when("its double braces are inspected")]
fn inspect_the_braces(_assets: &Assets) {
    // Asserted below, over the whole template.
}

#[when("the workspace floor is read")]
fn read_the_floor(_assets: &Assets) {
    // Asserted below, from the workspace's own manifest.
}

#[then("it holds an index page")]
fn holds_an_index(_assets: &Assets) {
    let index = portal_assets().get_file("index.html");
    assert!(index.is_some(), "the embedded portal has no index page");
    assert!(
        !index.expect("checked above").contents().is_empty(),
        "the embedded index page is empty"
    );
}

#[then("each one names a forty-character commit")]
fn forty_character_commit(assets: &Assets) {
    assert_eq!(
        assets.actions.len(),
        5,
        "an action joined the template unpinned"
    );
    for action in &assets.actions {
        let reference = action.split('@').nth(1).expect("an action reference");
        let commit = reference.split_whitespace().next().expect("a commit");
        assert_eq!(commit.len(), 40, "{action} is not pinned to a commit");
        assert!(
            commit.bytes().all(|byte| byte.is_ascii_hexdigit()),
            "{action}"
        );
    }
}

#[then("each one records the release it was pinned from")]
fn records_the_release(assets: &Assets) {
    for action in &assets.actions {
        // `uses: owner/repo@<sha> # vX.Y.Z` — the comment is the only place a reader learns
        // which release the sha was, and without it the next maintainer re-derives it by hand.
        let comment = action.split('#').nth(1).map(str::trim);
        assert!(
            matches!(comment, Some(tag) if tag.starts_with('v')),
            "{action} does not record the release it was pinned from"
        );
    }
}

#[then("every one of them belongs to a GitHub expression")]
fn only_github_expressions(assets: &Assets) {
    let template = assets
        .template
        .expect("a scenario asked about the template");
    for (index, _) in template.match_indices("{{") {
        assert!(
            template[..index].ends_with('$'),
            "a bare {{{{ at byte {index} would be read as a template variable"
        );
    }
    assert!(template.contains("${{ github.ref }}"), "{template}");
}

#[then("the template writes its own variables in angle brackets")]
fn angle_brackets(assets: &Assets) {
    let template = assets
        .template
        .expect("a scenario asked about the template");
    assert!(
        template.contains("<< name >>"),
        "the template names nothing"
    );
}

#[then("the pinned version is the floor without its leading v")]
fn matches_the_floor(_assets: &Assets) {
    let manifest = std::fs::read_to_string(workspace().join("pixi.toml"))
        .expect("the workspace manifest is readable");
    let floor = manifest
        .lines()
        .find_map(|line| line.trim().strip_prefix("requires-pixi = "))
        .expect("pixi.toml declares requires-pixi");
    let floor = floor.trim().trim_matches('"').trim_start_matches(">=");
    assert_eq!(
        PIXI_VERSION.trim_start_matches('v'),
        floor,
        "the generated workflow would install a different Pixi than this workspace builds with"
    );
}

#[scenario(
    path = "features/assets.feature",
    name = "The portal bundle is embedded, not read from the consumer's disk"
)]
fn the_portal_is_embedded(assets: Assets) {
    let _ = assets;
}

#[scenario(
    path = "features/assets.feature",
    name = "Every action the template can emit is pinned to a commit"
)]
fn every_action_is_pinned(assets: Assets) {
    let _ = assets;
}

#[scenario(
    path = "features/assets.feature",
    name = "The template can hold GitHub expressions without being read as a variable"
)]
fn the_template_survives_expressions(assets: Assets) {
    let _ = assets;
}

#[scenario(
    path = "features/assets.feature",
    name = "The generated workflow installs the Pixi this workspace builds with"
)]
fn the_pixi_version_matches(assets: Assets) {
    let _ = assets;
}
