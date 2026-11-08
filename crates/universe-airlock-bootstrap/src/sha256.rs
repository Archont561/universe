//! Streaming SHA-256, for verifying a payload a restore is about to activate.
//!
//! `universe-core` deliberately keeps digests as *text* and does not hash bytes, and the
//! renderer's hasher (`universe-workflow::sha256`) hashes one small string in memory. A
//! restore hashes artifacts that may be hundreds of megabytes, so this one takes chunks and
//! never holds a file: the requirement `UNI-32` states as "stream large objects with bounded
//! resource use".
//!
//! This is a second implementation of the same algorithm in the same repository. It is
//! duplicated rather than shared because the bootstrap is the one binary that must not carry
//! the workflow renderer, and because `universe-core` refuses to hash by design. Adding
//! `sha2` would be the better answer if the bootstrap were allowed a dependency on the
//! registry; it is not allowed one here without a relock of the whole transport, which is a
//! decision for `UNI-55` (where the signature verifier needs real cryptography anyway).
//!
//! Written against FIPS 180-4 and checked against the published vectors rather than against
//! the other copy, because two implementations that only agree with each other are a
//! checksum for the shared bug.

/// Round constants: the first 32 bits of the fractional parts of the cube roots of the
/// first 64 primes.
const K: [u32; 64] = [
    0x428a_2f98,
    0x7137_4491,
    0xb5c0_fbcf,
    0xe9b5_dba5,
    0x3956_c25b,
    0x59f1_11f1,
    0x923f_82a4,
    0xab1c_5ed5,
    0xd807_aa98,
    0x1283_5b01,
    0x2431_85be,
    0x550c_7dc3,
    0x72be_5d74,
    0x80de_b1fe,
    0x9bdc_06a7,
    0xc19b_f174,
    0xe49b_69c1,
    0xefbe_4786,
    0x0fc1_9dc6,
    0x240c_a1cc,
    0x2de9_2c6f,
    0x4a74_84aa,
    0x5cb0_a9dc,
    0x76f9_88da,
    0x983e_5152,
    0xa831_c66d,
    0xb003_27c8,
    0xbf59_7fc7,
    0xc6e0_0bf3,
    0xd5a7_9147,
    0x06ca_6351,
    0x1429_2967,
    0x27b7_0a85,
    0x2e1b_2138,
    0x4d2c_6dfc,
    0x5338_0d13,
    0x650a_7354,
    0x766a_0abb,
    0x81c2_c92e,
    0x9272_2c85,
    0xa2bf_e8a1,
    0xa81a_664b,
    0xc24b_8b70,
    0xc76c_51a3,
    0xd192_e819,
    0xd699_0624,
    0xf40e_3585,
    0x106a_a070,
    0x19a4_c116,
    0x1e37_6c08,
    0x2748_774c,
    0x34b0_bcb5,
    0x391c_0cb3,
    0x4ed8_aa4a,
    0x5b9c_ca4f,
    0x682e_6ff3,
    0x748f_82ee,
    0x78a5_636f,
    0x84c8_7814,
    0x8cc7_0208,
    0x90be_fffa,
    0xa450_6ceb,
    0xbef9_a3f7,
    0xc671_78f2,
];

/// The initial hash value: the first 32 bits of the fractional parts of the square roots of
/// the first eight primes.
const H0: [u32; 8] = [
    0x6a09_e667,
    0xbb67_ae85,
    0x3c6e_f372,
    0xa54f_f53a,
    0x510e_527f,
    0x9b05_688c,
    0x1f83_d9ab,
    0x5be0_cd19,
];

/// Size of the block a chunk is folded into.
const BLOCK: usize = 64;

/// A SHA-256 hasher that accepts input in pieces.
#[derive(Clone)]
pub struct Sha256 {
    state: [u32; 8],
    block: [u8; BLOCK],
    filled: usize,
    /// Total bytes absorbed, which is what the length padding needs.
    length: u64,
}

impl Sha256 {
    /// Start a new hash.
    #[must_use]
    pub fn new() -> Self {
        Self {
            state: H0,
            block: [0; BLOCK],
            filled: 0,
            length: 0,
        }
    }

    /// Absorb another piece of the input.
    pub fn update(&mut self, mut bytes: &[u8]) {
        self.length = self.length.wrapping_add(bytes.len() as u64);

        // Top the open block up first; only a full block is compressed.
        if self.filled > 0 {
            let wanted = BLOCK - self.filled;
            let taken = wanted.min(bytes.len());
            self.block[self.filled..self.filled + taken].copy_from_slice(&bytes[..taken]);
            self.filled += taken;
            bytes = &bytes[taken..];
            if self.filled == BLOCK {
                let block = self.block;
                self.compress(&block);
                self.filled = 0;
            }
        }

        // Then whole blocks straight out of the input, with no copy.
        while bytes.len() >= BLOCK {
            let mut block = [0u8; BLOCK];
            block.copy_from_slice(&bytes[..BLOCK]);
            self.compress(&block);
            bytes = &bytes[BLOCK..];
        }

        if !bytes.is_empty() {
            self.block[..bytes.len()].copy_from_slice(bytes);
            self.filled = bytes.len();
        }
    }

    /// Finish, returning the digest as lowercase hexadecimal text.
    #[must_use]
    pub fn finish(mut self) -> String {
        let bit_length = self.length.wrapping_mul(8);

        // Padding: a single 1 bit, then zeros, then the 64-bit big-endian bit length.
        self.block[self.filled] = 0x80;
        self.filled += 1;
        if self.filled > BLOCK - 8 {
            self.block[self.filled..].fill(0);
            let block = self.block;
            self.compress(&block);
            self.filled = 0;
        }
        self.block[self.filled..BLOCK - 8].fill(0);
        self.block[BLOCK - 8..].copy_from_slice(&bit_length.to_be_bytes());
        let block = self.block;
        self.compress(&block);

        let mut text = String::with_capacity(64);
        for word in self.state {
            text.push_str(&format!("{word:08x}"));
        }
        text
    }

    fn compress(&mut self, block: &[u8; BLOCK]) {
        let mut w = [0u32; 64];
        let (words, _) = block.as_chunks::<4>();
        for (index, word) in words.iter().enumerate() {
            w[index] = u32::from_be_bytes(*word);
        }
        for index in 16..64 {
            let s0 = w[index - 15].rotate_right(7)
                ^ w[index - 15].rotate_right(18)
                ^ (w[index - 15] >> 3);
            let s1 = w[index - 2].rotate_right(17)
                ^ w[index - 2].rotate_right(19)
                ^ (w[index - 2] >> 10);
            w[index] = w[index - 16]
                .wrapping_add(s0)
                .wrapping_add(w[index - 7])
                .wrapping_add(s1);
        }

        let [mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut h] = self.state;
        for index in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let choice = (e & f) ^ (!e & g);
            let temp1 = h
                .wrapping_add(s1)
                .wrapping_add(choice)
                .wrapping_add(K[index])
                .wrapping_add(w[index]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let majority = (a & b) ^ (a & c) ^ (b & c);
            let temp2 = s0.wrapping_add(majority);

            h = g;
            g = f;
            f = e;
            e = d.wrapping_add(temp1);
            d = c;
            c = b;
            b = a;
            a = temp1.wrapping_add(temp2);
        }

        for (slot, value) in self.state.iter_mut().zip([a, b, c, d, e, f, g, h]) {
            *slot = slot.wrapping_add(value);
        }
    }
}

impl Default for Sha256 {
    fn default() -> Self {
        Self::new()
    }
}

/// How much of a file is read at a time: large enough that a multi-hundred-megabyte artifact
/// is a few thousand reads, small enough that memory does not scale with the artifact.
pub const CHUNK: usize = 64 * 1024;

/// Hash a byte slice, for callers that already hold it.
#[must_use]
pub fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hasher.finish()
}

/// Hash everything a reader yields, returning the digest and the number of bytes seen.
///
/// The count is not a convenience: an artifact is verified against a digest *and* a size, and
/// reading once for both is what makes the two facts describe the same bytes.
///
/// # Errors
///
/// Propagates the reader's own error rather than hashing a truncated stream.
pub fn hash_reader<R: std::io::Read>(mut reader: R) -> std::io::Result<(String, u64)> {
    let mut hasher = Sha256::new();
    let mut buffer = vec![0u8; CHUNK];
    let mut seen: u64 = 0;
    loop {
        let read = reader.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
        seen += read as u64;
    }
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
    use super::{Sha256, sha256_hex};

    // The published FIPS 180-4 vectors. A hasher checked only against its own output would
    // agree with itself while being wrong, which is the failure that matters here.
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
    fn a_million_a_characters_match_the_published_digest() {
        let block = [b'a'; 1000];
        let mut hasher = Sha256::new();
        for _ in 0..1000 {
            hasher.update(&block);
        }
        assert_eq!(
            hasher.finish(),
            "cdc76e5c9914fb9281a1c7e284d73e67f1809a48a497200e046d39ccc7112cd0"
        );
    }

    #[test]
    fn the_result_does_not_depend_on_how_the_input_is_split() {
        // A restore feeds a file in 64 KiB chunks, so the chunk boundaries must not matter —
        // including boundaries that straddle the 64-byte block, which is where a hasher with a
        // bug in its buffering shows up.
        let data: Vec<u8> = (0..1000u32).map(|value| (value % 251) as u8).collect();
        let whole = sha256_hex(&data);
        for chunk in [1, 7, 63, 64, 65, 127, 999] {
            let mut hasher = Sha256::new();
            for piece in data.chunks(chunk) {
                hasher.update(piece);
            }
            assert_eq!(hasher.finish(), whole, "chunk size {chunk}");
        }
    }
}
