//! Speaking to a destination over HTTP.
//!
//! One blocking client, one place that composes a request, and one place that turns a
//! response into either an [`Object`] or an error. Everything a store protocol varies — the
//! path, the method, the auth header — is decided by the [`Target`], so a new destination is a
//! match arm rather than a second HTTP client.

use reqwest::blocking::Client;
use reqwest::header::{AUTHORIZATION, CONTENT_LENGTH, HeaderMap, HeaderName, HeaderValue};

use super::{Object, Remote, StoreError, Target, Upload};

/// The HTTP method one request uses.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Method {
    /// Read a document.
    Get,
    /// Read a document's metadata only.
    Head,
    /// Create or replace.
    Put,
    /// Ask the destination to create something.
    Post,
    /// Remove.
    Delete,
}

impl Method {
    /// The method as the wire spells it.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Get => "GET",
            Self::Head => "HEAD",
            Self::Put => "PUT",
            Self::Post => "POST",
            Self::Delete => "DELETE",
        }
    }

    /// Whether issuing it changes the destination.
    ///
    /// The whole of `--dry-run` rests on this predicate: a run that issues no mutating request
    /// cannot have changed anything, whatever the destination does with a read.
    #[must_use]
    pub const fn is_mutating(self) -> bool {
        !matches!(self, Self::Get | Self::Head)
    }
}

/// One HTTP request, fully composed.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Request {
    /// How to call it.
    pub method: Method,
    /// Where to call it.
    pub url: String,
    /// Headers, excluding the ones the client derives.
    pub headers: Vec<(String, String)>,
    /// Body, when the method carries one.
    pub body: Option<Vec<u8>>,
}

impl Request {
    /// A request with no body and no headers.
    #[must_use]
    pub fn new(method: Method, url: impl Into<String>) -> Self {
        Self {
            method,
            url: url.into(),
            headers: Vec::new(),
            body: None,
        }
    }

    /// The same request carrying `body`.
    #[must_use]
    pub fn with_body(mut self, body: Vec<u8>) -> Self {
        self.body = Some(body);
        self
    }

    /// The same request with one more header.
    #[must_use]
    pub fn with_header(mut self, name: &str, value: impl Into<String>) -> Self {
        self.headers.push((name.to_owned(), value.into()));
        self
    }
}

/// A response, reduced to what a store writer reads.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Response {
    /// Status code.
    pub status: u16,
    /// Headers the caller asked for, lowercased.
    pub headers: Vec<(String, String)>,
    /// Body.
    pub body: Vec<u8>,
}

impl Response {
    /// Whether the destination accepted the request.
    #[must_use]
    pub const fn is_success(&self) -> bool {
        self.status >= 200 && self.status < 300
    }

    /// One header's value, if the response carried it.
    #[must_use]
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(header, _)| header.eq_ignore_ascii_case(name))
            .map(|(_, value)| value.as_str())
    }
}

/// One blocking HTTP client, with no destination attached.
///
/// The token exchange that precedes a publish is not a store operation — it talks to the CI
/// provider and then to a mint endpoint — but it is the same client, the same request shape
/// and the same error vocabulary. Splitting it out is what lets [`super::oidc`] use it without
/// pretending a token exchange is an upload.
#[derive(Debug)]
pub struct Http {
    client: Client,
}

impl Http {
    /// A client with this crate's user agent.
    ///
    /// # Errors
    ///
    /// Returns [`StoreError::Transport`] when the client cannot be built, which means the TLS
    /// stack or the resolver is missing rather than that a destination refused anything.
    pub fn new() -> Result<Self, StoreError> {
        let client = Client::builder()
            .user_agent(concat!("universe/", env!("CARGO_PKG_VERSION")))
            .build()
            .map_err(|error| StoreError::Transport {
                url: "http client".to_owned(),
                message: format!("cannot build an HTTP client: {error}"),
            })?;
        Ok(Self { client })
    }

    /// Send one request.
    ///
    /// # Errors
    ///
    /// Returns [`StoreError::Transport`] when the request cannot be completed, and
    /// [`StoreError::Response`] when a header cannot be expressed. The status itself is *not*
    /// an error here: `404` is an answer, and only the caller knows which statuses mean what.
    pub fn send(&self, request: &Request) -> Result<Response, StoreError> {
        let mut headers = HeaderMap::new();
        for (name, value) in &request.headers {
            let name =
                HeaderName::from_bytes(name.as_bytes()).map_err(|error| StoreError::Response {
                    url: request.url.clone(),
                    message: format!("bad header name '{name}': {error}"),
                })?;
            let value = HeaderValue::from_str(value).map_err(|error| StoreError::Response {
                url: request.url.clone(),
                message: format!("bad value for header '{name}': {error}"),
            })?;
            headers.insert(name, value);
        }
        let method = match request.method {
            Method::Get => reqwest::Method::GET,
            Method::Head => reqwest::Method::HEAD,
            Method::Put => reqwest::Method::PUT,
            Method::Post => reqwest::Method::POST,
            Method::Delete => reqwest::Method::DELETE,
        };
        let mut builder = self.client.request(method, &request.url).headers(headers);
        if let Some(body) = request.body.as_ref() {
            builder = builder.body(body.clone());
        }
        let response = builder.send().map_err(|error| StoreError::Transport {
            url: request.url.clone(),
            message: error.to_string(),
        })?;

        let status = response.status().as_u16();
        let headers = response
            .headers()
            .iter()
            .map(|(name, value)| {
                (
                    name.as_str().to_owned(),
                    value.to_str().unwrap_or_default().to_owned(),
                )
            })
            .collect();
        let body = response
            .bytes()
            .map(|bytes| bytes.to_vec())
            .unwrap_or_else(|_| Vec::new());
        Ok(Response {
            status,
            headers,
            body,
        })
    }
}

/// A destination reached over HTTP.
#[derive(Debug)]
pub struct HttpRemote {
    target: Target,
    http: Http,
}

impl HttpRemote {
    /// A client for `target`.
    ///
    /// # Errors
    ///
    /// Returns [`StoreError::Unsupported`] when the endpoint's scheme is not one this build
    /// can speak. That check is here rather than at the first request because a publish that
    /// discovers a bad endpoint half-way through has already written to some destinations.
    pub fn new(target: Target) -> Result<Self, StoreError> {
        if let Some(reason) = unsupported_scheme(&target.endpoint) {
            return Err(StoreError::Unsupported {
                store: target.name.clone(),
                endpoint: target.endpoint.clone(),
                reason,
            });
        }
        Ok(Self {
            target,
            http: Http::new()?,
        })
    }

    /// The destination this client writes to.
    #[must_use]
    pub fn target(&self) -> &Target {
        &self.target
    }

    /// The absolute URL one channel path takes at this destination.
    ///
    /// # Errors
    ///
    /// Returns [`StoreError::Incomplete`] when the destination needs a channel it was not
    /// given.
    pub fn url_for(&self, upload: &Upload) -> Result<String, StoreError> {
        let location = self.target.location(upload)?;
        Ok(format!(
            "{}/{}",
            self.target.endpoint.trim_end_matches('/'),
            location
        ))
    }

    /// Send one request through this destination's client.
    ///
    /// # Errors
    ///
    /// Whatever [`Http::send`] returns.
    pub fn send(&self, request: &Request) -> Result<Response, StoreError> {
        self.http.send(request)
    }
}

impl Remote for HttpRemote {
    fn head(&self, path: &str) -> Result<Option<Object>, StoreError> {
        let upload = Upload::new(path, Vec::new());
        let mut request = Request::new(Method::Head, self.url_for(&upload)?);
        if let Some(token) = &self.target.token {
            request = request.with_header(AUTHORIZATION.as_str(), format!("Bearer {token}"));
        }
        let response = self.send(&request)?;
        if response.status == 404 {
            return Ok(None);
        }
        if !response.is_success() {
            return Err(StoreError::Transport {
                url: request.url,
                message: format!("HEAD answered {}", response.status),
            });
        }
        // Conda registries do not agree on a digest header, so this is opportunistic: a
        // destination that reports one gets the cheap unchanged check, and one that does not
        // gets a re-upload, which every registry here treats as a no-op.
        let sha256 = response
            .header("x-checksum-sha256")
            .or_else(|| response.header("x-amz-meta-sha256"))
            .map(str::to_owned);
        let size = response
            .header(CONTENT_LENGTH.as_str())
            .and_then(|value| value.parse().ok());
        Ok(Some(Object { sha256, size }))
    }

    fn put(&self, path: &str, bytes: &[u8]) -> Result<(), StoreError> {
        let upload = Upload::new(path, bytes.to_vec());
        let mut request = Request::new(self.target.kind.method(), self.url_for(&upload)?)
            .with_body(bytes.to_vec());
        if let Some(token) = &self.target.token {
            request = request.with_header(AUTHORIZATION.as_str(), format!("Bearer {token}"));
        }
        let response = self.send(&request)?;
        if response.is_success() {
            return Ok(());
        }
        Err(StoreError::Response {
            url: request.url,
            message: format!(
                "{} answered {}: {}",
                request.method.as_str(),
                response.status,
                summarise(&response.body)
            ),
        })
    }
}

/// Why an endpoint cannot be spoken to, if it cannot.
fn unsupported_scheme(endpoint: &str) -> Option<&'static str> {
    if endpoint.starts_with("https://") || endpoint.starts_with("http://") {
        return None;
    }
    Some(
        "a store is reached over http:// or https://, and a peer-to-peer or object-store URL \
         without a scheme names no host to call",
    )
}

/// A response body, shortened for a message.
fn summarise(body: &[u8]) -> String {
    let text = String::from_utf8_lossy(body);
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return "an empty body".to_owned();
    }
    let mut short: String = trimmed.chars().take(200).collect();
    if trimmed.chars().count() > 200 {
        short.push('…');
    }
    short
}

#[cfg(test)]
mod tests {
    use super::{HttpRemote, Method, Request, Response, summarise, unsupported_scheme};
    use crate::store::{Kind, Remote, StoreError, Target};

    fn target(kind: Kind, endpoint: &str, channel: Option<&str>) -> Target {
        Target {
            name: "test-store".to_owned(),
            kind,
            endpoint: endpoint.to_owned(),
            channel: channel.map(str::to_owned),
            token: None,
        }
    }

    #[test]
    fn a_read_request_does_not_mutate_and_a_write_does() {
        assert!(!Method::Get.is_mutating());
        assert!(!Method::Head.is_mutating());
        for method in [Method::Put, Method::Post, Method::Delete] {
            assert!(method.is_mutating(), "{method:?} writes");
        }
        assert_eq!(Method::Post.as_str(), "POST");
        assert_eq!(Method::Head.as_str(), "HEAD");
    }

    #[test]
    fn a_request_keeps_the_body_and_headers_it_was_given() {
        let request = Request::new(Method::Put, "http://example.invalid/artifact")
            .with_body(b"bytes".to_vec())
            .with_header("Authorization", "Bearer token");

        assert_eq!(request.body.as_deref(), Some(b"bytes".as_slice()));
        assert_eq!(
            request.headers,
            vec![("Authorization".to_owned(), "Bearer token".to_owned())]
        );
    }

    #[test]
    fn a_response_reports_only_the_headers_it_carries() {
        let response = Response {
            status: 201,
            headers: vec![("X-Checksum-Sha256".to_owned(), "abc".to_owned())],
            body: Vec::new(),
        };

        assert!(response.is_success());
        assert_eq!(response.header("x-checksum-sha256"), Some("abc"));
        assert_eq!(response.header("content-length"), None);
        assert!(
            !Response {
                status: 404,
                ..response
            }
            .is_success()
        );
    }

    #[test]
    fn a_scheme_that_names_no_host_to_call_is_refused() {
        assert_eq!(unsupported_scheme("https://prefix.dev"), None);
        assert_eq!(unsupported_scheme("http://127.0.0.1:8080"), None);
        for refused in ["ftp://example.invalid", "s3://bucket", "127.0.0.1:8080", ""] {
            assert!(unsupported_scheme(refused).is_some(), "{refused}");
        }
    }

    #[test]
    fn an_https_destination_is_reachable_because_the_build_links_rustls() {
        let remote = HttpRemote::new(target(Kind::PrefixDev, "https://prefix.dev", Some("acme")))
            .expect("an https destination is speakable");

        let url = remote
            .url_for(&crate::store::Upload::new(
                "linux-64/foo-1.0.0-0.conda",
                Vec::new(),
            ))
            .expect("a channel path");

        assert_eq!(url, "https://prefix.dev/api/v1/upload/acme");
    }

    #[test]
    fn an_https_connection_that_cannot_be_made_is_a_transport_error_not_a_panic() {
        // Nothing listens on port 1, so the handshake never starts: the point of the test is
        // that the TLS stack is wired in far enough to attempt one and report it as transport.
        let remote = HttpRemote::new(target(Kind::S3, "https://127.0.0.1:1", Some("bucket")))
            .expect("an https destination is speakable");

        let error = remote
            .head("linux-64/foo-1.0.0-0.conda")
            .expect_err("a closed port is a transport failure");

        assert!(matches!(error, StoreError::Transport { .. }), "{error:?}");
    }

    #[test]
    fn a_long_body_is_shortened_in_a_message_and_an_empty_one_is_named() {
        assert_eq!(summarise(b"  "), "an empty body");
        assert_eq!(summarise(b"denied"), "denied");
        let long = "a".repeat(500);
        let short = summarise(long.as_bytes());
        assert_eq!(short.chars().count(), 201, "200 characters and an ellipsis");
        assert!(short.ends_with('\u{2026}'));
    }

    #[test]
    fn a_destination_that_declares_no_channel_cannot_address_an_upload() {
        let remote = HttpRemote::new(target(Kind::PrefixDev, "http://127.0.0.1:1", None))
            .expect("a client is built even when the channel is missing");

        let error = remote
            .url_for(&crate::store::Upload::new(
                "linux-64/foo-1.0.0-0.conda",
                Vec::new(),
            ))
            .expect_err("no channel means no address");

        assert!(matches!(error, StoreError::Incomplete { .. }), "{error:?}");
    }

    #[test]
    fn an_upload_url_joins_the_endpoint_and_the_channel_path() {
        let remote = HttpRemote::new(target(
            Kind::PrefixDev,
            "http://127.0.0.1:8080/",
            Some("my-channel"),
        ))
        .expect("a client");

        let url = remote
            .url_for(&crate::store::Upload::new(
                "linux-64/foo-1.0.0-0.conda",
                Vec::new(),
            ))
            .expect("a channel path");

        assert_eq!(url, "http://127.0.0.1:8080/api/v1/upload/my-channel");
    }

    #[test]
    fn a_header_that_cannot_be_a_header_is_refused_before_a_request_is_made() {
        let remote = HttpRemote::new(target(Kind::S3, "http://127.0.0.1:1", Some("bucket")))
            .expect("a client");

        let error = remote
            .send(
                &Request::new(Method::Get, "http://127.0.0.1:1/bucket/x")
                    .with_header("bad header", "value"),
            )
            .expect_err("a space is not allowed in a header name");

        assert!(matches!(error, StoreError::Response { .. }), "{error:?}");
    }

    #[test]
    fn a_destination_that_refuses_the_connection_is_a_transport_error() {
        let remote = HttpRemote::new(target(Kind::S3, "http://127.0.0.1:1", Some("bucket")))
            .expect("a client");

        let error = remote
            .head("linux-64/foo-1.0.0-0.conda")
            .expect_err("nothing listens on port 1");

        assert!(matches!(error, StoreError::Transport { .. }), "{error:?}");
    }
}
