//! Trusted publishing: exchanging a CI job's identity for a short-lived token.
//!
//! A workflow job does not need a stored credential to upload. GitHub Actions will mint a
//! signed ID token for the running job — a JWT naming the repository, the workflow file and
//! the ref — and the destination will trade that token for a bearer token scoped to itself
//! and expiring in minutes. Nothing long-lived exists to leak, which is the whole point of
//! `decision-127`.
//!
//! The exchange is two requests, and the second is the one that differs between registries:
//!
//! 1. **Ask the CI provider.** `GET $ACTIONS_ID_TOKEN_REQUEST_URL&audience=<audience>` with
//!    `Authorization: Bearer $ACTIONS_ID_TOKEN_REQUEST_TOKEN` answers `{"value": "<jwt>"}`.
//!    Both variables exist only when the job declared `id-token: write`; without it the
//!    provider answers a 4xx, so the failure names the permission rather than looking like a
//!    network problem.
//! 2. **Ask the registry.** prefix.dev's own convention — not an RFC — is
//!    `POST {server}/api/oidc/mint_token` with `{"token": "<jwt>"}`, answering the bearer
//!    token as a raw body. This mirrors `rattler_networking`'s `PrefixMint` protocol, which is
//!    what the Conda ecosystem's uploaders speak.
//!
//! The audience is not free: prefix.dev deployments validate against the literal `prefix.dev`
//! for any `*.prefix.dev` host, and every other server following the same convention scopes
//! the token to its own host name. Sending the wrong audience is a token the destination
//! refuses, so the rule lives here rather than at the call site.

use super::{StoreError, http::Http};

/// The CI variables GitHub Actions injects into a job with `id-token: write`.
const REQUEST_URL: &str = "ACTIONS_ID_TOKEN_REQUEST_URL";
/// The bearer credential for the request URL above.
const REQUEST_TOKEN: &str = "ACTIONS_ID_TOKEN_REQUEST_TOKEN";

/// The path prefix.dev mints a bearer token at.
const PREFIX_MINT_PATH: &str = "/api/oidc/mint_token";

/// The audience prefix.dev's own deployments accept.
const PREFIX_DEV_AUDIENCE: &str = "prefix.dev";

/// Somewhere a short-lived token can be obtained from.
///
/// A trait rather than a struct everywhere it is used, because the publisher's job is to ask
/// *once per destination* and to keep the answer to itself — properties worth testing without
/// a network, and impossible to test through a concrete client bound to a real CI provider.
pub trait TrustedPublisher: std::fmt::Debug {
    /// A bearer token good at `server`, or why there is none.
    ///
    /// # Errors
    ///
    /// Returns [`StoreError::Response`] when a request is answered with an error status, and
    /// [`StoreError::Transport`] when one cannot be completed.
    fn mint(&self, server: &str) -> Result<String, StoreError>;
}

/// A job's ambient CI identity, ready to be exchanged.
#[derive(Debug)]
pub struct OidcExchange {
    request_url: String,
    request_token: String,
    http: Http,
}

impl OidcExchange {
    /// The exchange this process's environment describes.
    ///
    /// # Errors
    ///
    /// Returns [`StoreError::Incomplete`] naming both variables when the job did not declare
    /// `id-token: write` — the commonest cause, and one a message about a missing token
    /// elsewhere would hide.
    pub fn from_env() -> Result<Self, StoreError> {
        Self::from_lookup(|name| std::env::var(name).ok())
    }

    /// The same, over an injected environment.
    ///
    /// # Errors
    ///
    /// As [`from_env`](Self::from_env).
    pub fn from_lookup(lookup: impl Fn(&str) -> Option<String>) -> Result<Self, StoreError> {
        let request_url = lookup(REQUEST_URL).filter(|value| !value.trim().is_empty());
        let request_token = lookup(REQUEST_TOKEN).filter(|value| !value.trim().is_empty());
        let (Some(request_url), Some(request_token)) = (request_url, request_token) else {
            return Err(StoreError::Incomplete {
                store: "trusted publishing".to_owned(),
                missing: "`id-token: write` permission: the job environment holds neither \
                          ACTIONS_ID_TOKEN_REQUEST_URL nor ACTIONS_ID_TOKEN_REQUEST_TOKEN",
            });
        };
        Ok(Self {
            request_url,
            request_token,
            http: Http::new()?,
        })
    }

    /// Trade this job's identity for a bearer token good at `server`.
    ///
    /// # Errors
    ///
    /// Returns [`StoreError::Response`] when either request is answered with an error status,
    /// and [`StoreError::Transport`] when one cannot be completed.
    fn mint_token(&self, server: &str) -> Result<String, StoreError> {
        let jwt = self.id_token(&audience_for(server))?;
        let url = format!("{}{PREFIX_MINT_PATH}", server.trim_end_matches('/'));
        let body = serde_json::json!({ "token": jwt }).to_string().into_bytes();
        let request = super::http::Request::new(super::http::Method::Post, url.clone())
            .with_header("Content-Type", "application/json")
            .with_body(body);
        let response = self.http.send(&request)?;
        if !response.is_success() {
            return Err(StoreError::Response {
                url,
                message: format!(
                    "the token exchange answered {}: {}",
                    response.status,
                    body_text(&response.body)
                ),
            });
        }
        // prefix.dev answers with the token itself, not with a JSON envelope.
        let token = String::from_utf8_lossy(&response.body).trim().to_owned();
        if token.is_empty() {
            return Err(StoreError::Response {
                url,
                message: "the token exchange answered with an empty body".to_owned(),
            });
        }
        Ok(token)
    }

    /// The CI provider's signed ID token for `audience`.
    fn id_token(&self, audience: &str) -> Result<String, StoreError> {
        // The URL already carries a query string (`?api-version=2.0`), so the audience is
        // appended rather than introduced.
        let separator = if self.request_url.contains('?') {
            '&'
        } else {
            '?'
        };
        let url = format!("{}{separator}audience={audience}", self.request_url);
        let request = super::http::Request::new(super::http::Method::Get, url.clone())
            .with_header("Authorization", format!("Bearer {}", self.request_token))
            .with_header("Accept", "application/json");
        let response = self.http.send(&request)?;
        if !response.is_success() {
            return Err(StoreError::Response {
                url,
                message: format!(
                    "the CI provider answered {}: {} (a job needs `id-token: write` to mint one)",
                    response.status,
                    body_text(&response.body)
                ),
            });
        }
        let document: IdTokenResponse =
            serde_json::from_slice(&response.body).map_err(|error| StoreError::Response {
                url: url.clone(),
                message: format!(
                    "cannot read the identity document ({error}): {}",
                    body_text(&response.body)
                ),
            })?;
        if document.value.trim().is_empty() {
            return Err(StoreError::Response {
                url,
                message: "the CI provider returned an empty identity token".to_owned(),
            });
        }
        Ok(document.value)
    }
}

impl TrustedPublisher for OidcExchange {
    fn mint(&self, server: &str) -> Result<String, StoreError> {
        self.mint_token(server)
    }
}

/// What the CI provider answers with.
#[derive(Debug, serde::Deserialize)]
struct IdTokenResponse {
    /// The signed JWT.
    value: String,
}

/// The audience to ask the CI provider for when the token is destined for `server`.
///
/// prefix.dev's deployments share one audience regardless of subdomain; anything else that
/// speaks this mint convention gets its own host name, so a token minted for one server
/// cannot be replayed at another.
#[must_use]
pub fn audience_for(server: &str) -> String {
    let host = host_of(server);
    if is_prefix_dev_host(&host) {
        PREFIX_DEV_AUDIENCE.to_owned()
    } else {
        host
    }
}

/// The host of an endpoint, without scheme, credentials, port or path.
fn host_of(server: &str) -> String {
    let without_scheme = server.split_once("://").map_or(server, |(_, rest)| rest);
    let authority = without_scheme
        .split(['/', '?', '#'])
        .next()
        .unwrap_or(without_scheme);
    let host = authority
        .rsplit_once('@')
        .map_or(authority, |(_, host)| host);
    // `host:port`, but not an IPv6 literal's own colons.
    if host.starts_with('[') {
        return host.to_owned();
    }
    host.split(':').next().unwrap_or(host).to_ascii_lowercase()
}

/// Whether `host` is a prefix.dev deployment, mirroring what the server itself accepts.
#[must_use]
pub fn is_prefix_dev_host(host: &str) -> bool {
    host == PREFIX_DEV_AUDIENCE || host.ends_with(".prefix.dev")
}

/// A response body, shortened for a message.
fn body_text(body: &[u8]) -> String {
    let text = String::from_utf8_lossy(body);
    let mut short: String = text.trim().chars().take(200).collect();
    if text.trim().chars().count() > 200 {
        short.push('…');
    }
    if short.is_empty() {
        "an empty body".to_owned()
    } else {
        short
    }
}

#[cfg(test)]
mod tests {
    use super::{OidcExchange, audience_for, body_text, host_of, is_prefix_dev_host};
    use crate::store::StoreError;

    #[test]
    fn an_endpoint_is_reduced_to_its_host() {
        assert_eq!(host_of("https://prefix.dev"), "prefix.dev");
        assert_eq!(host_of("https://prefix.dev/"), "prefix.dev");
        assert_eq!(
            host_of("https://user:pass@beta.prefix.dev:8443/api"),
            "beta.prefix.dev"
        );
        assert_eq!(host_of("http://127.0.0.1:8080"), "127.0.0.1");
        assert_eq!(host_of("https://PREFIX.DEV/x"), "prefix.dev");
    }

    #[test]
    fn the_prefix_dev_family_is_recognised_and_lookalikes_are_not() {
        for host in ["prefix.dev", "beta.prefix.dev", "staging.beta.prefix.dev"] {
            assert!(is_prefix_dev_host(host), "{host}");
        }
        for host in [
            "evil-prefix.dev",
            "prefix.dev.evil.com",
            "prefix.dev.",
            "anaconda.org",
        ] {
            assert!(!is_prefix_dev_host(host), "{host}");
        }
    }

    #[test]
    fn prefix_dev_gets_the_shared_audience_and_other_servers_their_own_host() {
        assert_eq!(audience_for("https://prefix.dev"), "prefix.dev");
        assert_eq!(audience_for("https://beta.prefix.dev/"), "prefix.dev");
        assert_eq!(
            audience_for("https://conda.example.org"),
            "conda.example.org"
        );
    }

    #[test]
    fn a_job_without_the_id_token_permission_is_told_which_permission() {
        let error = OidcExchange::from_lookup(|_| None).expect_err("no ambient identity");

        match error {
            StoreError::Incomplete { missing, .. } => {
                assert!(missing.contains("id-token: write"), "{missing}");
                assert!(
                    missing.contains("ACTIONS_ID_TOKEN_REQUEST_URL"),
                    "{missing}"
                );
            }
            other => panic!("expected an incomplete store, got {other:?}"),
        }
    }

    #[test]
    fn a_half_populated_environment_is_still_refused() {
        let error = OidcExchange::from_lookup(|name| {
            (name == "ACTIONS_ID_TOKEN_REQUEST_URL").then(|| "https://example.invalid".to_owned())
        })
        .expect_err("the request token is missing");

        assert!(matches!(error, StoreError::Incomplete { .. }), "{error:?}");
    }

    #[test]
    fn a_blank_variable_counts_as_absent() {
        let error = OidcExchange::from_lookup(|_| Some("   ".to_owned()))
            .expect_err("blank is not a value");

        assert!(matches!(error, StoreError::Incomplete { .. }), "{error:?}");
    }

    #[test]
    fn a_long_or_empty_body_is_shortened_or_named() {
        assert_eq!(body_text(b"  "), "an empty body");
        assert_eq!(body_text(b" denied "), "denied");
        let long = "a".repeat(500);
        assert_eq!(body_text(long.as_bytes()).chars().count(), 201);
    }
}
