//! GitHub Releases as an artifact store.
//!
//! Releases are not a Conda channel, but they are a place a workflow can put `.conda` files
//! without a server, so `publish-artifacts` treats them as one more destination. Three things
//! make them awkward enough to deserve their own type rather than a [`Kind`](super::Kind) arm:
//!
//! - **A release is created, not addressed.** `publish` finds the rolling release named after
//!   the subdir (`packages-linux-64`) or creates it, and a job that runs twice must land in
//!   the same release the second time. Creation is therefore only attempted after a lookup
//!   says the release is absent.
//! - **An asset upload is a `POST` to a URL the server hands back**, not a path this code can
//!   compose. The `upload_url` template from the release document is used verbatim, with the
//!   `{?name,label}` expansion cut off.
//! - **Assets are capped.** A release holds at most 1,000 assets, so `max_builds_per_version`
//!   has to prune or the channel eventually refuses every upload. Pruning happens after the
//!   uploads, grouped by `(package, version)`, and keeps the highest build numbers.

use serde::Deserialize;

use super::http::{HttpRemote, Method, Request, Response};
use super::{Policy, StoreError, Target, Upload, split_name};

/// A release, as this module needs it.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct Release {
    /// Server-assigned id, used to list and upload assets.
    pub id: u64,
    /// Tag the release is published under.
    pub tag_name: String,
    /// Template URL for an asset upload, with its `{?...}` expansion.
    pub upload_url: String,
    /// Whether GitHub has published the release.
    #[serde(default)]
    pub draft: bool,
}

/// One asset in a release.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct Asset {
    /// Server-assigned id, used to delete the asset.
    pub id: u64,
    /// File name.
    pub name: String,
    /// Size in bytes.
    #[serde(default)]
    pub size: u64,
    /// Digest the server computed, as `sha256:<hex>`, when it reports one.
    #[serde(default)]
    pub digest: Option<String>,
}

impl Asset {
    /// The digest as this crate spells it, if the server reported one.
    #[must_use]
    pub fn sha256(&self) -> Option<&str> {
        self.digest
            .as_deref()
            .and_then(|digest| digest.strip_prefix("sha256:").or(Some(digest)))
            .filter(|digest| !digest.is_empty())
    }
}

/// What publishing to a release did, or would do.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ReleaseReport {
    /// Tag published to.
    pub tag: String,
    /// Whether the release was created, or would be.
    pub created: bool,
    /// Assets that were sent.
    pub uploaded: Vec<String>,
    /// Assets the release already held, byte for byte.
    pub unchanged: Vec<String>,
    /// Assets removed by retention.
    pub pruned: Vec<String>,
    /// Bytes actually sent.
    pub bytes: u64,
    /// Whether nothing was written.
    pub dry_run: bool,
}

/// A release store: one repository, addressed as `owner/repo`.
#[derive(Debug)]
pub struct Releases {
    http: HttpRemote,
    repository: String,
}

impl Releases {
    /// A release store for the repository `target.channel` names.
    ///
    /// # Errors
    ///
    /// Returns [`StoreError::Incomplete`] when the store names no repository, and whatever
    /// [`HttpRemote::new`] returns when the endpoint cannot be spoken to.
    pub fn new(target: &Target) -> Result<Self, StoreError> {
        let repository = target
            .channel
            .as_deref()
            .map(str::trim)
            .filter(|repository| repository.split('/').count() == 2)
            .ok_or_else(|| StoreError::Incomplete {
                store: target.name.clone(),
                missing: "repository as 'owner/repo'",
            })?
            .to_owned();
        Ok(Self {
            http: HttpRemote::new(target.clone())?,
            repository,
        })
    }

    /// The repository this store publishes to.
    #[must_use]
    pub fn repository(&self) -> &str {
        &self.repository
    }

    /// The URL of one document in the repository's API.
    fn api(&self, suffix: &str) -> String {
        format!(
            "{}/repos/{}/{suffix}",
            self.http.target().endpoint.trim_end_matches('/'),
            self.repository
        )
    }

    /// Send one request, with the store's token and the API's media type.
    fn send(&self, request: Request) -> Result<Response, StoreError> {
        let mut request = request
            .with_header("Accept", "application/vnd.github+json")
            .with_header("X-GitHub-Api-Version", "2022-11-28");
        if let Some(token) = &self.http.target().token {
            request = request.with_header("Authorization", format!("Bearer {token}"));
        }
        self.http.send(&request)
    }

    /// The release tagged `tag`, if it exists.
    ///
    /// # Errors
    ///
    /// Returns [`StoreError::Transport`] when the answer is neither a release nor a `404`.
    pub fn find(&self, tag: &str) -> Result<Option<Release>, StoreError> {
        let request = Request::new(Method::Get, self.api(&format!("releases/tags/{tag}")));
        let response = self.send(request)?;
        if response.status == 404 {
            return Ok(None);
        }
        if !response.is_success() {
            return Err(StoreError::Transport {
                url: self.api(&format!("releases/tags/{tag}")),
                message: format!("GitHub answered {}", response.status),
            });
        }
        decode(&response)
    }

    /// Create the release tagged `tag`.
    ///
    /// # Errors
    ///
    /// Returns [`StoreError::Response`] when GitHub refuses, which for an existing tag means
    /// the lookup and the creation disagreed — a race two concurrent builds can lose.
    pub fn create(&self, tag: &str, title: Option<&str>) -> Result<Release, StoreError> {
        let body = serde_json::json!({
            "tag_name": tag,
            "name": title.unwrap_or(tag),
            "body": format!("Artifacts published by Universe under `{tag}`."),
        });
        let request = Request::new(Method::Post, self.api("releases"))
            .with_header("Content-Type", "application/json")
            .with_body(body.to_string().into_bytes());
        let response = self.send(request)?;
        if !response.is_success() {
            return Err(StoreError::Response {
                url: self.api("releases"),
                message: format!(
                    "GitHub answered {}: {}",
                    response.status,
                    short(&response.body)
                ),
            });
        }
        decode(&response)
    }

    /// The release tagged `tag`, created when absent — unless `dry_run`, which never creates.
    ///
    /// # Errors
    ///
    /// Propagates [`find`](Self::find) and [`create`](Self::create).
    pub fn ensure(
        &self,
        tag: &str,
        title: Option<&str>,
        dry_run: bool,
    ) -> Result<(Option<Release>, bool), StoreError> {
        if let Some(release) = self.find(tag)? {
            return Ok((Some(release), false));
        }
        if dry_run {
            return Ok((None, true));
        }
        // A second lookup after a failed create is not retried here: the caller reports the
        // refusal, and the next run starts from `find` again.
        Ok((Some(self.create(tag, title)?), true))
    }

    /// Every asset in `release`, following pagination.
    ///
    /// # Errors
    ///
    /// Returns [`StoreError::Transport`] when a page cannot be read.
    pub fn assets(&self, release: &Release) -> Result<Vec<Asset>, StoreError> {
        let mut assets: Vec<Asset> = Vec::new();
        for page in 1..=50 {
            let url = self.api(&format!(
                "releases/{}/assets?per_page=100&page={page}",
                release.id
            ));
            let response = self.send(Request::new(Method::Get, url.clone()))?;
            if !response.is_success() {
                return Err(StoreError::Transport {
                    url,
                    message: format!("GitHub answered {}", response.status),
                });
            }
            let page: Vec<Asset> = decode(&response)?;
            let count = page.len();
            assets.extend(page);
            if count < 100 {
                break;
            }
        }
        Ok(assets)
    }

    /// Upload one asset to `release`.
    ///
    /// # Errors
    ///
    /// Returns [`StoreError::Response`] when GitHub refuses the upload — most often because
    /// the release has reached its asset cap, which retention exists to prevent.
    pub fn upload(&self, release: &Release, upload: &Upload) -> Result<(), StoreError> {
        let url = upload_url(&release.upload_url, upload.file_name());
        let request = Request::new(Method::Post, url.clone())
            .with_header("Content-Type", "application/octet-stream")
            .with_body(upload.bytes.clone());
        let response = self.send(request)?;
        if response.is_success() {
            return Ok(());
        }
        Err(StoreError::Response {
            url,
            message: format!(
                "GitHub answered {}: {}",
                response.status,
                short(&response.body)
            ),
        })
    }

    /// Remove one asset.
    ///
    /// # Errors
    ///
    /// Returns [`StoreError::Response`] when GitHub refuses the deletion.
    pub fn delete(&self, asset: &Asset) -> Result<(), StoreError> {
        let url = self.api(&format!("releases/assets/{}", asset.id));
        let request = Request::new(Method::Delete, url.clone());
        let response = self.send(request)?;
        if response.is_success() || response.status == 404 {
            return Ok(());
        }
        Err(StoreError::Response {
            url,
            message: format!(
                "GitHub answered {}: {}",
                response.status,
                short(&response.body)
            ),
        })
    }

    /// Publish `uploads` to the release tagged `tag`, then apply retention.
    ///
    /// The order is deliberate: uploads first, so a build that is the newest of its version is
    /// never pruned by the same run that published it; retention last, so the asset list it
    /// groups is the one the uploads produced.
    ///
    /// # Errors
    ///
    /// Returns [`StoreError::Refused`] when an asset exists under a different digest and
    /// `policy.force` is not set.
    pub fn publish(
        &self,
        tag: &str,
        title: Option<&str>,
        uploads: &[Upload],
        policy: Policy,
        max_builds: Option<usize>,
    ) -> Result<ReleaseReport, StoreError> {
        let (release, created) = self.ensure(tag, title, policy.dry_run)?;
        let mut report = ReleaseReport {
            tag: tag.to_owned(),
            created,
            dry_run: policy.dry_run,
            ..ReleaseReport::default()
        };
        let existing = match release.as_ref() {
            Some(release) => self.assets(release)?,
            None => Vec::new(),
        };

        for upload in uploads {
            match existing
                .iter()
                .find(|asset| asset.name == upload.file_name())
            {
                Some(asset) if asset.sha256() == Some(upload.sha256.as_str()) => {
                    report.unchanged.push(upload.file_name().to_owned());
                }
                Some(asset) => {
                    if !policy.force {
                        return Err(StoreError::Refused {
                            path: upload.file_name().to_owned(),
                            published: upload.sha256.clone(),
                            present: asset
                                .sha256()
                                .map_or_else(|| "an unknown digest".to_owned(), str::to_owned),
                        });
                    }
                    if !policy.dry_run {
                        self.delete(asset)?;
                        self.upload(
                            release.as_ref().expect("a release with assets exists"),
                            upload,
                        )?;
                        report.bytes += upload.bytes.len() as u64;
                    }
                    report.uploaded.push(upload.file_name().to_owned());
                }
                None => {
                    if !policy.dry_run {
                        self.upload(
                            release.as_ref().expect("a release to upload to exists"),
                            upload,
                        )?;
                        report.bytes += upload.bytes.len() as u64;
                    }
                    report.uploaded.push(upload.file_name().to_owned());
                }
            }
        }

        if let (Some(max_builds), Some(release)) = (max_builds, release.as_ref()) {
            let assets = self.assets(release)?;
            for asset in exceeded(&assets, max_builds) {
                if !policy.dry_run {
                    self.delete(&asset)?;
                }
                report.pruned.push(asset.name.clone());
            }
        }
        Ok(report)
    }
}

/// The assets retention would remove, given how many builds of each version to keep.
#[must_use]
pub fn exceeded(assets: &[Asset], max_builds: usize) -> Vec<Asset> {
    let mut grouped: std::collections::BTreeMap<(String, String), Vec<&Asset>> =
        std::collections::BTreeMap::new();
    for asset in assets {
        let (package, version) = split_name(&asset.name);
        grouped.entry((package, version)).or_default().push(asset);
    }
    let mut doomed = Vec::new();
    for (_, mut builds) in grouped {
        // Newest build last; a name that carries no number sorts first and goes first.
        builds.sort_by(|left, right| {
            build_number(&left.name)
                .cmp(&build_number(&right.name))
                .then_with(|| left.name.cmp(&right.name))
        });
        if builds.len() > max_builds {
            let cut = builds.len() - max_builds;
            doomed.extend(builds.into_iter().take(cut).cloned());
        }
    }
    doomed.sort_by(|left, right| left.name.cmp(&right.name));
    doomed
}

/// The trailing build number of a Conda file name, or 0 when it carries none.
fn build_number(file_name: &str) -> u64 {
    let stem = file_name
        .strip_suffix(".conda")
        .or_else(|| file_name.strip_suffix(".tar.bz2"))
        .unwrap_or(file_name);
    // Three parts make a Conda name: package, version, build. Two of them mean the name
    // carries no build at all, and reading the version as one would mis-order retention.
    let mut parts = stem.rsplit('-');
    let (Some(build), Some(_version), Some(_package)) = (parts.next(), parts.next(), parts.next())
    else {
        return 0;
    };
    let digits: String = build.chars().take_while(char::is_ascii_digit).collect();
    digits.parse().unwrap_or(0)
}

/// The asset upload URL for `name`, from the release's `upload_url` template.
fn upload_url(template: &str, name: &str) -> String {
    let base = template
        .split('{')
        .next()
        .unwrap_or(template)
        .trim_end_matches('?');
    format!("{base}?name={}", encode(name))
}

/// Percent-encode a query value, keeping the characters a file name is made of.
fn encode(value: &str) -> String {
    let mut encoded = String::with_capacity(value.len());
    for byte in value.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                encoded.push(byte as char);
            }
            other => encoded.push_str(&format!("%{other:02X}")),
        }
    }
    encoded
}

/// Decode a JSON document, reporting the document when it does not parse.
fn decode<T: serde::de::DeserializeOwned>(response: &Response) -> Result<T, StoreError> {
    serde_json::from_slice(&response.body).map_err(|error| StoreError::Response {
        url: "response".to_owned(),
        message: format!(
            "cannot read the document GitHub returned ({error}): {}",
            short(&response.body)
        ),
    })
}

/// A body, shortened for a message.
fn short(body: &[u8]) -> String {
    let text = String::from_utf8_lossy(body);
    let mut short: String = text.trim().chars().take(200).collect();
    if text.trim().chars().count() > 200 {
        short.push('…');
    }
    short
}

#[cfg(test)]
mod tests {
    use super::{Asset, Release, Releases, build_number, encode, exceeded, upload_url};
    use crate::store::{Kind, StoreError, Target};

    fn asset(id: u64, name: &str) -> Asset {
        Asset {
            id,
            name: name.to_owned(),
            size: 0,
            digest: None,
        }
    }

    fn target(channel: Option<&str>) -> Target {
        Target {
            name: "releases".to_owned(),
            kind: Kind::Releases,
            endpoint: "http://127.0.0.1:8080".to_owned(),
            channel: channel.map(str::to_owned),
            token: None,
        }
    }

    #[test]
    fn an_asset_digest_loses_its_algorithm_and_an_absent_one_stays_absent() {
        let with = Asset {
            digest: Some("sha256:abc".to_owned()),
            ..asset(1, "foo-1.0.0-0.conda")
        };
        assert_eq!(with.sha256(), Some("abc"));
        let bare = Asset {
            digest: Some("abc".to_owned()),
            ..asset(2, "foo-1.0.0-0.conda")
        };
        assert_eq!(bare.sha256(), Some("abc"));
        let missing = Asset {
            digest: Some(String::new()),
            ..asset(3, "foo-1.0.0-0.conda")
        };
        assert_eq!(missing.sha256(), None);
        assert_eq!(asset(4, "foo-1.0.0-0.conda").sha256(), None);
    }

    #[test]
    fn an_upload_url_drops_its_template_and_encodes_the_file_name() {
        assert_eq!(
            upload_url(
                "http://127.0.0.1:8080/uploads/repos/acme/x/releases/7/assets{?name,label}",
                "foo-1.0.0-0.conda"
            ),
            "http://127.0.0.1:8080/uploads/repos/acme/x/releases/7/assets?name=foo-1.0.0-0.conda"
        );
        assert_eq!(encode("a b+c"), "a%20b%2Bc");
        assert_eq!(encode("a-1.0.0_0~.conda"), "a-1.0.0_0~.conda");
    }

    #[test]
    fn retention_keeps_the_highest_build_numbers_of_each_version() {
        let assets = vec![
            asset(1, "foo-1.0.0-0.conda"),
            asset(2, "foo-1.0.0-10.conda"),
            asset(3, "foo-1.0.0-2.conda"),
            asset(4, "bar-1.0.0-0.conda"),
        ];

        let doomed = exceeded(&assets, 2);

        assert_eq!(
            doomed
                .iter()
                .map(|asset| asset.name.as_str())
                .collect::<Vec<_>>(),
            vec!["foo-1.0.0-0.conda"]
        );
    }

    #[test]
    fn a_name_without_a_build_number_falls_back_to_zero() {
        let assets = vec![asset(1, "foo-1.0.0-1.conda"), asset(2, "foo-1.0.0.conda")];

        assert_eq!(build_number("foo-1.0.0.conda"), 0);
        assert_eq!(build_number("foo-1.0.0-7.conda"), 7);
        assert_eq!(build_number("foo-1.0.0-0.tar.bz2"), 0);
        assert_eq!(build_number("foo-1.0.0-py310_2.tar.bz2"), 0);
        // A name the split cannot group is left alone: "foo-1.0.0.conda" has no build, so the
        // split reads "1.0" as its version, and it becomes a group of one.
        assert!(exceeded(&assets, 1).is_empty());
    }

    #[test]
    fn nothing_is_pruned_when_every_version_fits() {
        let assets = vec![asset(1, "foo-1.0.0-0.conda"), asset(2, "bar-2.0.0-0.conda")];
        assert!(exceeded(&assets, 5).is_empty());
    }

    #[test]
    fn a_repository_must_be_declared_as_an_owner_and_a_name() {
        for channel in [None, Some("acme"), Some("acme/universe/extra")] {
            let error = Releases::new(&target(channel)).expect_err("not a repository");
            assert!(matches!(error, StoreError::Incomplete { .. }), "{error:?}");
        }
        let releases = Releases::new(&target(Some("acme/universe"))).expect("a repository");
        assert_eq!(releases.repository(), "acme/universe");
    }

    #[test]
    fn a_release_document_carries_the_tag_and_the_upload_template() {
        let release: Release = serde_json::from_str(
            r#"{"id": 7, "tag_name": "packages-linux-64",
                "upload_url": "http://127.0.0.1:8080/uploads/7/assets{?name,label}"}"#,
        )
        .expect("a release document");

        assert_eq!(release.id, 7);
        assert_eq!(release.tag_name, "packages-linux-64");
        assert!(!release.draft);
    }
}
