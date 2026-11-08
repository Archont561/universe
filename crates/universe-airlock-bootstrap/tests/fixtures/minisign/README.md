# Minisign verification vector

Four files, and one honest caveat: these were generated for the tests, not published.

| File | What it is |
| --- | --- |
| `release-key.pub` | The minisign public key that signed `universe-release.json.minisig`. |
| `rotation-key.pub` | A second, unrelated minisign public key. Its key id matches no signature here, which is what the rotation tests need. |
| `universe-release.json.minisig` | A prehashed (`ED`) detached signature over the four bytes `test`. |
| `legacy-universe-release.json.minisig` | The same key, signing the same bytes in legacy (`Ed`) mode — the pre-prehash format, which the verifier refuses on purpose. |

Both keys are the public halves of Ed25519 key pairs derived from the fixed seeds
`0101…01` and `0202…02`, and the signatures were produced with an independent RFC 8032
implementation against the same BLAKE2b-512 prehash the verifier computes. They are
**throwaway keys for tests only**: no release has ever been signed with them, and the
private halves are not kept.

A real release key replaces them in `UNI-55`, where publishing the key is the whole task.
These fixtures stay as they are, so the verifier keeps being exercised without a key being
compiled into the binary.
