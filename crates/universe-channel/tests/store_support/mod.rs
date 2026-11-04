#![allow(dead_code)]

//! Shared scaffolding for the store tests.
//!
//! Each test stands a `wiremock` server in front of the real HTTP client, so the assertions
//! are about requests that actually went over a socket: the method, the path, the body. The
//! runtime is held here rather than created per test because the client under test is
//! *blocking*: the requests are issued outside the runtime, on the test thread, while the
//! mock server answers from the runtime's worker threads.

use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

/// A mock destination and the runtime that serves it.
pub struct Harness {
    runtime: tokio::runtime::Runtime,
    server: MockServer,
}

impl Harness {
    /// Start a mock destination.
    ///
    /// # Panics
    ///
    /// Panics when the runtime or the server cannot be created, which means the test cannot
    /// observe anything.
    pub fn start() -> Self {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .build()
            .expect("a tokio runtime");
        let server = runtime.block_on(MockServer::start());
        Self { runtime, server }
    }

    /// The destination's base URL, as a store would declare it in `universe.toml`.
    pub fn endpoint(&self) -> String {
        self.server.uri()
    }

    /// Register a mock before a request is made.
    pub fn mount(&self, mock: Mock) {
        self.runtime.block_on(self.server.register(mock));
    }

    /// Answer `HEAD` on `relative` as though nothing were published yet.
    pub fn nothing_published(&self, relative: &str) {
        self.mount(
            Mock::given(method("HEAD"))
                .and(path(relative))
                .respond_with(ResponseTemplate::new(404)),
        );
    }

    /// Answer `GET` on `relative` with `body`.
    pub fn get(&self, relative: &str, body: serde_json::Value) {
        self.mount(
            Mock::given(method("GET"))
                .and(path(relative))
                .respond_with(ResponseTemplate::new(200).set_body_json(body)),
        );
    }

    /// Assert every registered expectation held — including every `expect(0)`.
    ///
    /// # Panics
    ///
    /// Panics with the unmet expectations, which is the only failure a store-writer test can
    /// have on the request side.
    pub fn verify(&self) {
        self.runtime.block_on(self.server.verify());
    }
}

/// A mock that no mutating request may reach.
///
/// Returned rather than mounted so a test can say which method it is guarding; mounted with
/// `expect(0)`, a request that matches fails the expectation in [`Harness::verify`] instead of
/// being answered with the `404` an unmounted path would give — a `404` a writer could
/// mistake for "nothing there yet" and then legitimately retry around.
pub fn no_requests(verb: &str) -> Mock {
    Mock::given(method(verb))
        .respond_with(ResponseTemplate::new(200))
        .expect(0)
}

/// A mock that no mutating request may reach, narrowed to one path.
pub fn no_requests_at(verb: &str, relative: &str) -> Mock {
    Mock::given(method(verb))
        .and(path(relative))
        .respond_with(ResponseTemplate::new(200))
        .expect(0)
}

/// A `HEAD` answer carrying the digest a destination claims to hold.
pub fn head_with_digest(digest: &str) -> ResponseTemplate {
    ResponseTemplate::new(200).insert_header("x-checksum-sha256", digest)
}
