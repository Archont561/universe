//! Scenarios for rendering a channel and for the three rules a republish obeys.
//!
//! Rendering is a pure function of the fragments, so the scenarios compare real bytes rather
//! than inspecting a structure twice. Publication goes to a real directory, because the rule
//! that matters — a changed digest under an occupied name is refused — is about what the
//! destination ends up holding.

use std::fs;

use rstest::fixture;
use rstest_bdd_macros::{given, scenario, then, when};
use tempfile::TempDir;
use universe_channel::channel::{ChannelError, ChannelFile, ChannelSpec, render};
use universe_channel::fragment::Fragment;
use universe_channel::retention::{Retained, RetentionError, prune};
use universe_channel::store::{FilesystemRemote, Policy, StoreError, Upload, apply};

/// The digest of the fixture's artifact bytes, and of nothing else.
const DIGEST: &str = "9f2c1b0a3d4e5f60718293a4b5c6d7e8f90a1b2c3d4e5f60718293a4b5c6d7e8";

/// A channel, its fragments, the render it produced, and any refusal collected on the way.
#[derive(Default)]
struct Channel {
    spec: Option<ChannelSpec>,
    fragments: Vec<Fragment>,
    renders: Vec<Vec<ChannelFile>>,
    render_error: Option<ChannelError>,
    directory: Option<TempDir>,
    uploads: Vec<Upload>,
    report: Option<universe_channel::store::Report>,
    store_error: Option<StoreError>,
    retained: Option<Retained>,
    retention_error: Option<RetentionError>,
}

impl Channel {
    fn spec(&self) -> &ChannelSpec {
        self.spec.as_ref().expect("a scenario declared a channel")
    }

    fn files(&self) -> &[ChannelFile] {
        self.renders.last().expect("the channel was rendered")
    }

    fn file(&self, path: &str) -> &[u8] {
        self.files()
            .iter()
            .find(|file| file.path == path)
            .unwrap_or_else(|| {
                let paths: Vec<&str> = self.files().iter().map(|f| f.path.as_str()).collect();
                panic!("the channel has no {path}: {paths:?}")
            })
            .bytes
            .as_slice()
    }

    fn render(&mut self) {
        match render(&self.fragments, self.spec()) {
            Ok(files) => self.renders.push(files),
            Err(error) => self.render_error = Some(error),
        }
    }

    fn store(&mut self) -> FilesystemRemote {
        let directory = self
            .directory
            .get_or_insert_with(|| TempDir::new().expect("a scratch channel directory"));
        FilesystemRemote::new(directory.path())
    }

    /// Publish the queued uploads, keeping either the report or the refusal.
    fn publish(&mut self) {
        let remote = self.store();
        match apply(&remote, &self.uploads, Policy::default()) {
            Ok(report) => self.report = Some(report),
            Err(error) => self.store_error = Some(error),
        }
    }
}

/// One audited artifact, as `universe audit --emit-fragment` would have written it.
fn fragment(package: &str, subdir: &str, build: &str, build_number: u64) -> Fragment {
    Fragment {
        package: package.to_owned(),
        version: "0.1.0".to_owned(),
        build: build.to_owned(),
        build_number,
        subdir: subdir.to_owned(),
        digest: DIGEST.to_owned(),
        size_bytes: 7,
        depends: Vec::new(),
        license: Some("MIT".to_owned()),
        license_files: Vec::new(),
        path: String::new(),
    }
}

#[fixture]
fn channel() -> Channel {
    Channel::default()
}

#[given("a channel serving linux-64 and noarch")]
fn a_two_subdir_channel(channel: &mut Channel) {
    channel.spec = Some(ChannelSpec::new(
        "archont561",
        vec!["linux-64".to_owned(), "noarch".to_owned()],
    ));
}

#[given("a channel serving linux-64 only")]
fn a_one_subdir_channel(channel: &mut Channel) {
    channel.spec = Some(ChannelSpec::new("archont561", vec!["linux-64".to_owned()]));
}

#[given("one audited artifact for linux-64")]
fn an_artifact_for_linux(channel: &mut Channel) {
    channel
        .fragments
        .push(fragment("universe-airlock", "linux-64", "h1234567_0", 0));
}

#[given("one audited artifact for noarch")]
fn an_artifact_for_noarch(channel: &mut Channel) {
    channel
        .fragments
        .push(fragment("universe-kit", "noarch", "py_0", 0));
}

#[given("a store that already holds one artifact")]
fn a_store_with_one_artifact(channel: &mut Channel) {
    channel.uploads.push(Upload::new(
        "linux-64/airlock-0.1.0-0.conda",
        b"artifact".to_vec(),
    ));
    channel.publish();
    // The first publish emptied the queue: what the scenario publishes next is what is new.
    channel.uploads.clear();
    channel.report = None;
}

#[given("a channel holding three builds of one version")]
fn three_builds(channel: &mut Channel) {
    channel.spec = Some(ChannelSpec::new("archont561", vec!["linux-64".to_owned()]));
    for (number, build) in [(0, "h1111111_0"), (2, "h3333333_2"), (1, "h2222222_1")] {
        channel
            .fragments
            .push(fragment("universe-airlock", "linux-64", build, number));
    }
}

#[when("the channel is rendered")]
fn rendered(channel: &mut Channel) {
    channel.render();
}

#[when("the channel is rendered twice")]
fn rendered_twice(channel: &mut Channel) {
    channel.render();
    channel.render();
}

#[when("the same artifact is published again")]
fn published_again(channel: &mut Channel) {
    channel.uploads.push(Upload::new(
        "linux-64/airlock-0.1.0-0.conda",
        b"artifact".to_vec(),
    ));
    channel.publish();
}

#[when("different bytes are published under the same name")]
fn published_with_different_bytes(channel: &mut Channel) {
    channel.uploads.push(Upload::new(
        "linux-64/airlock-0.1.0-0.conda",
        b"rewritten".to_vec(),
    ));
    channel.publish();
}

#[when("retention keeps one build of each version")]
fn keep_one(channel: &mut Channel) {
    match prune(&channel.fragments, 1) {
        Ok(retained) => channel.retained = Some(retained),
        Err(error) => channel.retention_error = Some(error),
    }
}

#[when("retention keeps nothing")]
fn keep_nothing(channel: &mut Channel) {
    match prune(&channel.fragments, 0) {
        Ok(retained) => channel.retained = Some(retained),
        Err(error) => channel.retention_error = Some(error),
    }
}

#[then("it answers for noarch as well")]
fn answers_for_noarch(channel: &Channel) {
    let repodata = channel.file("noarch/repodata.json");
    assert!(repodata.starts_with(b"{"));

    let paths: Vec<&str> = channel
        .files()
        .iter()
        .map(|file| file.path.as_str())
        .collect();
    for expected in [
        "linux-64/repodata.json",
        "noarch/repodata.json",
        ".nojekyll",
    ] {
        assert!(
            paths.contains(&expected),
            "the channel has no {expected}: {paths:?}"
        );
    }
}

#[then("it names the channel in its channel data")]
fn names_the_channel(channel: &Channel) {
    let data = String::from_utf8(channel.file("channeldata.json").to_vec())
        .expect("channel data is UTF-8");
    assert!(data.contains("archont561"), "{data}");
    assert!(data.contains("universe-airlock"), "{data}");
}

#[then("the linux-64 repodata names the artifact and its digest")]
fn repodata_repeats_the_audit(channel: &Channel) {
    let repodata = String::from_utf8(channel.file("linux-64/repodata.json").to_vec())
        .expect("repodata is UTF-8");
    assert!(
        repodata.contains("universe-airlock-0.1.0-h1234567_0.conda"),
        "{repodata}"
    );
    assert!(repodata.contains(DIGEST), "{repodata}");
}

#[then("both renders have the same files and the same bytes")]
fn identical_renders(channel: &Channel) {
    assert_eq!(
        channel.renders.len(),
        2,
        "the channel was not rendered twice"
    );
    assert_eq!(channel.renders[0], channel.renders[1]);
}

#[then("rendering is refused because the subdir is not served")]
fn undeclared_subdir(channel: &Channel) {
    assert!(
        matches!(
            channel.render_error,
            Some(ChannelError::UndeclaredSubdir { .. })
        ),
        "expected an undeclared subdir, got {:?}",
        channel.render_error
    );
}

#[then("the publish is a no-op")]
fn no_op(channel: &Channel) {
    let report = channel.report.as_ref().expect("a publish ran");
    assert!(
        report.is_noop(),
        "the publish sent {} uploads",
        report.uploaded.len()
    );
    assert_eq!(report.bytes, 0, "an unchanged artifact was sent again");
}

#[then("the artifact is reported as unchanged")]
fn reported_unchanged(channel: &Channel) {
    let report = channel.report.as_ref().expect("a publish ran");
    assert_eq!(
        report.unchanged,
        ["linux-64/airlock-0.1.0-0.conda".to_owned()]
    );
}

#[then("the publish is refused and the stored bytes are kept")]
fn refused_and_kept(channel: &Channel) {
    assert!(
        matches!(channel.store_error, Some(StoreError::Refused { .. })),
        "expected a refusal, got {:?}",
        channel.store_error
    );
    let directory = channel.directory.as_ref().expect("a scratch directory");
    let stored = fs::read(directory.path().join("linux-64/airlock-0.1.0-0.conda"))
        .expect("the stored artifact");
    assert_eq!(stored, b"artifact", "the destination was rewritten");
}

#[then("two builds are pruned and the newest is kept")]
fn pruned_to_the_newest(channel: &Channel) {
    let retained = channel.retained.as_ref().expect("retention ran");
    assert_eq!(retained.kept.len(), 1);
    assert_eq!(retained.kept[0].build, "h3333333_2");
    assert_eq!(
        retained.pruned_paths(),
        [
            "linux-64/universe-airlock-0.1.0-h1111111_0.conda".to_owned(),
            "linux-64/universe-airlock-0.1.0-h2222222_1.conda".to_owned(),
        ]
    );
}

#[then("retention is refused because the channel would be empty")]
fn retention_zero(channel: &Channel) {
    assert!(
        matches!(channel.retention_error, Some(RetentionError::Zero)),
        "expected a refusal, got {:?}",
        channel.retention_error
    );
}

#[scenario(
    path = "features/channel.feature",
    name = "Every served subdir is answered even when it holds nothing"
)]
fn every_subdir_is_answered(channel: Channel) {
    let _ = channel;
}

#[scenario(
    path = "features/channel.feature",
    name = "The repodata repeats the numbers the audit measured"
)]
fn the_repodata_repeats_the_audit(channel: Channel) {
    let _ = channel;
}

#[scenario(
    path = "features/channel.feature",
    name = "Two renders of the same fragments are byte-identical"
)]
fn rendering_is_a_function_of_the_fragments(channel: Channel) {
    let _ = channel;
}

#[scenario(
    path = "features/channel.feature",
    name = "A fragment for a subdir the channel does not serve is refused"
)]
fn an_undeclared_subdir_is_refused(channel: Channel) {
    let _ = channel;
}

#[scenario(
    path = "features/channel.feature",
    name = "Republishing unchanged bytes writes nothing"
)]
fn an_unchanged_publish_writes_nothing(channel: Channel) {
    let _ = channel;
}

#[scenario(
    path = "features/channel.feature",
    name = "Changed bytes under an occupied name are refused"
)]
fn changed_bytes_are_refused(channel: Channel) {
    let _ = channel;
}

#[scenario(
    path = "features/channel.feature",
    name = "Retention keeps the newest builds of each version"
)]
fn retention_keeps_the_newest(channel: Channel) {
    let _ = channel;
}

#[scenario(
    path = "features/channel.feature",
    name = "Retention that would empty the channel is refused"
)]
fn retention_of_zero_is_refused(channel: Channel) {
    let _ = channel;
}
