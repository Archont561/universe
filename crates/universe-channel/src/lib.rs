//! Conda channels for Universe: fragments in, a deterministic static channel out.
//!
//! The crate owns two things and deliberately nothing else. **Rendering** turns audit
//! fragments into the metadata a solver reads — CEP-15 `repodata.json` per subdir,
//! `channeldata.json`, `run_exports.json`, the marker files — and every file it produces is a
//! pure function of the fragment set, so two renders of the same artifacts are byte-identical.
//! **Publication** moves that tree to a destination exactly once, behind a transport seam, so
//! a refused push and an idempotent republish are testable without a network.
//!
//! What is *not* here: no HTTP client, no git library, no solver. The channel is a tree of
//! files; how it travels is the transport's business, and which store it travels to is
//! `universe-kit`'s.

pub mod channel;
pub mod fragment;
pub mod readme;
pub mod transport;

pub use channel::{
    BLOB_CEILING, ChannelError, ChannelFile, ChannelSpec, STAGED_CEILING, TRANSPORT_CEILING, render,
};
pub use fragment::Fragment;
pub use transport::{
    BudgetError, Budgets, ChannelTree, MemoryTransport, PushOutcome, Transport, TransportError,
    TreeIdentity,
};
