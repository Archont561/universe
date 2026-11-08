//! A publish to a hosted channel, with only the far end of the socket faked.
//!
//! What runs here is everything but the destination: the manifest is parsed, the fragment is
//! read, the artifact's digest is verified against it, the store's endpoint and credentials
//! are resolved, the blocking HTTP client is built, and the upload goes over a real socket.
//! The destination is a socket the test owns, which is the one thing it may substitute — it is
//! the only part of a publish that needs an account.
//!
//! The mock is written against `std::net` rather than pulled in as a dependency, deliberately:
//! a dev-dependency would change `Cargo.lock`, and the lock is written by the relock workflow
//! rather than by hand. Everything the test needs is a request line, headers, a body, and a
//! canned response.
//!
//! This is the shape a synthetic tag run needs. `universe-publish.yml` is triggered by a tag,
//! so before the first real tag the pipeline can be exercised end to end by pointing a store's
//! `url` at a mock and running the same verb the workflow runs. What it cannot prove is that
//! prefix.dev accepts the artifact, or that GitHub's OIDC exchange mints a token — those still
//! need the real destination, and `UNI-59` owes them.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use serde_json::Value;
use tempfile::TempDir;
use universe_kit::run;

/// One request as it arrived, which is what the assertions are about.
#[derive(Clone, Debug)]
struct Recorded {
    method: String,
    path: String,
    body: Vec<u8>,
}

/// How the mock answers.
#[derive(Clone)]
struct Answers {
    /// What `HEAD` reports: a status and, when present, the digest the destination holds.
    head: (u16, Option<String>),
}

/// A destination on a loopback port, recording what it was sent.
struct Destination {
    address: SocketAddr,
    recorded: Arc<Mutex<Vec<Recorded>>>,
    stopping: Arc<AtomicBool>,
}

impl Destination {
    fn start(answers: Answers) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").expect("a loopback port");
        let address = listener.local_addr().expect("the bound address");
        let recorded = Arc::new(Mutex::new(Vec::new()));
        let stopping = Arc::new(AtomicBool::new(false));

        let handler_records = Arc::clone(&recorded);
        let handler_stopping = Arc::clone(&stopping);
        listener
            .set_nonblocking(true)
            .expect("a non-blocking listener");
        std::thread::spawn(move || {
            while !handler_stopping.load(Ordering::SeqCst) {
                match listener.accept() {
                    Ok((stream, _)) => {
                        answer(stream, &answers, &handler_records);
                    }
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        std::thread::sleep(Duration::from_millis(2));
                    }
                    Err(_) => break,
                }
            }
        });

        Self {
            address,
            recorded,
            stopping,
        }
    }

    /// The base URL a store would declare in `universe.toml`.
    fn endpoint(&self) -> String {
        format!("http://{}", self.address)
    }

    fn requests(&self) -> Vec<Recorded> {
        self.recorded.lock().expect("the record").clone()
    }
}

impl Drop for Destination {
    fn drop(&mut self) {
        self.stopping.store(true, Ordering::SeqCst);
        // Wake the accept loop so the thread observes the flag instead of waiting out its
        // two-millisecond sleep on a socket nobody will connect to again.
        let _ = TcpStream::connect(self.address);
    }
}

/// Read one request, record it, and answer with the canned response for its method.
fn answer(mut stream: TcpStream, answers: &Answers, recorded: &Arc<Mutex<Vec<Recorded>>>) {
    let _ = stream.set_read_timeout(Some(Duration::from_secs(5)));
    let mut reader = BufReader::new(match stream.try_clone() {
        Ok(clone) => clone,
        Err(_) => return,
    });

    let mut request_line = String::new();
    if reader.read_line(&mut request_line).is_err() {
        return;
    }
    let mut parts = request_line.split_whitespace();
    let (Some(method), Some(path)) = (parts.next(), parts.next()) else {
        return;
    };
    let method = method.to_owned();
    let path = path.to_owned();

    let mut length = 0usize;
    loop {
        let mut line = String::new();
        if reader.read_line(&mut line).is_err() || line.trim().is_empty() {
            break;
        }
        if let Some((name, value)) = line.split_once(':')
            && name.eq_ignore_ascii_case("content-length")
        {
            length = value.trim().parse().unwrap_or(0);
        }
    }

    let mut body = vec![0_u8; length];
    if length > 0 && reader.read_exact(&mut body).is_err() {
        return;
    }
    recorded.lock().expect("the record").push(Recorded {
        method: method.clone(),
        path,
        body,
    });

    let (status, extra) = match method.as_str() {
        "HEAD" => answers.head.clone(),
        // A registry that assigns identity answers a create with 201; anything else means the
        // test observed an upload it did not expect, so it is answered as a failure.
        "POST" | "PUT" => (201, None),
        _ => (404, None),
    };
    let reason = match status {
        200 => "OK",
        201 => "Created",
        404 => "Not Found",
        _ => "Error",
    };
    let mut response =
        format!("HTTP/1.1 {status} {reason}\r\nContent-Length: 0\r\nConnection: close\r\n");
    if let Some(digest) = extra {
        response.push_str(&format!("x-checksum-sha256: {digest}\r\n"));
    }
    response.push_str("\r\n");
    let _ = stream.write_all(response.as_bytes());
    let _ = stream.flush();
}

/// A project whose only writable store is a hosted channel at `endpoint`.
///
/// The store is declared `auth = "token"` with no `secrets`, which is the one combination a
/// mock needs: the real OIDC exchange is impossible off a GitHub runner, and a token nobody
/// supplies is a token nobody has to fake. The upload path — the part under test — is the same
/// either way.
fn manifest(endpoint: &str) -> String {
    format!(
        r#"
schema = 1

[project]
name = "fixture-publish"
platforms = ["linux-64"]

[[packages]]
name = "example-airlock"
kind = "conda"
command = ["cargo", "build"]
outputs = ["output/**/*.conda"]
smoke = ["example-airlock --version"]
publish = ["hosted"]

[[stores]]
name = "hosted"
kind = "prefix-dev"
url = "{endpoint}"
channel = "acme/channel"
writable = true
auth = "token"
"#
    )
}

struct Fixture {
    directory: TempDir,
}

impl Fixture {
    fn new(endpoint: &str) -> Self {
        let directory = tempfile::tempdir().expect("temp dir");
        std::fs::write(directory.path().join("universe.toml"), manifest(endpoint))
            .expect("manifest");
        let fixture = Self { directory };
        fixture.add_package(b"conda bytes");
        fixture
    }

    fn root(&self) -> &Path {
        self.directory.path()
    }

    /// Write an artifact under `output/` and the fragment that vouches for it.
    fn add_package(&self, bytes: &[u8]) {
        let relative = "output/example-airlock-1.0.0-0.conda";
        let artifact = self.root().join(relative);
        std::fs::create_dir_all(artifact.parent().expect("parent")).expect("output dir");
        std::fs::write(&artifact, bytes).expect("artifact");

        let digest = universe_workflow::sha256::sha256_hex(bytes);
        let fragments = self.root().join("output/fragments");
        std::fs::create_dir_all(&fragments).expect("fragment dir");
        std::fs::write(
            fragments.join("example-airlock-linux-64.json"),
            format!(
                r#"{{
  "package": "example-airlock",
  "version": "1.0.0",
  "build": "0",
  "build_number": 0,
  "subdir": "linux-64",
  "digest": "{digest}",
  "size_bytes": {},
  "depends": [],
  "license": "MIT",
  "license_files": [],
  "path": "{relative}"
}}
"#,
                bytes.len()
            ),
        )
        .expect("fragment");
    }

    fn publish(&self, store: &str) -> universe_core::CommandOutput {
        let invocation: Vec<String> = [
            "publish-artifacts",
            "--store",
            store,
            "--json",
            "--root",
            self.root().to_str().expect("utf-8 root"),
        ]
        .into_iter()
        .map(str::to_owned)
        .collect();
        run(invocation)
    }
}

fn result_of(output: &universe_core::CommandOutput) -> Value {
    let envelope: Value = serde_json::from_str(output.stdout().trim()).expect("a JSON envelope");
    envelope["result"].clone()
}

/// Where a `prefix-dev` store puts both the existence check and the upload.
///
/// The registry takes the file name and files it under the channel it was told, so the
/// address is the channel rather than a path inside it (`Target::location`). The consequence
/// is worth knowing: for this kind, `apply`'s per-artifact `head` degenerates to one check per
/// channel, which is why the mock has to answer with the digest of the artifact under test
/// rather than with a directory listing.
const UPLOAD: &str = "/api/v1/upload/acme/channel";

#[test]
fn a_hosted_publish_posts_the_artifact_to_the_endpoint_the_manifest_declares() {
    // The premise of the whole exercise: the endpoint comes from the store, not from a
    // constant, which is what makes a mock possible without a second code path.
    let destination = Destination::start(Answers { head: (404, None) });
    let fixture = Fixture::new(&destination.endpoint());

    let output = fixture.publish("hosted");
    assert_eq!(output.exit_code(), 0, "stdout: {}", output.stdout());
    let result = result_of(&output);
    assert_eq!(result["dry_run"], false);
    assert_eq!(result["stores"][0]["store"], "hosted");
    assert_eq!(result["stores"][0]["bytes"], 11);
    assert_eq!(
        result["stores"][0]["uploaded"],
        serde_json::json!(["linux-64/example-airlock-1.0.0-0.conda"])
    );

    let requests = destination.requests();
    let paths: Vec<(&str, &str)> = requests
        .iter()
        .map(|request| (request.method.as_str(), request.path.as_str()))
        .collect();
    assert!(
        requests.iter().any(|request| request.method == "POST"
            && request.path == UPLOAD
            && request.body == b"conda bytes"),
        "the artifact's bytes were never posted; the mock saw {paths:?}"
    );
    assert_eq!(
        requests
            .iter()
            .filter(|request| request.method == "HEAD")
            .count(),
        1,
        "the destination is asked once whether it already holds the artifact"
    );
}

#[test]
fn a_destination_that_already_holds_the_artifact_receives_no_upload() {
    // Idempotence, proven on the traffic rather than on the report: a second run of an
    // unchanged build must not re-send bytes.
    let digest = universe_workflow::sha256::sha256_hex(b"conda bytes");
    let destination = Destination::start(Answers {
        head: (200, Some(digest.clone())),
    });
    let fixture = Fixture::new(&destination.endpoint());

    let output = fixture.publish("hosted");
    assert_eq!(output.exit_code(), 0, "stdout: {}", output.stdout());

    let requests = destination.requests();
    assert!(
        requests.iter().all(|request| request.method != "POST"),
        "a HEAD that matched the digest still triggered an upload: {requests:?}"
    );
    let heads: Vec<&str> = requests
        .iter()
        .filter(|request| request.method == "HEAD")
        .map(|request| request.path.as_str())
        .collect();
    assert_eq!(
        heads,
        vec![UPLOAD],
        "the existence check is addressed where the upload would go"
    );
}

#[test]
fn a_destination_holding_a_different_digest_is_refused_rather_than_overwritten() {
    // The third property a tag run depends on: publishing 0.1.0 over an artifact somebody
    // else put at the same name fails loudly unless `--force` says otherwise.
    let destination = Destination::start(Answers {
        head: (200, Some("0".repeat(64))),
    });
    let fixture = Fixture::new(&destination.endpoint());

    let output = fixture.publish("hosted");
    assert_ne!(output.exit_code(), 0, "stdout: {}", output.stdout());
    assert!(
        destination
            .requests()
            .iter()
            .all(|request| request.method != "POST"),
        "a refused publish must not have uploaded first"
    );
}
