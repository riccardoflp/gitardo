//! The tests of the folders followed: what is kept of a read, what is said of it, and when a folder is read.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use git::read::worktree::GitFolderStamp;
use model::repo::{SummaryHead, WorktreeSummary};
use model::state::Worktree;

use super::{Due, Followed, Next, Read};

/// A read of a folder that found it on `main` with this many files changed.
fn read(folder: &str, changed: u32) -> Read {
    Read {
        folder: PathBuf::from(folder),
        stamp: GitFolderStamp::default(),
        summary: Some(Ok(WorktreeSummary {
            head: SummaryHead::Branch(String::from("main")),
            changed,
            upstream: None,
        })),
        worktrees: None,
        took: Duration::ZERO,
    }
}

/// A worktree git lists.
fn worktree(folder: &str, is_missing: bool) -> Worktree {
    Worktree {
        folder: PathBuf::from(folder),
        commit: None,
        branch: Some(String::from("feature/login")),
        is_main: false,
        is_current: false,
        is_missing,
        uncommitted: None,
    }
}

#[test]
fn a_folder_asked_again_keeps_what_it_said_and_one_no_longer_asked_is_dropped() {
    let mut followed = Followed::default();
    followed.ask(&[PathBuf::from("/work/api"), PathBuf::from("/work/web")]);
    assert!(matches!(followed.next(false), Next::Read(ask) if ask.folder == Path::new("/work/api")));
    assert!(followed.keep(read("/work/api", 3)).is_some(), "its first read is said");
    followed.answering = 1;
    assert!(matches!(followed.next(false), Next::Read(ask) if ask.folder == Path::new("/work/web")));
    assert!(
        followed.keep(read("/work/web", 0)).is_none(),
        "the answer under way will say its first read"
    );
    followed.answering = 0;
    followed.ask(&[PathBuf::from("/work/api/"), PathBuf::from("/work/docs")]);
    let folders: Vec<&str> = followed
        .entries
        .iter()
        .filter_map(|entry| entry.folder.to_str())
        .collect();
    assert_eq!(folders, ["/work/api", "/work/docs"]);
    let api = followed.entries.first().unwrap();
    assert_eq!(api.summary.as_ref().map(|summary| summary.changed), Some(3));
    assert!(
        api.due.is_none(),
        "what was read is not read again for being asked again"
    );
}

#[test]
fn a_read_that_found_what_was_said_before_says_nothing_and_one_that_failed_says_it_is_stale() {
    let mut followed = Followed::default();
    followed.ask(&[PathBuf::from("/work/api")]);
    assert!(followed.keep(read("/work/api", 3)).is_some());
    assert!(followed.keep(read("/work/api", 3)).is_none(), "the same again");
    let failed = Read {
        summary: Some(Err(())),
        ..read("/work/api", 0)
    };
    let said = followed.keep(failed).unwrap();
    assert!(said.is_stale);
    assert_eq!(
        said.summary.map(|summary| summary.changed),
        Some(3),
        "what it said before"
    );
    let unchanged = Read {
        summary: None,
        ..read("/work/api", 0)
    };
    assert!(
        followed.keep(unchanged).is_none(),
        "nothing changed, nothing read, nothing said"
    );
}

#[test]
fn the_worktrees_a_folder_asked_lists_are_followed_but_one_whose_folder_is_gone() {
    let mut followed = Followed::default();
    followed.ask(&[PathBuf::from("/work/api")]);
    let listed = Read {
        worktrees: Some(Ok(vec![
            worktree("/work/api", false),
            worktree("/work/api-login", false),
            worktree("/work/api-gone", true),
        ])),
        ..read("/work/api", 0)
    };
    let said = followed.keep(listed).unwrap();
    assert_eq!(said.worktrees.map(|worktrees| worktrees.len()), Some(3));
    let folders: Vec<&str> = followed
        .entries
        .iter()
        .filter_map(|entry| entry.folder.to_str())
        .collect();
    assert_eq!(folders, ["/work/api", "/work/api-login"]);
    let again = Read {
        worktrees: Some(Ok(vec![worktree("/work/api", false)])),
        ..read("/work/api", 0)
    };
    assert!(followed.keep(again).is_some(), "the list changed");
    assert_eq!(
        followed.entries.len(),
        1,
        "a worktree no longer listed is no longer followed"
    );
}

#[test]
fn a_folder_is_read_again_two_seconds_after_its_last_read_and_never_while_something_else_runs() {
    let mut followed = Followed::default();
    followed.ask(&[PathBuf::from("/work/api")]);
    followed.is_reading = true;
    assert!(matches!(followed.next(true), Next::Wait(_)), "something else runs");
    assert!(matches!(followed.next(false), Next::Read(ask) if ask.is_forced && ask.lists_worktrees));
    assert!(followed.keep(read("/work/api", 0)).is_some());
    assert!(matches!(followed.next(false), Next::End), "nothing is due");
    assert!(!followed.is_reading);

    let entry = followed.entries.first_mut().unwrap();
    entry.due = Some(Due::default());
    assert!(
        matches!(followed.next(false), Next::Wait(wait) if wait > Duration::ZERO),
        "too soon"
    );
    let entry = followed.entries.first_mut().unwrap();
    entry.read_at = Instant::now().checked_sub(Duration::from_secs(3));
    assert!(matches!(followed.next(false), Next::Read(ask) if !ask.is_forced));
}
