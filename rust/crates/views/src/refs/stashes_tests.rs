//! The Stashes list of the branch rail made from stashes made by hand (`stash.pane`).

#![allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]

use std::collections::{BTreeMap, BTreeSet};

use model::details::CommitDetail;
use model::state::Stash;
use model::{CommitId, Timestamp};

use super::rows::{Entry, ListHead, Marks, RailRow};
use super::stashes::{StashFilesRead, StashLine, StashesShown, with_stash_rows};
use crate::intent::RailList;

/// No stash unfolded.
static UNFOLDED: BTreeSet<CommitId> = BTreeSet::new();
/// No file read.
static FILES: BTreeMap<CommitId, StashFilesRead> = BTreeMap::new();

/// The Stashes list with no stash unfolded, for the tests of the rest of the rail.
pub(super) fn nothing_unfolded() -> StashesShown<'static> {
    StashesShown {
        unfolded: &UNFOLDED,
        files: &FILES,
        is_operation_running: false,
        now: 0,
    }
}

/// A stash made on a commit of a test.
fn stash() -> Stash {
    Stash {
        commit: CommitId::parse(&"5".repeat(40)).unwrap_or_else(|| panic!("no hash")),
        index: 0,
        message: String::from("On main: half done"),
        date: Timestamp(0),
    }
}

/// The title of the Stashes list open, with so many stashes.
fn title(all: usize) -> RailRow<'static> {
    RailRow::Kept(ListHead {
        list: RailList::Stashes,
        matches: all,
        all,
        is_typed: false,
        is_open: true,
    })
}

/// The lines under a stash unfolded, with what was read of it.
fn lines_with(read: Option<StashFilesRead>) -> Vec<String> {
    let stash = stash();
    let unfolded = BTreeSet::from([stash.commit.clone()]);
    let files: BTreeMap<CommitId, StashFilesRead> = read.into_iter().map(|read| (stash.commit.clone(), read)).collect();
    let shown = StashesShown {
        unfolded: &unfolded,
        files: &files,
        is_operation_running: false,
        now: 0,
    };
    let rows = vec![
        title(1),
        RailRow::Entry {
            list: RailList::Stashes,
            entry: Entry::Stash(&stash),
            marks: Marks::default(),
        },
    ];
    with_stash_rows(rows, &shown)
        .iter()
        .filter_map(|row| match row {
            RailRow::StashLine { line, .. } => Some(match line {
                StashLine::Loading => String::from("Loading"),
                StashLine::NoFiles => String::from("No files"),
                StashLine::Failed => String::from("Failed"),
                StashLine::File { file, is_untracked, .. } => {
                    format!("{}{}", file.path, if *is_untracked { " (untracked)" } else { "" })
                }
            }),
            _ => None,
        })
        .collect()
}

#[test]
fn stash_pane__a_stash_unfolded_says_loading_while_it_is_read_and_no_files_when_it_has_none() {
    assert_eq!(lines_with(None), ["Loading"]);
    assert_eq!(lines_with(Some(StashFilesRead::Loading)), ["Loading"]);
    let empty = CommitDetail {
        id: Some(stash().commit),
        parents: Vec::new(),
        author: None,
        committer: None,
        message: String::new(),
        files: Vec::new(),
        untracked_files: Vec::new(),
    };
    assert_eq!(lines_with(Some(StashFilesRead::Read(Box::new(empty)))), ["No files"]);
}

#[test]
fn stash_pane__the_list_open_with_no_stash_says_how_to_make_one_and_closed_says_nothing() {
    let shown = nothing_unfolded();
    assert_eq!(with_stash_rows(vec![title(0)], &shown), [title(0), RailRow::NoStashes]);
    let RailRow::Kept(mut closed) = title(0) else {
        return;
    };
    closed.is_open = false;
    assert_eq!(
        with_stash_rows(vec![RailRow::Kept(closed.clone())], &shown),
        [RailRow::Kept(closed)]
    );
}
