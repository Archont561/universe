//! What a store writer sends, over a socket, for each destination kind.
//!
//! Three behaviours are asserted for every registry: the path and method the destination
//! expects; that an artifact the destination already holds is not sent again; and that a dry
//! run reads without writing. The last one is asserted with a mock that fails if a single
//! mutating request arrives, so it is a property of the traffic and not of a flag.

mod store_support;

use store_support::{Harness, head_with_digest, no_requests};
use universe_channel::store::{HttpRemote, Kind, Policy, StoreError, Target, Upload, apply};
use wiremock::matchers::{body_bytes, method, path};
use wiremock::{Mock, ResponseTemplate};

/// A store declaration, as `universe.toml` would give it.
fn target(harness: &Harness, kind: Kind, channel: &str) -> Target {
    Target {
        name: "test-store".to_owned(),
        kind,
        endpoint: harness.endpoint(),
        channel: Some(channel.to_owned()),
        token: Some("token".to_owned()),
    }
}

#[test]
fn a_prefix_dev_upload_posts_each_artifact_to_the_channel() {
    let harness = Harness::start();
    harness.nothing_published("/api/v1/upload/my-channel");
    harness.mount(
        Mock::given(method("POST"))
            .and(path("/api/v1/upload/my-channel"))
            .and(body_bytes(b"artifact bytes".to_vec()))
            .respond_with(ResponseTemplate::new(201))
            .expect(1),
    );

    let remote = HttpRemote::new(target(&harness, Kind::PrefixDev, "my-channel"))
        .expect("a client for the store");
    let uploads = vec![Upload::new(
        "linux-64/foo-1.0.0-0.conda",
        b"artifact bytes".to_vec(),
    )];
    let report = apply(&remote, &uploads, Policy::default()).expect("the upload succeeds");

    assert_eq!(
        report.uploaded,
        vec!["linux-64/foo-1.0.0-0.conda".to_owned()]
    );
    assert_eq!(report.bytes, 14);
    harness.verify();
}

#[test]
fn an_artifact_the_registry_already_holds_is_not_sent_again() {
    let harness = Harness::start();
    let bytes = b"artifact bytes".to_vec();
    let upload = Upload::new("linux-64/foo-1.0.0-0.conda", bytes);
    harness.mount(
        Mock::given(method("HEAD"))
            .and(path("/api/v1/upload/my-channel"))
            .respond_with(head_with_digest(&upload.sha256)),
    );
    harness.mount(no_requests("POST"));

    let remote = HttpRemote::new(target(&harness, Kind::PrefixDev, "my-channel"))
        .expect("a client for the store");
    let report = apply(&remote, std::slice::from_ref(&upload), Policy::default())
        .expect("the upload is skipped");

    assert!(report.is_noop());
    assert_eq!(
        report.unchanged,
        vec!["linux-64/foo-1.0.0-0.conda".to_owned()]
    );
    assert_eq!(report.bytes, 0);
    harness.verify();
}

#[test]
fn an_anaconda_upload_posts_to_the_owner() {
    let harness = Harness::start();
    harness.nothing_published("/upload/my-owner");
    harness.mount(
        Mock::given(method("POST"))
            .and(path("/upload/my-owner"))
            .respond_with(ResponseTemplate::new(201))
            .expect(1),
    );

    let remote = HttpRemote::new(target(&harness, Kind::Anaconda, "my-owner")).expect("a client");
    apply(
        &remote,
        &[Upload::new("noarch/foo-1.0.0-0.conda", b"x".to_vec())],
        Policy::default(),
    )
    .expect("the upload succeeds");

    harness.verify();
}

#[test]
fn a_quetz_upload_is_filed_under_its_subdir() {
    let harness = Harness::start();
    harness.nothing_published("/api/channels/my-channel/upload/osx-arm64");
    harness.mount(
        Mock::given(method("POST"))
            .and(path("/api/channels/my-channel/upload/osx-arm64"))
            .respond_with(ResponseTemplate::new(201))
            .expect(1),
    );

    let remote = HttpRemote::new(target(&harness, Kind::Quetz, "my-channel")).expect("a client");
    apply(
        &remote,
        &[Upload::new("osx-arm64/foo-1.0.0-0.conda", b"x".to_vec())],
        Policy::default(),
    )
    .expect("the upload succeeds");

    harness.verify();
}

#[test]
fn an_s3_store_puts_at_the_channel_and_path() {
    let harness = Harness::start();
    harness.nothing_published("/bucket/linux-64/foo-1.0.0-0.conda");
    harness.mount(
        Mock::given(method("PUT"))
            .and(path("/bucket/linux-64/foo-1.0.0-0.conda"))
            .and(body_bytes(b"x".to_vec()))
            .respond_with(ResponseTemplate::new(200))
            .expect(1),
    );

    let remote = HttpRemote::new(target(&harness, Kind::S3, "bucket")).expect("a client");
    apply(
        &remote,
        &[Upload::new("linux-64/foo-1.0.0-0.conda", b"x".to_vec())],
        Policy::default(),
    )
    .expect("the upload succeeds");

    harness.verify();
}

#[test]
fn an_artifactory_store_puts_at_the_channel_and_path() {
    let harness = Harness::start();
    harness.nothing_published("/conda/win-64/foo-1.0.0-0.conda");
    harness.mount(
        Mock::given(method("PUT"))
            .and(path("/conda/win-64/foo-1.0.0-0.conda"))
            .respond_with(ResponseTemplate::new(201))
            .expect(1),
    );

    let remote = HttpRemote::new(target(&harness, Kind::Artifactory, "conda")).expect("a client");
    apply(
        &remote,
        &[Upload::new("win-64/foo-1.0.0-0.conda", b"x".to_vec())],
        Policy::default(),
    )
    .expect("the upload succeeds");

    harness.verify();
}

#[test]
fn a_changed_digest_under_an_occupied_name_is_refused() {
    let harness = Harness::start();
    harness.mount(
        Mock::given(method("HEAD"))
            .and(path("/api/v1/upload/my-channel"))
            .respond_with(head_with_digest(
                "0000000000000000000000000000000000000000000000000000000000000000",
            )),
    );
    harness.mount(no_requests("POST"));

    let remote = HttpRemote::new(target(&harness, Kind::PrefixDev, "my-channel"))
        .expect("a client for the store");
    let error = apply(
        &remote,
        &[Upload::new(
            "linux-64/foo-1.0.0-0.conda",
            b"new bytes".to_vec(),
        )],
        Policy::default(),
    )
    .expect_err("a republished name with new bytes is refused");

    match error {
        StoreError::Refused {
            path,
            published,
            present,
        } => {
            assert_eq!(path, "linux-64/foo-1.0.0-0.conda");
            assert_ne!(published, present);
            assert_eq!(present, "0".repeat(64));
        }
        other => panic!("expected a refusal, got {other:?}"),
    }
    harness.verify();
}

#[test]
fn force_replaces_a_changed_digest() {
    let harness = Harness::start();
    harness.mount(
        Mock::given(method("HEAD"))
            .and(path("/api/v1/upload/my-channel"))
            .respond_with(head_with_digest(
                "0000000000000000000000000000000000000000000000000000000000000000",
            )),
    );
    harness.mount(
        Mock::given(method("POST"))
            .and(path("/api/v1/upload/my-channel"))
            .respond_with(ResponseTemplate::new(201))
            .expect(1),
    );

    let remote = HttpRemote::new(target(&harness, Kind::PrefixDev, "my-channel"))
        .expect("a client for the store");
    let report = apply(
        &remote,
        &[Upload::new(
            "linux-64/foo-1.0.0-0.conda",
            b"new bytes".to_vec(),
        )],
        Policy {
            force: true,
            dry_run: false,
        },
    )
    .expect("--force replaces the artifact");

    assert_eq!(report.uploaded.len(), 1);
    harness.verify();
}

#[test]
fn a_dry_run_reads_but_never_writes() {
    let harness = Harness::start();
    harness.nothing_published("/api/v1/upload/my-channel");
    harness.mount(no_requests("POST"));
    harness.mount(no_requests("PUT"));
    harness.mount(no_requests("DELETE"));

    let remote = HttpRemote::new(target(&harness, Kind::PrefixDev, "my-channel"))
        .expect("a client for the store");
    let report = apply(
        &remote,
        &[
            Upload::new("linux-64/foo-1.0.0-0.conda", b"one".to_vec()),
            Upload::new("noarch/foo-1.0.0-0.conda", b"two".to_vec()),
        ],
        Policy {
            force: false,
            dry_run: true,
        },
    )
    .expect("a dry run plans the uploads");

    assert!(report.dry_run);
    assert_eq!(report.uploaded.len(), 2);
    assert_eq!(report.bytes, 0, "a dry run transfers nothing");
    harness.verify();
}

#[test]
fn an_endpoint_without_a_scheme_is_refused_before_a_single_request() {
    let harness = Harness::start();
    let mut target = target(&harness, Kind::PrefixDev, "my-channel");
    target.endpoint = "prefix.dev/api".to_owned();

    let error = HttpRemote::new(target).expect_err("a schemeless endpoint names no host to call");

    match error {
        StoreError::Unsupported { endpoint, .. } => assert_eq!(endpoint, "prefix.dev/api"),
        other => panic!("expected an unsupported scheme, got {other:?}"),
    }
    harness.verify();
}

#[test]
fn an_https_destination_is_addressed_over_tls() {
    // The scheme is accepted because the build links `rustls`; the assertion is on the
    // address, since a `wiremock` server speaks plaintext and cannot answer a handshake.
    let harness = Harness::start();
    let mut target = target(&harness, Kind::PrefixDev, "my-channel");
    target.endpoint = "https://prefix.dev".to_owned();
    let remote = HttpRemote::new(target).expect("an https destination is speakable");

    let url = remote
        .url_for(&Upload::new("linux-64/foo-1.0.0-0.conda", Vec::new()))
        .expect("a channel path");

    assert_eq!(url, "https://prefix.dev/api/v1/upload/my-channel");
    harness.verify();
}
