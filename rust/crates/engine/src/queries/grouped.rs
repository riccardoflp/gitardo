//! The answers of queries read in groups: those about branches, and those the engine answers before asking git.

use git::read::branch;
use git::{Git, GitError};
use model::contract::{Answer, Query};

/// The answer of a query about branches: how far apart two are, whether one holds another, which are merged or hold a
/// commit, and whether a revision has a file.
pub(super) fn branch_reads(git: &Git, query: &Query) -> Result<Answer, GitError> {
    match query {
        Query::Counts { left, right } => {
            branch::read_counts(git, left, right).map(|(left, right)| Answer::Counts { left, right })
        }
        Query::IsAncestor { ancestor, descendant } => {
            branch::read_is_ancestor(git, ancestor, descendant).map(Answer::Flag)
        }
        Query::MergedBranches { into } => branch::read_merged(git, into).map(Answer::Names),
        Query::Containing { commit } => branch::read_containing(git, commit.as_str()).map(Answer::Names),
        Query::HasFile { revision, path } => branch::read_has_file(git, revision, path).map(Answer::Flag),
        _ => Ok(Answer::Names(Vec::new())), // Only the queries about branches come here.
    }
}

/// What a query the engine answers before asking git anything gives when it reaches git: none of them does.
pub(super) fn answered_by_engine(query: &Query) -> Answer {
    match query {
        // The engine answers it with its credential store.
        Query::GitHubToken { .. } | Query::HostingToken { .. } => Answer::Token(None),
        // The engine answers it with the hosting service.
        Query::PullRequestList { .. } => Answer::PullList(model::hosting::PullList::Unread(String::from(
            "The pull requests are listed by the engine.",
        ))),
        // The engine answers it with the AI provider.
        Query::AiModels { .. } => Answer::AiModels(model::ai::AiModels::Failed(String::from(
            "The models of the AI provider are listed by the engine.",
        ))),
        // The engine asks the window for it.
        Query::Clipboard => Answer::Clipboard(None),
        _ => Answer::Journal(None), // The journal, the last of those the engine answers, comes here.
    }
}
