//! The release store: find-or-create, idempotent upload, retention.

mod store_support;

use store_support::{Harness, no_requests, no_requests_at};
use universe_channel::store::{Policy, Releases, Target, Upload};
use wiremock::matchers::{body_partial_json, method, path, query_param};
use wiremock::{Mock, ResponseTemplate};

/// The document GitHub answers a release lookup with.
fn release_json(tag: &str, id: u64) -> serde_json::Value {
    // `name` and `body` are what the lookup has to report for `publish` to know whether the
    // front page is stale; a document without them reads as unknown, which is an update.
    serde_json::json!({
        "id": id,
        "tag_name": tag,
        "name": format!("{tag} artifacts"),
        "body": format!("Artifacts published by Universe under `{tag}`."),
        "upload_url": format!("http://placeholder/uploads/repos/acme/universe/releases/{id}/assets{{?name,label}}"),
        "draft": false,
    })
}

/// An asset document as the release's asset list reports it.
fn asset(id: u64, name: &str, digest: Option<String>) -> serde_json::Value {
    serde_json::json!({
        "id": id,
        "name": name,
        "size": 3,
        "digest": digest,
    })
}

fn store(harness: &Harness) -> Releases {
    Releases::new(&Target {
        name: "releases".to_owned(),
        kind: universe_channel::store::Kind::Releases,
        endpoint: harness.endpoint(),
        channel: Some("acme/universe".to_owned()),
        token: Some("token".to_owned()),
    })
    .expect("a release store")
}

#[test]
fn a_missing_release_is_created_and_its_assets_uploaded() {
    let harness = Harness::start();
    let tag = "packages-linux-64";
    harness.mount(
        Mock::given(method("GET"))
            .and(path(format!("/repos/acme/universe/releases/tags/{tag}")))
            .respond_with(ResponseTemplate::new(404)),
    );
    let mut created = release_json(tag, 7);
    created["upload_url"] = serde_json::json!(format!(
        "{}/uploads/repos/acme/universe/releases/7/assets{{?name,label}}",
        harness.endpoint()
    ));
    harness.mount(
        Mock::given(method("POST"))
            .and(path("/repos/acme/universe/releases"))
            .respond_with(ResponseTemplate::new(201).set_body_json(created))
            .expect(1),
    );
    harness.get(
        "/repos/acme/universe/releases/7/assets",
        serde_json::json!([]),
    );
    harness.mount(
        Mock::given(method("POST"))
            .and(path("/uploads/repos/acme/universe/releases/7/assets"))
            .and(query_param("name", "foo-1.0.0-0.conda"))
            .respond_with(ResponseTemplate::new(201))
            .expect(1),
    );

    let report = store(&harness)
        .publish(
            tag,
            Some("linux-64 packages"),
            None,
            &[Upload::new("linux-64/foo-1.0.0-0.conda", b"abc".to_vec())],
            Policy::default(),
            None,
        )
        .expect("the release is created and filled");

    assert!(report.created);
    assert_eq!(report.uploaded, vec!["foo-1.0.0-0.conda".to_owned()]);
    harness.verify();
}

#[test]
fn an_existing_release_is_reused_and_no_second_release_is_created() {
    let harness = Harness::start();
    let tag = "packages-linux-64";
    harness.get(
        &format!("/repos/acme/universe/releases/tags/{tag}"),
        release_json(tag, 7),
    );
    harness.get(
        "/repos/acme/universe/releases/7/assets",
        serde_json::json!([]),
    );
    harness.mount(no_requests_at("POST", "/repos/acme/universe/releases"));

    let report = store(&harness)
        .publish(tag, None, None, &[], Policy::default(), None)
        .expect("the existing release is used");

    assert!(!report.created);
    harness.verify();
}

#[test]
fn an_asset_with_the_same_digest_is_not_uploaded_again() {
    let harness = Harness::start();
    let tag = "packages-linux-64";
    let upload = Upload::new("linux-64/foo-1.0.0-0.conda", b"abc".to_vec());
    harness.get(
        &format!("/repos/acme/universe/releases/tags/{tag}"),
        release_json(tag, 7),
    );
    harness.get(
        "/repos/acme/universe/releases/7/assets",
        serde_json::json!([asset(
            1,
            "foo-1.0.0-0.conda",
            Some(format!("sha256:{}", upload.sha256))
        )]),
    );
    harness.mount(no_requests_at(
        "POST",
        "/uploads/repos/acme/universe/releases/7/assets",
    ));

    let report = store(&harness)
        .publish(tag, None, None, &[upload], Policy::default(), None)
        .expect("the asset is already published");

    assert_eq!(report.unchanged, vec!["foo-1.0.0-0.conda".to_owned()]);
    assert!(report.uploaded.is_empty());
    harness.verify();
}

#[test]
fn an_asset_with_a_different_digest_is_refused_without_force() {
    let harness = Harness::start();
    let tag = "packages-linux-64";
    harness.get(
        &format!("/repos/acme/universe/releases/tags/{tag}"),
        release_json(tag, 7),
    );
    harness.get(
        "/repos/acme/universe/releases/7/assets",
        serde_json::json!([asset(
            1,
            "foo-1.0.0-0.conda",
            Some("sha256:deadbeef".to_owned())
        )]),
    );
    harness.mount(no_requests_at(
        "POST",
        "/uploads/repos/acme/universe/releases/7/assets",
    ));
    harness.mount(no_requests("DELETE"));

    let error = store(&harness)
        .publish(
            tag,
            None,
            None,
            &[Upload::new("linux-64/foo-1.0.0-0.conda", b"abc".to_vec())],
            Policy::default(),
            None,
        )
        .expect_err("a different digest under an occupied name is refused");

    assert!(matches!(
        error,
        universe_channel::store::StoreError::Refused { .. }
    ));
    harness.verify();
}

#[test]
fn retention_prunes_the_oldest_builds_of_a_version() {
    let harness = Harness::start();
    let tag = "packages-linux-64";
    harness.get(
        &format!("/repos/acme/universe/releases/tags/{tag}"),
        release_json(tag, 7),
    );
    harness.get(
        "/repos/acme/universe/releases/7/assets",
        serde_json::json!([
            asset(1, "foo-1.0.0-0.conda", None),
            asset(2, "foo-1.0.0-1.conda", None),
            asset(3, "foo-1.0.0-2.conda", None),
            asset(4, "bar-2.0.0-0.conda", None),
        ]),
    );
    harness.mount(
        Mock::given(method("DELETE"))
            .and(path("/repos/acme/universe/releases/assets/1"))
            .respond_with(ResponseTemplate::new(204))
            .expect(1),
    );
    harness.mount(no_requests_at(
        "DELETE",
        "/repos/acme/universe/releases/assets/2",
    ));
    harness.mount(no_requests_at(
        "DELETE",
        "/repos/acme/universe/releases/assets/3",
    ));

    let report = store(&harness)
        .publish(tag, None, None, &[], Policy::default(), Some(2))
        .expect("retention keeps two builds");

    assert_eq!(report.pruned, vec!["foo-1.0.0-0.conda".to_owned()]);
    harness.verify();
}

#[test]
fn a_dry_run_creates_no_release_and_uploads_nothing() {
    let harness = Harness::start();
    let tag = "packages-linux-64";
    harness.mount(
        Mock::given(method("GET"))
            .and(path(format!("/repos/acme/universe/releases/tags/{tag}")))
            .respond_with(ResponseTemplate::new(404)),
    );
    harness.mount(no_requests("POST"));
    harness.mount(no_requests("DELETE"));

    let report = store(&harness)
        .publish(
            tag,
            None,
            None,
            &[Upload::new("linux-64/foo-1.0.0-0.conda", b"abc".to_vec())],
            Policy {
                force: false,
                dry_run: true,
            },
            Some(1),
        )
        .expect("a dry run plans the release");

    assert!(report.dry_run);
    assert!(report.created, "the release would be created");
    assert_eq!(report.uploaded, vec!["foo-1.0.0-0.conda".to_owned()]);
    assert_eq!(report.bytes, 0);
    harness.verify();
}

/// A release whose front page is what the caller asked for is left alone.
///
/// This is the idempotence the rolling release depends on: four packages of one version are
/// four `publish-artifacts` calls into the same release, and only the first may write its name.
#[test]
fn a_release_already_named_and_described_is_not_rewritten() {
    let harness = Harness::start();
    let tag = "universe-linux-64";
    let mut existing = release_json(tag, 7);
    existing["name"] = serde_json::json!("Universe 0.1.0 (linux-64)");
    existing["body"] = serde_json::json!("## 0.1.0\n\nThe notes.");
    harness.get(
        &format!("/repos/acme/universe/releases/tags/{tag}"),
        existing,
    );
    harness.get(
        "/repos/acme/universe/releases/7/assets",
        serde_json::json!([]),
    );
    harness.mount(no_requests_at("PATCH", "/repos/acme/universe/releases/7"));
    harness.mount(no_requests("POST"));

    let report = store(&harness)
        .publish(
            tag,
            Some("Universe 0.1.0 (linux-64)"),
            Some("## 0.1.0\n\nThe notes."),
            &[],
            Policy::default(),
            None,
        )
        .expect("the existing release is used");

    assert!(!report.created);
    assert!(!report.updated);
    harness.verify();
}

/// A rolling release whose name and body were written by the previous version is stale, and a
/// name that says `0.1.0` while the release holds `0.1.1` is worse than no name at all.
#[test]
fn a_stale_name_and_body_are_rewritten_on_the_rolling_release() {
    let harness = Harness::start();
    let tag = "universe-linux-64";
    let mut existing = release_json(tag, 7);
    existing["name"] = serde_json::json!("Universe 0.1.0 (linux-64)");
    existing["body"] = serde_json::json!("old notes");
    harness.get(
        &format!("/repos/acme/universe/releases/tags/{tag}"),
        existing,
    );
    harness.mount(
        Mock::given(method("PATCH"))
            .and(path("/repos/acme/universe/releases/7"))
            .and(body_partial_json(serde_json::json!({
                "name": "Universe 0.1.1 (linux-64)",
                "body": "new notes",
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(release_json(tag, 7)))
            .expect(1),
    );
    harness.get(
        "/repos/acme/universe/releases/7/assets",
        serde_json::json!([]),
    );
    harness.mount(no_requests_at("POST", "/repos/acme/universe/releases"));

    let report = store(&harness)
        .publish(
            tag,
            Some("Universe 0.1.1 (linux-64)"),
            Some("new notes"),
            &[],
            Policy::default(),
            None,
        )
        .expect("the release is renamed in place");

    assert!(!report.created);
    assert!(report.updated);
    harness.verify();
}

/// A created release carries the notes, rather than a sentence the publisher made up.
#[test]
fn a_created_release_carries_the_body_it_was_given() {
    let harness = Harness::start();
    let tag = "universe-linux-64";
    harness.mount(
        Mock::given(method("GET"))
            .and(path(format!("/repos/acme/universe/releases/tags/{tag}")))
            .respond_with(ResponseTemplate::new(404)),
    );
    harness.mount(
        Mock::given(method("POST"))
            .and(path("/repos/acme/universe/releases"))
            .and(body_partial_json(serde_json::json!({
                "tag_name": tag,
                "name": "Universe 0.1.0 (linux-64)",
                "body": "## 0.1.0\n\nWhat this tag contains.",
            })))
            .respond_with(ResponseTemplate::new(201).set_body_json(release_json(tag, 7)))
            .expect(1),
    );
    harness.get(
        "/repos/acme/universe/releases/7/assets",
        serde_json::json!([]),
    );

    let report = store(&harness)
        .publish(
            tag,
            Some("Universe 0.1.0 (linux-64)"),
            Some("## 0.1.0\n\nWhat this tag contains."),
            &[],
            Policy::default(),
            None,
        )
        .expect("the release is created with its notes");

    assert!(report.created);
    harness.verify();
}
