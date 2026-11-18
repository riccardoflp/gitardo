//! A page of the graph: the commits listed, newest first, with the stashes among them.

use std::ffi::OsString;

use model::graph::{PageSize, Row, StashCommit};
use model::state::Stash;

use super::pages::Pager;
use crate::read::graph::History;
use crate::read::log_records::{FORMAT, log_arguments, parse_commits};
use crate::{Git, GitError, Taken};

/// How many commits one read of the timeline lists at most: past them, the next read starts from the last of them
/// (D54, Q435).
pub const READ_LIMIT: usize = 100_000;

/// Which commits the graph reads.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CommitsRequest {
    /// Which history is listed.
    pub history: History,
    /// How many commits a page has: `graph.pageSize`.
    pub page_size: PageSize,
    /// How many commits the read lists at most, after those it leaves out: [`READ_LIMIT`] for the timeline of the
    /// window, the pages asked for by a client that asks for pages.
    pub count: usize,
    /// How many of the newest commits are left out, since the window has them already: the read starts after them
    /// (`graph.load-more`).
    pub skip: usize,
    /// The stashes to place among the commits: those of the repository while `graph.showStashes` is on, none while it
    /// is off. They are read only while nothing narrows the history but the branches shown.
    pub stashes: Vec<Stash>,
}

impl CommitsRequest {
    /// The same request, for as many commits as some pages of it hold, from the newest commit, but those left out.
    #[must_use]
    pub fn with_pages(self, pages: usize) -> CommitsRequest {
        let count = self
            .page_size
            .get()
            .saturating_mul(pages.max(1))
            .saturating_sub(self.skip);
        CommitsRequest { count, ..self }
    }
}

/// Some commits of the graph read, as rows.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CommitPage {
    /// The commits, newest first, each before its parents, with the stashes among them.
    pub rows: Vec<Row>,
    /// Whether the history has more commits than these and those read before them: more pages of the same read, or
    /// commits past the end of the read.
    pub has_more: bool,
    /// Whether more pages of the same read come after this one.
    pub follows: bool,
}

/// How a read of the commits ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ReadEnd {
    /// Whether the history has more commits than the read listed.
    pub has_more: bool,
}

/// `git log -z` with the format of every list, one commit more than the read lists (which says whether there are
/// more), the commits left out skipped, and the history listed.
#[must_use]
pub fn commits_arguments(request: &CommitsRequest) -> Vec<OsString> {
    let mut arguments = log_arguments();
    arguments.push(OsString::from(format!(
        "--max-count={}",
        request.count.saturating_add(1)
    )));
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

/// The commits the graph lists, newest first, with their parents, a page at a time as git prints them: each page of
/// `graph.pageSize` commits is given to `page` as soon as it is read, and the last commits when git ends, up to the
/// count asked for, but the newest ones the request leaves out; the end says whether there are more. With
/// `graph.firstParent` each commit keeps only its first parent. The stashes asked for are read first and placed among
/// the commits of each page while nothing narrows the history but the branches shown; `refs/stash` is never part of
/// the selection, since the commits of a stash would clutter the graph. Cancelled, the read kills its git and gives no
/// more pages.
///
/// Feature: graph.read-commits
///
/// # Errors
///
/// When git cannot be started, fails, is cancelled, or prints what this does not understand.
pub fn read_commits(
    git: &Git,
    request: &CommitsRequest,
    page: &mut dyn FnMut(CommitPage),
) -> Result<ReadEnd, GitError> {
    let shows_stashes = !request.stashes.is_empty() && request.history.narrowing.is_empty();
    let stashes = if shows_stashes {
        parse_stash_commits(&git.run_read(&stashes_arguments(&request.stashes))?, &request.stashes)?
    } else {
        Vec::new()
    };
    let mut pager = Pager::new(request.page_size.get(), request.count, request.history.parents, stashes);
    let mut unread: Vec<u8> = Vec::new();
    git.run_read_as_printed(&commits_arguments(request), &mut |part| {
        unread.extend_from_slice(part);
        let Some(end) = unread.iter().rposition(|byte| *byte == 0) else {
            return Ok(Taken::More);
        };
        let records: Vec<u8> = unread.drain(..=end).collect();
        for commit in parse_commits(&records)? {
            for full in pager.push(commit) {
                page(full);
                // A part of what git printed can hold many pages: once one cancels the read, none follows it.
                if git.is_cancelled() {
                    return Ok(Taken::Enough);
                }
            }
            if pager.has_enough() {
                return Ok(Taken::Enough);
            }
        }
        Ok(Taken::More)
    })?;
    if !pager.has_enough() {
        for commit in parse_commits(&unread)? {
            for full in pager.push(commit) {
                page(full);
            }
        }
    }
    let last = pager.finish();
    let end = ReadEnd {
        has_more: last.has_more,
    };
    page(last);
    Ok(end)
}

/// Every commit of a read in one page, for a client that waits for all of them, such as the extension before
/// version 1.8 of the protocol.
///
/// # Errors
///
/// When the read fails, as [`read_commits`] says.
pub fn read_commits_at_once(git: &Git, request: &CommitsRequest) -> Result<CommitPage, GitError> {
    let mut rows = Vec::new();
    let end = read_commits(git, request, &mut |page| rows.extend(page.rows))?;
    Ok(CommitPage {
        rows,
        has_more: end.has_more,
        follows: false,
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
            count: 0,
            skip: 0,
            stashes: Vec::new(),
        }
        .with_pages(3);
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
            count: 0,
            skip: 199,
            stashes: Vec::new(),
        }
        .with_pages(3);
        let arguments: Vec<String> = commits_arguments(&request)
            .iter()
            .map(|argument| argument.to_string_lossy().into_owned())
            .collect();
        assert_eq!(arguments[3..], ["--max-count=102", "--skip=199", "HEAD", "--"]);
    }
}
