//! What a repository or a worktree is on, as the rows of **Repositories** and the entries of their **▾** say it
//! (`repo.worktree-summary`).

/// The branch checked out in a folder, how many files are changed there, and how many commits are to pull and to push.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct WorktreeSummary {
    /// What is checked out.
    pub head: SummaryHead,
    /// How many files are changed and not committed, untracked files included.
    pub changed: u32,
    /// How far the branch is from its upstream, `(ahead, behind)`; none for a branch without one, or detached.
    pub upstream: Option<(u32, u32)>,
}

/// What a folder has checked out.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub enum SummaryHead {
    /// A branch, by its short name.
    Branch(String),
    /// A commit with no branch, by its short hash.
    Detached(String),
    /// A branch with no commit yet.
    #[default]
    NoCommits,
    /// The folder is gone.
    Missing,
}

impl WorktreeSummary {
    /// The summary of a folder that is gone.
    #[must_use]
    pub fn missing() -> WorktreeSummary {
        WorktreeSummary {
            head: SummaryHead::Missing,
            changed: 0,
            upstream: None,
        }
    }

    /// The commits to pull: how far the branch is behind its upstream, none without one.
    #[must_use]
    pub fn behind(&self) -> u32 {
        self.upstream.map_or(0, |(_, behind)| behind)
    }

    /// The commits to push: how far the branch is ahead of its upstream, none without one.
    #[must_use]
    pub fn ahead(&self) -> u32 {
        self.upstream.map_or(0, |(ahead, _)| ahead)
    }

    /// The summary of a repository from its own read (`repo.read-state`): what the row of the repository shown says,
    /// with no other read.
    #[must_use]
    pub fn of_state(state: &crate::state::RepositoryState) -> WorktreeSummary {
        let head = match (state.checked_out_branch(), &state.head.commit) {
            (Some(branch), _) => SummaryHead::Branch(branch.to_owned()),
            (None, Some(commit)) => SummaryHead::Detached(commit.short().to_owned()),
            (None, None) => SummaryHead::NoCommits,
        };
        let upstream = state.checked_out_branch().and_then(|name| {
            let branch = state.branches.iter().find(|branch| branch.name == name)?;
            let upstream = branch.upstream.as_ref().filter(|upstream| !upstream.is_gone)?;
            Some((upstream.ahead, upstream.behind))
        });
        WorktreeSummary {
            head,
            changed: u32::try_from(state.working_tree.file_count()).unwrap_or(u32::MAX),
            upstream,
        }
    }
}
