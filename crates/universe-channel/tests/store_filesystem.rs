//! The filesystem store: the same three rules as a registry, without a socket.

use std::fs;

use universe_channel::store::{FilesystemRemote, Policy, Remote, StoreError, Upload, apply};

fn upload(name: &str, bytes: &[u8]) -> Upload {
    Upload::new(name, bytes.to_vec())
}

#[test]
fn an_upload_lands_under_its_subdir() {
    let directory = tempfile::tempdir().expect("a scratch directory");
    let remote = FilesystemRemote::new(directory.path());

    let report = apply(
        &remote,
        &[upload("linux-64/foo-1.0.0-0.conda", b"artifact")],
        Policy::default(),
    )
    .expect("the artifact is written");

    assert_eq!(
        report.uploaded,
        vec!["linux-64/foo-1.0.0-0.conda".to_owned()]
    );
    assert_eq!(
        fs::read(directory.path().join("linux-64/foo-1.0.0-0.conda")).expect("the artifact"),
        b"artifact"
    );
}

#[test]
fn a_second_publish_of_the_same_bytes_writes_nothing() {
    let directory = tempfile::tempdir().expect("a scratch directory");
    let remote = FilesystemRemote::new(directory.path());
    let artifacts = [upload("noarch/foo-1.0.0-0.conda", b"artifact")];

    apply(&remote, &artifacts, Policy::default()).expect("the first publish");
    let report = apply(&remote, &artifacts, Policy::default()).expect("the second publish");

    assert!(report.is_noop());
    assert_eq!(
        report.unchanged,
        vec!["noarch/foo-1.0.0-0.conda".to_owned()]
    );
}

#[test]
fn changed_bytes_under_an_occupied_name_are_refused_and_the_file_kept() {
    let directory = tempfile::tempdir().expect("a scratch directory");
    let remote = FilesystemRemote::new(directory.path());
    apply(
        &remote,
        &[upload("noarch/foo-1.0.0-0.conda", b"first")],
        Policy::default(),
    )
    .expect("the first publish");

    let error = apply(
        &remote,
        &[upload("noarch/foo-1.0.0-0.conda", b"second")],
        Policy::default(),
    )
    .expect_err("a changed artifact is refused");

    assert!(matches!(error, StoreError::Refused { .. }));
    assert_eq!(
        fs::read(directory.path().join("noarch/foo-1.0.0-0.conda")).expect("the artifact"),
        b"first",
        "a refusal leaves the published artifact alone"
    );
}

#[test]
fn force_replaces_the_artifact() {
    let directory = tempfile::tempdir().expect("a scratch directory");
    let remote = FilesystemRemote::new(directory.path());
    apply(
        &remote,
        &[upload("noarch/foo-1.0.0-0.conda", b"first")],
        Policy::default(),
    )
    .expect("the first publish");

    apply(
        &remote,
        &[upload("noarch/foo-1.0.0-0.conda", b"second")],
        Policy {
            force: true,
            dry_run: false,
        },
    )
    .expect("--force replaces the artifact");

    assert_eq!(
        fs::read(directory.path().join("noarch/foo-1.0.0-0.conda")).expect("the artifact"),
        b"second"
    );
}

#[test]
fn a_dry_run_creates_nothing() {
    let directory = tempfile::tempdir().expect("a scratch directory");
    let remote = FilesystemRemote::new(directory.path());

    let report = apply(
        &remote,
        &[upload("linux-64/foo-1.0.0-0.conda", b"artifact")],
        Policy {
            force: false,
            dry_run: true,
        },
    )
    .expect("a dry run plans the upload");

    assert!(report.dry_run);
    assert_eq!(report.uploaded.len(), 1);
    assert_eq!(report.bytes, 0);
    assert!(
        !directory.path().join("linux-64").exists(),
        "a dry run creates no directory"
    );
}

#[test]
fn a_path_that_climbs_out_of_the_channel_is_refused() {
    let directory = tempfile::tempdir().expect("a scratch directory");
    let remote = FilesystemRemote::new(directory.path());

    let error = remote
        .put("../escaped.conda", b"artifact", false)
        .expect_err("a climbing path is refused");

    assert!(matches!(error, StoreError::Unsupported { .. }));
    assert!(
        !directory
            .path()
            .parent()
            .expect("a parent")
            .join("escaped.conda")
            .exists()
    );
}

#[test]
fn a_written_artifact_leaves_no_staging_file_behind() {
    let directory = tempfile::tempdir().expect("a scratch directory");
    let remote = FilesystemRemote::new(directory.path());

    remote
        .put("linux-64/foo-1.0.0-0.conda", b"artifact", false)
        .expect("the artifact is written");

    let names: Vec<String> = fs::read_dir(directory.path().join("linux-64"))
        .expect("the subdir")
        .map(|entry| {
            entry
                .expect("an entry")
                .file_name()
                .to_string_lossy()
                .into_owned()
        })
        .collect();
    assert_eq!(names, vec!["foo-1.0.0-0.conda".to_owned()]);
}
