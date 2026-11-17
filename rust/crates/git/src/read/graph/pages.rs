//! The commits of a read cut into pages as git prints them, with the stashes placed among them as they would be among
//! all the commits read at once (`graph.place-stashes`).

use std::collections::BTreeMap;

use model::commit::Commit;
use model::graph::{Row, StashCommit};

use super::CommitPage;
use crate::read::graph::Parents;

/// A stash whose commit has not been read yet.
#[derive(Debug, Clone)]
struct Waiting {
    /// The stash.
    stash: StashCommit,
    /// The first commit read that is not newer than it, counted from the first commit of the read.
    by_date: Option<usize>,
}

/// The commits of one read, given a page at a time.
#[derive(Debug)]
pub(crate) struct Pager {
    /// How many commits a page has.
    page_size: usize,
    /// How many commits the read gives at most.
    count: usize,
    /// Which parents each commit keeps.
    parents: Parents,
    /// How many commits were read.
    read: usize,
    /// The commits read and not given yet, the first of them the commit numbered `held_from`.
    held: Vec<Commit>,
    /// The number of the first commit held, counted from the first commit of the read.
    held_from: usize,
    /// The stashes whose commit was not read yet.
    waiting: Vec<Waiting>,
    /// The stashes placed, by the number of the commit they go above.
    placed: BTreeMap<usize, Vec<StashCommit>>,
    /// Whether the history has more commits than the read gives.
    has_more: bool,
}

impl Pager {
    /// A pager for pages of `page_size` commits, up to `count` commits, with the stashes to place among them.
    pub(crate) fn new(page_size: usize, count: usize, parents: Parents, stashes: Vec<StashCommit>) -> Pager {
        Pager {
            page_size: page_size.max(1),
            count,
            parents,
            read: 0,
            held: Vec::new(),
            held_from: 0,
            waiting: stashes
                .into_iter()
                .map(|stash| Waiting { stash, by_date: None })
                .collect(),
            placed: BTreeMap::new(),
            has_more: false,
        }
    }

    /// Whether the read has all it wants: one commit more than it gives, which says there are more.
    pub(crate) fn has_enough(&self) -> bool {
        self.has_more
    }

    /// Takes the next commit printed, and gives the pages it completes: a page is given once a commit after it is
    /// read, and once no stash whose date puts it among its commits still waits for its own commit.
    pub(crate) fn push(&mut self, mut commit: Commit) -> Vec<CommitPage> {
        if self.read >= self.count {
            self.has_more = true;
            return Vec::new();
        }
        if self.parents == Parents::First {
            commit.parents.truncate(1);
        }
        self.place(&commit);
        self.held.push(commit);
        self.read += 1;
        let mut pages = Vec::new();
        while self.ready() >= self.page_size && self.read > self.held_from + self.page_size {
            pages.push(self.give(self.page_size, true));
        }
        pages
    }

    /// The last commits, once git ended or the read has enough: the stashes whose commit was never read are left out.
    pub(crate) fn finish(mut self) -> CommitPage {
        let rest = self.held.len();
        let has_more = self.has_more;
        CommitPage {
            follows: false,
            ..self.give(rest, has_more)
        }
    }

    /// Notes where the stashes go, from a commit read: the first commit not newer than a stash, and its own commit.
    fn place(&mut self, commit: &Commit) {
        let number = self.read;
        let mut index = 0;
        while let Some(waiting) = self.waiting.get_mut(index) {
            if waiting.by_date.is_none() && commit.author_date <= waiting.stash.commit.author_date {
                waiting.by_date = Some(number);
            }
            if waiting.stash.commit.parents.first() == Some(&commit.id) {
                let at = waiting.by_date.unwrap_or(number).min(number);
                let stash = self.waiting.swap_remove(index).stash;
                let above = self.placed.entry(at).or_default();
                above.push(stash);
                above.sort_by_key(|stash| stash.index);
            } else {
                index += 1;
            }
        }
    }

    /// How many of the commits held can be given: those above the place of the first stash still waiting for its
    /// commit.
    fn ready(&self) -> usize {
        let blocked = self.waiting.iter().filter_map(|waiting| waiting.by_date).min();
        blocked.unwrap_or(self.read).saturating_sub(self.held_from)
    }

    /// Gives the first commits held, with the stashes placed above them, as a page that more pages follow.
    fn give(&mut self, length: usize, has_more: bool) -> CommitPage {
        let length = length.min(self.held.len());
        let commits: Vec<Commit> = self.held.drain(..length).collect();
        let mut rows = Vec::with_capacity(commits.len());
        for (offset, commit) in commits.into_iter().enumerate() {
            if let Some(stashes) = self.placed.remove(&(self.held_from + offset)) {
                rows.extend(stashes.into_iter().map(stash_row));
            }
            rows.push(Row::Commit(commit));
        }
        self.held_from += length;
        CommitPage {
            rows,
            has_more,
            follows: true,
        }
    }
}

/// The row of a stash: it keeps only its first parent, the commit it was made on.
fn stash_row(stash: StashCommit) -> Row {
    let StashCommit { mut commit, index } = stash;
    commit.parents.truncate(1);
    Row::Stash { commit, index }
}
