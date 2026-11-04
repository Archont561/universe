//! Where a channel goes: a whole snapshot, moved in one operation.
//!
//! The transport is a seam on purpose. Publication is the one step that cannot be undone by
//! editing a file, so its failure modes — a refused push, a remote that moved underneath us,
//! a republish that must not manufacture a commit — are tested against an in-memory double
//! rather than against whatever network the test host happens to have.

use std::collections::BTreeMap;

use crate::channel::{BLOB_CEILING, STAGED_CEILING, TRANSPORT_CEILING};

/// A channel as a set of paths and bytes: exactly what one commit holds.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ChannelTree {
    files: BTreeMap<String, Vec<u8>>,
}

impl ChannelTree {
    /// Add one file, replacing any file already at that path.
    pub fn insert(&mut self, path: impl Into<String>, bytes: Vec<u8>) {
        self.files.insert(path.into(), bytes);
    }

    /// Add a file from a slice.
    pub fn insert_slice(&mut self, path: impl Into<String>, bytes: &[u8]) {
        self.insert(path, bytes.to_vec());
    }

    /// Every path in the tree, sorted.
    pub fn paths(&self) -> impl Iterator<Item = &String> {
        self.files.keys()
    }

    /// Every `(path, bytes)` pair, sorted by path.
    pub fn files(&self) -> impl Iterator<Item = (&String, &Vec<u8>)> {
        self.files.iter()
    }

    /// Whether the tree is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.files.is_empty()
    }

    /// How many files the tree holds.
    #[must_use]
    pub fn len(&self) -> usize {
        self.files.len()
    }

    /// Total bytes, which is what a transport budget measures.
    #[must_use]
    pub fn total_bytes(&self) -> u64 {
        self.files.values().map(|bytes| bytes.len() as u64).sum()
    }

    /// The largest single file, name and size.
    #[must_use]
    pub fn largest(&self) -> Option<(&str, u64)> {
        self.files
            .iter()
            .map(|(path, bytes)| (path.as_str(), bytes.len() as u64))
            .max_by(|left, right| left.1.cmp(&right.1))
    }

    /// A content digest of the whole tree, independent of insertion order.
    ///
    /// Two trees with the same files produce the same digest, which is what lets an in-memory
    /// transport answer "already published" without pretending to be git.
    #[must_use]
    pub fn digest(&self) -> String {
        let mut material = Vec::new();
        for (path, bytes) in &self.files {
            material.extend_from_slice(path.as_bytes());
            material.push(0);
            material.extend_from_slice(&(bytes.len() as u64).to_le_bytes());
            material.extend_from_slice(bytes);
        }
        universe_workflow_digest(&material)
    }

    /// Split any single file above `ceiling` into `.partNNN` pieces.
    ///
    /// Whole files are the unit of transport because Git content-addresses them: an unchanged
    /// `.conda` costs nothing on the next publish, while fixed-size chunking would rewrite
    /// every piece whenever one byte moved. Only a file that a transport would refuse is cut.
    ///
    /// Package files only. `repodata.json` is what a solver reads first and cannot be
    /// reconstructed from pieces by anything that speaks the protocol, so cutting it would
    /// produce a tree that is small and useless.
    #[must_use]
    pub fn split_oversized(&self, ceiling: u64) -> ChannelTree {
        let mut split = ChannelTree::default();
        for (path, bytes) in &self.files {
            if bytes.len() as u64 <= ceiling || !path.ends_with(".conda") {
                split.insert(path.clone(), bytes.clone());
                continue;
            }
            let width = bytes.len().div_ceil(ceiling as usize);
            let digits = width.to_string().len().max(3);
            for (index, piece) in bytes.chunks(ceiling as usize).enumerate() {
                split.insert(format!("{path}.part{:0digits$}", index + 1), piece.to_vec());
            }
        }
        split
    }
}

/// The limits one publish is measured against, and the field that would change each.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Budgets {
    /// Largest single blob the transport accepts.
    pub blob: u64,
    /// Total bytes one publish may move.
    pub transport: u64,
    /// Bytes one staged push may hold.
    pub staged: u64,
}

impl Default for Budgets {
    fn default() -> Self {
        Self {
            blob: BLOB_CEILING,
            transport: TRANSPORT_CEILING,
            staged: STAGED_CEILING,
        }
    }
}

/// Why a publish was refused before anything moved.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum BudgetError {
    /// One file is larger than the transport accepts whole.
    #[error(
        "'{path}' is {measured} bytes, above the {limit}-byte blob ceiling; the nearest fix is to split the artifact or raise [stores.budgets].blob"
    )]
    Blob {
        /// The file.
        path: String,
        /// Its size.
        measured: u64,
        /// The ceiling.
        limit: u64,
    },

    /// The tree as a whole is larger than one publish may move.
    #[error(
        "the snapshot is {measured} bytes, above the {limit}-byte transport ceiling; the nearest fix is to publish fewer artifacts per snapshot or raise [stores.budgets].transport"
    )]
    Transport {
        /// Total size.
        measured: u64,
        /// The ceiling.
        limit: u64,
    },

    /// The staged push is larger than the transport accepts at once.
    #[error(
        "the staged push is {measured} bytes, above the {limit}-byte staging ceiling; the nearest fix is to publish fewer artifacts per snapshot or raise [stores.budgets].staged"
    )]
    Staged {
        /// Total size.
        measured: u64,
        /// The ceiling.
        limit: u64,
    },
}

impl Budgets {
    /// Refuse a snapshot that no transport would accept.
    ///
    /// # Errors
    ///
    /// Returns the first ceiling the snapshot exceeds, naming the measured value and the
    /// limit, because "too large" without a number is advice a maintainer cannot act on.
    pub fn check(&self, tree: &ChannelTree) -> Result<(), BudgetError> {
        if let Some((path, size)) = tree.largest()
            && size > self.blob
        {
            return Err(BudgetError::Blob {
                path: path.to_owned(),
                measured: size,
                limit: self.blob,
            });
        }
        let total = tree.total_bytes();
        if total > self.transport {
            return Err(BudgetError::Transport {
                measured: total,
                limit: self.transport,
            });
        }
        if total > self.staged {
            return Err(BudgetError::Staged {
                measured: total,
                limit: self.staged,
            });
        }
        Ok(())
    }
}

/// What the remote branch holds, as far as publication is concerned.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TreeIdentity {
    /// The commit the branch points at.
    pub commit: String,
    /// The tree that commit holds.
    pub tree: String,
}

/// What a push did.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PushOutcome {
    /// The remote already held exactly this tree, so nothing was pushed.
    Unchanged,
    /// A new snapshot commit was force-pushed.
    Published {
        /// The commit that was pushed.
        commit: String,
    },
}

/// Why a transport failed.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum TransportError {
    /// The underlying transport refused or failed.
    #[error("{operation} failed: {message}")]
    Failed {
        /// The operation that failed.
        operation: String,
        /// What the transport said.
        message: String,
    },
}

/// A destination that holds exactly one channel snapshot.
pub trait Transport {
    /// The tree the destination currently holds, if it holds one.
    ///
    /// # Errors
    ///
    /// Returns [`TransportError`] when the destination cannot be read.
    fn current(&self) -> Result<Option<TreeIdentity>, TransportError>;

    /// Force-push one snapshot, or report that it is already there.
    ///
    /// # Errors
    ///
    /// Returns [`TransportError`] when the push fails.
    fn push(&self, tree: &ChannelTree, message: &str) -> Result<PushOutcome, TransportError>;
}

/// An in-memory transport: enough of a destination to test publication against.
///
/// It is not a mock of git. It stores trees by content digest and answers the only two
/// questions publication asks — what is there now, and did this change it — which is what
/// makes a failed push and an idempotent republish testable without a network.
#[derive(Debug, Default)]
pub struct MemoryTransport {
    held: std::cell::RefCell<Option<(String, ChannelTree)>>,
    /// Refuse every push, to test the failure path.
    pub refuse: bool,
    /// Every push this double received, in order.
    pub attempts: std::cell::RefCell<Vec<String>>,
}

impl MemoryTransport {
    /// A transport holding nothing.
    #[must_use]
    pub fn empty() -> Self {
        Self::default()
    }

    /// A transport already holding `tree`.
    #[must_use]
    pub fn holding(tree: ChannelTree) -> Self {
        let digest = tree.digest();
        Self {
            held: std::cell::RefCell::new(Some((digest.clone(), tree))),
            refuse: false,
            attempts: std::cell::RefCell::new(Vec::new()),
        }
    }

    /// The tree this transport holds, if any.
    #[must_use]
    pub fn held(&self) -> Option<ChannelTree> {
        self.held.borrow().as_ref().map(|(_, tree)| tree.clone())
    }
}

impl Transport for MemoryTransport {
    fn current(&self) -> Result<Option<TreeIdentity>, TransportError> {
        Ok(self.held.borrow().as_ref().map(|(digest, _)| TreeIdentity {
            commit: format!("memory:{}", &digest[..12]),
            tree: digest.clone(),
        }))
    }

    fn push(&self, tree: &ChannelTree, message: &str) -> Result<PushOutcome, TransportError> {
        self.attempts.borrow_mut().push(message.to_owned());
        if self.refuse {
            return Err(TransportError::Failed {
                operation: "push".to_owned(),
                message: "the remote refused the update".to_owned(),
            });
        }
        let digest = tree.digest();
        if let Some((held, _)) = self.held.borrow().as_ref()
            && held == &digest
        {
            return Ok(PushOutcome::Unchanged);
        }
        *self.held.borrow_mut() = Some((digest.clone(), tree.clone()));
        Ok(PushOutcome::Published {
            commit: format!("memory:{}", &digest[..12]),
        })
    }
}

/// SHA-256 in lowercase hexadecimal.
fn universe_workflow_digest(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hasher
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{BudgetError, Budgets, ChannelTree, MemoryTransport, PushOutcome, Transport};

    fn tree(bytes: &[u8]) -> ChannelTree {
        let mut tree = ChannelTree::default();
        tree.insert_slice("linux-64/pkg-0.1.0-0.conda", bytes);
        tree.insert_slice("linux-64/repodata.json", b"{}");
        tree
    }

    #[test]
    fn the_same_content_pushed_twice_is_pushed_once() {
        let transport = MemoryTransport::empty();
        assert!(matches!(
            transport.push(&tree(b"payload"), "first").expect("pushes"),
            PushOutcome::Published { .. }
        ));
        assert_eq!(
            transport.push(&tree(b"payload"), "again").expect("pushes"),
            PushOutcome::Unchanged
        );
        assert_eq!(transport.attempts.borrow().len(), 2);
    }

    #[test]
    fn insertion_order_does_not_change_the_digest() {
        let mut left = ChannelTree::default();
        left.insert_slice("a", b"1");
        left.insert_slice("b", b"2");
        let mut right = ChannelTree::default();
        right.insert_slice("b", b"2");
        right.insert_slice("a", b"1");
        assert_eq!(left.digest(), right.digest());
    }

    #[test]
    fn a_refused_push_reports_the_transport_rather_than_pretending_to_publish() {
        let transport = MemoryTransport {
            refuse: true,
            ..MemoryTransport::empty()
        };
        assert!(transport.push(&tree(b"payload"), "x").is_err());
        assert!(transport.held().is_none());
    }

    #[test]
    fn a_small_tree_fits_every_budget() {
        assert_eq!(Budgets::default().check(&tree(b"payload")), Ok(()));
    }

    #[test]
    fn a_file_above_the_blob_ceiling_is_named_with_its_size() {
        let budgets = Budgets {
            blob: 4,
            ..Budgets::default()
        };
        let error = budgets.check(&tree(b"0123456789")).expect_err("refused");
        assert!(matches!(
            error,
            BudgetError::Blob {
                measured: 10,
                limit: 4,
                ..
            }
        ));
        assert!(error.to_string().contains("budgets"));
    }

    #[test]
    fn an_oversized_file_is_split_into_parts_and_nothing_else_moves() {
        let tree = tree(b"0123456789");
        let split = tree.split_oversized(4);
        let paths: Vec<&String> = split.paths().collect();
        assert_eq!(
            paths,
            [
                "linux-64/pkg-0.1.0-0.conda.part001",
                "linux-64/pkg-0.1.0-0.conda.part002",
                "linux-64/pkg-0.1.0-0.conda.part003",
                "linux-64/repodata.json"
            ]
        );
        assert_eq!(split.total_bytes(), tree.total_bytes());
    }
}

#[cfg(test)]
mod budget_tests {
    use super::{BudgetError, Budgets, ChannelTree, MemoryTransport, PushOutcome, Transport};

    fn tree_with(path: &str, bytes: &[u8]) -> ChannelTree {
        let mut tree = ChannelTree::default();
        tree.insert_slice(path, bytes);
        tree
    }

    #[test]
    fn a_blob_above_the_ceiling_names_the_file_the_measurement_and_the_field() {
        let budgets = Budgets {
            blob: 4,
            ..Budgets::default()
        };
        let error = budgets
            .check(&tree_with("linux-64/a.conda", b"12345"))
            .expect_err("refused");
        assert_eq!(
            error,
            BudgetError::Blob {
                path: "linux-64/a.conda".to_owned(),
                measured: 5,
                limit: 4,
            }
        );
        assert!(error.to_string().contains("blob ceiling"), "{error}");
        assert!(
            error.to_string().contains("[stores.budgets].blob"),
            "{error}"
        );
    }

    #[test]
    fn a_tree_above_the_transport_ceiling_is_refused_before_any_staging() {
        let budgets = Budgets {
            transport: 4,
            ..Budgets::default()
        };
        let error = budgets
            .check(&tree_with("channeldata.json", b"12345"))
            .expect_err("refused");
        assert!(matches!(error, BudgetError::Transport { .. }), "{error:?}");
        assert!(
            error.to_string().contains("[stores.budgets].transport"),
            "{error}"
        );
    }

    #[test]
    fn a_tree_above_the_staging_ceiling_is_refused_with_its_own_field() {
        let budgets = Budgets {
            staged: 4,
            ..Budgets::default()
        };
        let error = budgets
            .check(&tree_with("channeldata.json", b"12345"))
            .expect_err("refused");
        assert!(matches!(error, BudgetError::Staged { .. }), "{error:?}");
        assert!(
            error.to_string().contains("[stores.budgets].staged"),
            "{error}"
        );
    }

    #[test]
    fn only_package_files_are_split_because_metadata_cannot_be_reassembled() {
        // A split `repodata.json` would be a channel no client can read: it is the document a
        // solver starts from, not an artifact it downloads after reading something else.
        let mut tree = ChannelTree::default();
        tree.insert_slice("linux-64/a.conda", b"1234567890");
        tree.insert_slice("linux-64/repodata.json", b"1234567890");
        let split = tree.split_oversized(4);
        assert!(split.paths().any(|path| path == "linux-64/a.conda.part001"));
        assert!(split.paths().any(|path| path == "linux-64/repodata.json"));
        assert!(
            !split
                .paths()
                .any(|path| path == "linux-64/repodata.json.part001")
        );
    }

    #[test]
    fn a_refusing_transport_records_the_attempt_and_reports_the_failure() {
        let transport = MemoryTransport {
            refuse: true,
            ..MemoryTransport::empty()
        };
        let tree = tree_with("linux-64/a.conda", b"bytes");
        let error = transport
            .push(&tree, "channel fixture")
            .expect_err("refused");
        assert!(error.to_string().contains("refused the update"), "{error}");
        assert_eq!(transport.attempts.borrow().len(), 1);
        assert_eq!(transport.held(), None);
    }

    #[test]
    fn a_transport_that_already_holds_the_tree_publishes_nothing() {
        let tree = tree_with("linux-64/a.conda", b"bytes");
        let transport = MemoryTransport::holding(tree.clone());
        assert!(matches!(
            transport.push(&tree, "channel fixture").expect("push"),
            PushOutcome::Unchanged
        ));
        assert_eq!(
            transport
                .current()
                .expect("current")
                .map(|identity| identity.tree),
            Some(tree.digest())
        );
    }
}
