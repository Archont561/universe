//! The branch store's transport: one orphan snapshot, force-pushed.
//!
//! A channel on a Git branch is a poor CDN and a very good transport for a disconnected host:
//! it needs no endpoint, no credentials beyond the ones that already push the repository, and
//! a consumer restores it by cloning. The costs are real and are stated where they bite —
//! a blob above the ceiling, a repository that grows with every build, a force-push that
//! bounds what is *served* without shrinking what is *stored*.
//!
//! Every `git` invocation goes through [`GitTransport::run`], and every argv it issues is
//! visible to a test through that one seam.

use std::path::{Path, PathBuf};
use std::process::Command;

use universe_channel::transport::{
    ChannelTree, PushOutcome, Transport, TransportError, TreeIdentity,
};

/// A channel held on a Git branch, reached through an existing remote.
#[derive(Debug)]
pub(crate) struct GitTransport {
    remote: String,
    branch: String,
    scratch: PathBuf,
    /// Fixed identity and date, so the commit id is a function of the content.
    identity: [(&'static str, &'static str); 4],
}

impl GitTransport {
    /// A transport for `remote` at `branch`, staging work in `scratch`.
    pub(crate) fn new(remote: &str, branch: &str, scratch: &Path) -> Self {
        Self {
            remote: remote.to_owned(),
            branch: branch.to_owned(),
            scratch: scratch.to_path_buf(),
            identity: [
                ("user.name", "Universe"),
                ("user.email", "universe@invalid"),
                ("GIT_AUTHOR_DATE", "2000-01-01T00:00:00 +0000"),
                ("GIT_COMMITTER_DATE", "2000-01-01T00:00:00 +0000"),
            ],
        }
    }

    /// Run one git command in the scratch repository.
    fn run(&self, arguments: &[&str]) -> Result<String, TransportError> {
        let mut command = Command::new("git");
        // Global options come before the subcommand: `git -c a=b init` is config, `git init
        // -c a=b` is an unknown switch.
        command.arg("-C").arg(&self.scratch);
        for (name, value) in self.identity {
            // The dates are environment and the identity is config; each mechanism ignores
            // the other's variables.
            if name.starts_with("GIT_") {
                command.env(name, value);
            } else {
                command.arg("-c").arg(format!("{name}={value}"));
            }
        }
        command.args(arguments);
        let output = command.output().map_err(|error| TransportError::Failed {
            operation: "git".to_owned(),
            message: format!("cannot run git: {error}"),
        })?;
        if !output.status.success() {
            return Err(TransportError::Failed {
                operation: format!("git {}", arguments.join(" ")),
                message: String::from_utf8_lossy(&output.stderr).trim().to_owned(),
            });
        }
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_owned())
    }

    fn prepare(&self) -> Result<(), TransportError> {
        std::fs::create_dir_all(&self.scratch).map_err(|error| TransportError::Failed {
            operation: "create scratch".to_owned(),
            message: error.to_string(),
        })?;
        if self.scratch.join(".git").exists() {
            return Ok(());
        }
        self.run(&["init", "--quiet"]).map(|_| ())
    }

    /// Stage `tree` and return the git tree hash it produces.
    fn write_tree(&self, tree: &ChannelTree) -> Result<String, TransportError> {
        self.prepare()?;
        // A stale index would carry files from a previous snapshot into this one.
        self.run(&["rm", "-r", "--cached", "--quiet", "."]).ok();
        for (path, bytes) in tree.files() {
            let target = self.scratch.join(path);
            if let Some(parent) = target.parent() {
                std::fs::create_dir_all(parent).map_err(|error| TransportError::Failed {
                    operation: format!("create {}", parent.display()),
                    message: error.to_string(),
                })?;
            }
            std::fs::write(&target, bytes).map_err(|error| TransportError::Failed {
                operation: format!("write {path}"),
                message: error.to_string(),
            })?;
        }
        self.run(&["add", "--all", "--force"])?;
        self.run(&["write-tree"])
    }

    /// The git tree the given snapshot produces, without pushing it.
    ///
    /// `--check` compares *this* with the remote, not the snapshot's content digest: the
    /// question is whether git would write a different tree, and only git can answer it.
    pub(crate) fn staged_tree(&self, tree: &ChannelTree) -> Result<String, TransportError> {
        self.write_tree(tree)
    }

    /// The tree hash the remote branch currently holds, if it holds one.
    fn remote_tree(&self) -> Result<Option<String>, TransportError> {
        let listing = self.run(&[
            "ls-remote",
            &self.remote,
            &format!("refs/heads/{}", self.branch),
        ])?;
        if listing.is_empty() {
            return Ok(None);
        }
        self.run(&[
            "fetch",
            "--quiet",
            "--depth",
            "1",
            &self.remote,
            &self.branch,
        ])?;
        Ok(Some(self.run(&["rev-parse", "FETCH_HEAD^{tree}"])?))
    }
}

impl Transport for GitTransport {
    fn current(&self) -> Result<Option<TreeIdentity>, TransportError> {
        self.prepare()?;
        match self.remote_tree()? {
            None => Ok(None),
            Some(tree) => Ok(Some(TreeIdentity {
                commit: self.run(&["rev-parse", "FETCH_HEAD"])?,
                tree,
            })),
        }
    }

    fn push(&self, tree: &ChannelTree, message: &str) -> Result<PushOutcome, TransportError> {
        let staged = self.write_tree(tree)?;
        if self.remote_tree()?.as_deref() == Some(staged.as_str()) {
            return Ok(PushOutcome::Unchanged);
        }
        // No parent: the branch is a snapshot, not a changelog.
        let commit = self.run(&["commit-tree", &staged, "-m", message])?;
        self.run(&[
            "push",
            "--quiet",
            "--force",
            &self.remote,
            &format!("{commit}:refs/heads/{}", self.branch),
        ])?;
        Ok(PushOutcome::Published { commit })
    }
}
