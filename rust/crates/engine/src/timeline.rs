//! What a read of the timeline asks of git (`graph.read-commits`): the history of the branches shown, as the window
//! of the desktop and the extension ask for it, turned into git's arguments here, so that neither builds them.

use std::ffi::OsString;

use git::read::graph::{CommitsRequest, History, Parents};
use model::CommitId;
use model::filter::{BranchesShown, checked, log_arguments};
use model::graph::PageSize;
use model::refs::References;
use model::state::RepositoryState;

/// How the timeline of a repository is read.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TimelineRequest {
    /// How many commits a page has.
    pub page_size: PageSize,
    /// How many pages are read, from the newest commit.
    pub pages: usize,
    /// Which parents are followed (`graph.firstParent`).
    pub parents: Parents,
    /// Whether the stashes are placed among the commits (`graph.showStashes`).
    pub shows_stashes: bool,
    /// The branches shown, as chosen: the read checks them against the references it finds (`filter.remember`).
    pub shown: Option<BranchesShown>,
    /// What narrows the history further: the arguments of a search shown as a list (`search.list-mode`).
    pub narrowing: Vec<OsString>,
    /// The commit whose history alone is listed, in place of the branches shown: a commit gone to that is on no branch
    /// or tag (`search.go-to-hash`).
    pub alone: Option<CommitId>,
}

/// The history the timeline lists, in date order, each commit before its parents: the branches shown, checked against
/// the references the repository has now, as `filter.log-arguments` gives them, with the upstream of the branch
/// checked out for **Upstream**.
#[must_use]
pub fn history_of(repository: &RepositoryState, shown: Option<&BranchesShown>, parents: Parents) -> History {
    let shown = checked(shown, &References::of(repository));
    let upstream = repository
        .branches
        .iter()
        .find(|branch| branch.is_checked_out)
        .and_then(|branch| branch.upstream.as_ref())
        .map(|upstream| upstream.reference.as_str())
        .filter(|reference| reference.starts_with("refs/"));
    History {
        selection: log_arguments(shown.as_ref(), repository.head.commit.as_ref(), upstream),
        narrowing: Vec::new(),
        parents,
    }
}

/// Whether a read of the timeline lists the same rows for a repository as read now as for it as read before: the same
/// commit checked out, references and stashes, and the same arguments for git (`repo.schedule-reads`).
#[must_use]
pub fn is_same_history(before: &RepositoryState, after: &RepositoryState, request: &TimelineRequest) -> bool {
    before.head.commit == after.head.commit
        && same(&before.branches, &after.branches, |one, other| {
            one.name == other.name && one.commit == other.commit
        })
        && same(&before.remote_branches, &after.remote_branches, |one, other| {
            one.name == other.name && one.commit == other.commit
        })
        && same(&before.tags, &after.tags, |one, other| {
            one.name == other.name && one.commit == other.commit
        })
        && same(&before.stashes, &after.stashes, |one, other| one.commit == other.commit)
        && commits_request(before, request) == commits_request(after, request)
}

/// Whether two lists have the same things in the same order, as a comparison of two of them says.
fn same<T>(before: &[T], after: &[T], is_same: impl Fn(&T, &T) -> bool) -> bool {
    before.len() == after.len() && before.iter().zip(after).all(|(one, other)| is_same(one, other))
}

/// What a read of the timeline asks of git, for a repository as read: the commits of the pages asked for.
#[must_use]
pub fn commits_request(repository: &RepositoryState, request: &TimelineRequest) -> CommitsRequest {
    let mut history = history_of(repository, request.shown.as_ref(), request.parents);
    if let Some(commit) = &request.alone {
        history.selection = vec![OsString::from("--date-order"), OsString::from(commit.as_str())];
    }
    history.narrowing.clone_from(&request.narrowing);
    CommitsRequest {
        history,
        page_size: request.page_size,
        count: 0,
        skip: 0,
        stashes: if request.shows_stashes {
            repository.stashes.clone()
        } else {
            Vec::new()
        },
    }
    .with_pages(request.pages)
}
