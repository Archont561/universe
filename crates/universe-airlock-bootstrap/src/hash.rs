//! SHA-256 over files and streams, for verifying a payload before it is activated.
//!
//! `universe-core` deliberately keeps digests as *text* and does not hash bytes, so hashing
//! belongs to the component that owns the artifact. This module is the thin part: it reads in
//! bounded chunks so a multi-hundred-megabyte artifact costs a few thousand reads and no
//! memory (`UNI-32`'s "stream large objects with bounded resource use"), and it returns the
//! byte count alongside the digest, because a payload is verified against both and reading
//! once for both is what makes the two facts describe the same bytes.
//!
//! The algorithm itself is `sha2`, not a local implementation. A security-critical binary
//! should not carry bespoke cryptography, and the earlier in-repo hasher existed only because
//! the transport could not be relocked mid-slice.

use std::io::Read;

use sha2::{Digest, Sha256};

/// How much of a file is read at a time: large enough that an artifact is a few thousand
/// reads, small enough that memory does not scale with the artifact.
pub const CHUNK: usize = 64 * 1024;

/// Lowercase hexadecimal, the one form every digest in this project is recorded in.
#[must_use]
pub fn hex(bytes: &[u8]) -> String {
    let mut text = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        text.push_str(&format!("{byte:02x}"));
    }
    text
}

/// A hasher that can be fed while bytes are also being written somewhere else.
///
/// The restore hashes what it *writes*, so the copy is verified rather than the read: a source
/// that changes between verification and copying is caught at the moment the bytes land.
#[derive(Clone)]
pub struct Hasher {
    inner: Sha256,
    seen: u64,
}

impl Hasher {
    /// Start a new hash.
    #[must_use]
    pub fn new() -> Self {
        Self {
            inner: Sha256::new(),
            seen: 0,
        }
    }

    /// Absorb another piece of the input.
    pub fn update(&mut self, bytes: &[u8]) {
        self.inner.update(bytes);
        self.seen += bytes.len() as u64;
    }

    /// Bytes absorbed so far.
    #[must_use]
    pub const fn len(&self) -> u64 {
        self.seen
    }

    /// Whether nothing has been absorbed yet.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.seen == 0
    }

    /// Finish, returning the digest as lowercase hexadecimal text.
    #[must_use]
    pub fn finish(self) -> String {
        hex(&self.inner.finalize())
    }
}

impl Default for Hasher {
    fn default() -> Self {
        Self::new()
    }
}

/// Hash a byte slice, for callers that already hold it.
#[must_use]
pub fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Hasher::new();
    hasher.update(bytes);
    hasher.finish()
}

/// Hash everything a reader yields, returning the digest and the number of bytes seen.
///
/// # Errors
///
/// Propagates the reader's own error rather than hashing a truncated stream, because a digest
/// over half a file is a digest nobody should act on.
pub fn hash_reader<R: Read>(mut reader: R) -> std::io::Result<(String, u64)> {
    let mut hasher = Hasher::new();
    let mut buffer = vec![0u8; CHUNK];
    loop {
        let read = reader.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    let seen = hasher.len();
    Ok((hasher.finish(), seen))
}

/// Hash a file without holding it in memory.
///
/// # Errors
///
/// Returns the open or read error, so a caller can distinguish "absent" from "unreadable".
pub fn hash_file(path: &std::path::Path) -> std::io::Result<(String, u64)> {
    hash_reader(std::fs::File::open(path)?)
}

#[cfg(test)]
mod tests {
    use super::{Hasher, hash_reader, sha256_hex};

    // The published FIPS 180-4 vectors. They no longer test our arithmetic — `sha2` owns
    // that — but they do test that the digest is encoded, fed and truncated the way the rest
    // of the project assumes, which is where a wrapper goes wrong.
    #[test]
    fn the_published_test_vectors_match() {
        let vectors = [
            (
                "",
                "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
            ),
            (
                "abc",
                "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad",
            ),
            (
                "abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq",
                "248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1",
            ),
        ];
        for (input, expected) in vectors {
            assert_eq!(sha256_hex(input.as_bytes()), expected, "input {input:?}");
        }
    }

    #[test]
    fn the_result_does_not_depend_on_how_the_input_is_split() {
        let data: Vec<u8> = (0..1000u32).map(|value| (value % 251) as u8).collect();
        let whole = sha256_hex(&data);
        for chunk in [1, 7, 63, 64, 65, 127, 999] {
            let mut hasher = Hasher::new();
            for piece in data.chunks(chunk) {
                hasher.update(piece);
            }
            assert_eq!(hasher.finish(), whole, "chunk size {chunk}");
        }

        // The byte count is what a caller compares against a manifest, so it counts what was
        // fed and nothing else.
        let mut counted = Hasher::new();
        assert!(counted.is_empty());
        counted.update(&data[..1]);
        assert!(!counted.is_empty());
        assert_eq!(counted.len(), 1);
    }

    #[test]
    fn a_reader_reports_the_digest_and_the_length_it_actually_read() {
        let data = b"the only artifact\n";
        let (digest, seen) = hash_reader(&data[..]).expect("hashes");
        assert_eq!(digest, sha256_hex(data));
        assert_eq!(seen, data.len() as u64);
    }
}
