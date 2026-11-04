//! Conda channels for Universe: fragments in, a deterministic static channel out.
//!
//! The crate owns two things and deliberately nothing else. **Rendering** turns audit
//! fragments into the metadata a solver reads — CEP-15 `repodata.json` per subdir,
//! `channeldata.json`, `run_exports.json`, the marker files — and every file it produces is a
//! pure function of the fragment set, so two renders of the same artifacts are byte-identical.
//! **Publication** moves that tree to a destination exactly once, behind a transport seam, so
//! a refused push and an idempotent republish are testable without a network.
//!
//! **Store writes** ([`store`]) publish one artifact at a time to a destination that owns the
//! identity of a file — a Conda registry, an object store, a release — behind a [`Remote`]
//! seam that fixes the three rules a republish must obey: an unchanged digest is not re-sent,
//! a changed digest under an occupied name is refused, and a dry run sends no mutating
//! request.
//!
//! What is *not* here: no git library, no solver, and no store's configuration. The channel is
//! a tree of files; how it travels is the transport's business, and which store it travels to
//! is `universe-kit`'s.

pub mod channel;
pub mod fragment;
pub mod readme;
pub mod store;
pub mod transport;

pub use channel::{
    BLOB_CEILING, ChannelError, ChannelFile, ChannelSpec, STAGED_CEILING, TRANSPORT_CEILING, render,
};
pub use fragment::Fragment;
pub use transport::{
    BudgetError, Budgets, ChannelTree, MemoryTransport, PushOutcome, Transport, TransportError,
    TreeIdentity,
};
