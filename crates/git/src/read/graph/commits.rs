//! A page of the graph: the commits listed, newest first, with the stashes among them.

use std::ffi::OsString;

use model::graph::{PageSize, Row, StashCommit, place_stashes};
use model::state::Stash;

use crate::read::graph::{History, Parents};
use crate::read::log_records::{FORMAT, log_arguments, parse_commits};
use crate::{Git, GitError};

/// Which commits the graph reads.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CommitsRequest {
    /// Which history is listed.
    pub history: History,
    /// How many commits a page has: `graph.pageSize`.
    pub page_size: PageSize,
    /// How many pages are read, from the newest commit; one at least.
    pub pages: usize,
    /// How many of the newest commits of those pages are left out, since the window has them already: the read
    /// starts after them (`graph.load-more`).
    pub skip: usize,
    /// The stashes to place among the commits: those of the repository while `graph.showStashes` is on, none while it
    /// is off. They are read only while nothing narrows the history but the branches shown.
    pub stashes: Vec<Stash>,
}

impl CommitsRequest {
    /// How many commits the pages asked for hold.
    #[must_use]
    pub fn count(&self) -> usize {
        self.page_size.get().saturating_mul(self.pages.max(1))
    }

    /// How many commits the read prints: those of the pages asked for, but the ones left out.
    #[must_use]
    pub fn printed(&self) -> usize {
        self.count().saturating_sub(self.skip)
    }
}

/// The commits of the pages read, as rows of the graph.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CommitPage {
    /// The commits, newest first, each before its parents, with the stashes among them.
    pub rows: Vec<Row>,
    /// Whether the history has more commits than were read.
    pub has_more: bool,
}

/// `git log -z` with the format of every list, one commit more than the pages asked for (which says whether there are
/// more), the commits left out skipped, and the history listed.
#[must_use]
pub fn commits_arguments(request: &CommitsRequest) -> Vec<OsString> {
    let mut arguments = log_arguments();
    arguments.push(OsString::from(format!("--max-count={}", request.printed() + 1)));
    if request.skip > 0 {
        arguments.push(OsString::from(format!("--skip={}", request.skip)));
    }
    arguments.extend(request.history.arguments());
    arguments
}

/// `git log --no-walk=unsorted -z` with the format of every list, over the commits of the stashes: each stash alone,
/// not what it was made on.
#[must_use]
pub fn stashes_arguments(stashes: &[Stash]) -> Vec<OsString> {
    let mut arguments: Vec<OsString> = ["log", "--no-walk=unsorted", "-z", FORMAT]
        .into_iter()
        .map(OsString::from)
        .collect();
    arguments.extend(stashes.iter().map(|stash| OsString::from(stash.commit.as_str())));
    arguments.push(OsString::from("--"));
    arguments
}

/// The commits of the stashes that `git log` printed with [`stashes_arguments`], each with its place in the list of
/// stashes; a commit that is not one of the stashes is left out.
///
/// # Errors
///
/// When a commit cannot be read.
pub fn parse_stash_commits(output: &[u8], stashes: &[Stash]) -> Result<Vec<StashCommit>, GitError> {
    Ok(parse_commits(output)?
        .into_iter()
        .filter_map(|commit| {
            let stash = stashes.iter().find(|stash| stash.commit == commit.id)?;
            Some(StashCommit {
                commit,
                index: stash.index,
            })
        })
        .collect())
}

/// The commits the graph lists, newest first, with their parents: as many pages as were asked for, but the newest
/// ones the request leaves out, and whether there are more. With `graph.firstParent` each commit keeps only its first parent. The stashes asked for are placed among
/// the commits while nothing narrows the history but the branches shown; `refs/stash` is never part of the selection,
/// since the commits of a stash would clutter the graph.
///
/// Feature: graph.read-commits
///
/// # Errors
///
/// When git cannot be started, fails, or prints what this does not understand.
pub fn read_commits(git: &Git, request: &CommitsRequest) -> Result<CommitPage, GitError> {
    let mut commits = parse_commits(&git.run_read(&commits_arguments(request))?)?;
    let has_more = commits.len() > request.printed();
    commits.truncate(request.printed());
    if request.history.parents == Parents::First {
        for commit in &mut commits {
            commit.parents.truncate(1);
        }
    }
    let rows: Vec<Row> = commits.into_iter().map(Row::Commit).collect();
    let shows_stashes = !request.stashes.is_empty() && request.history.narrowing.is_empty();
    if !shows_stashes {
        return Ok(CommitPage { rows, has_more });
    }
    let output = git.run_read(&stashes_arguments(&request.stashes))?;
    let stashes = parse_stash_commits(&output, &request.stashes)?;
    Ok(CommitPage {
        rows: place_stashes(&rows, &stashes),
        has_more,
    })
}

#[cfg(test)]
mod tests {
    use std::ffi::OsString;

    use model::graph::PageSize;

    use super::{CommitsRequest, commits_arguments};
    use crate::read::graph::{History, Parents};

    #[test]
    fn asks_one_commit_more_than_the_pages_and_puts_the_first_parent_before_the_selection() {
        let request = CommitsRequest {
            history: History {
                selection: vec![OsString::from("--date-order"), OsString::from("HEAD")],
                narrowing: Vec::new(),
                parents: Parents::First,
            },
            page_size: PageSize::from_setting(Some(100.0)),
            pages: 3,
            skip: 0,
            stashes: Vec::new(),
        };
        let arguments: Vec<String> = commits_arguments(&request)
            .iter()
            .map(|argument| argument.to_string_lossy().into_owned())
            .collect();
        assert_eq!(
            arguments[3..],
            ["--max-count=301", "--first-parent", "--date-order", "HEAD", "--"]
        );
    }

    #[test]
    fn skips_the_commits_left_out_and_asks_for_the_rest_of_the_pages() {
        let request = CommitsRequest {
            history: History {
                selection: vec![OsString::from("HEAD")],
                narrowing: Vec::new(),
                parents: Parents::All,
            },
            page_size: PageSize::from_setting(Some(100.0)),
            pages: 3,
            skip: 199,
            stashes: Vec::new(),
        };
        let arguments: Vec<String> = commits_arguments(&request)
            .iter()
            .map(|argument| argument.to_string_lossy().into_owned())
            .collect();
        assert_eq!(arguments[3..], ["--max-count=102", "--skip=199", "HEAD", "--"]);
    }
}
