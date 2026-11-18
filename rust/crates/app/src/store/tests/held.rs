//! `graph.load-more` and D72: the timeline keeps the length of its list while the commits of the same history are
//! read again, after a commit, a fetch or **Refresh**, the rows listed before kept aside under those read until the
//! read reaches them or ends.

use git::read::graph::CommitPage;
use model::commit::Commit;
use model::graph::Row;
use model::{CommitId, Timestamp};
use views::Intent;

use model::repo::Repository;

use super::{folder, intent, open, repository_state, started, window};
use crate::store::{Found, JobId, JobKind, Message, State, reduce};

/// A line of commits, each the parent of the one after it, the newest first: commit `number` down to commit 1.
fn line(count: usize) -> Vec<Row> {
    let id = |number: usize| CommitId::parse(&format!("{number:040x}")).unwrap();
    (1..=count)
        .rev()
        .map(|number| {
            Row::Commit(Commit {
                id: id(number),
                parents: if number > 1 { vec![id(number - 1)] } else { Vec::new() },
                author_name: String::from("Test Author"),
                author_email: String::from("author@example.com"),
                author_date: Timestamp(i64::try_from(number).unwrap()),
                subject: format!("Commit {number}"),
            })
        })
        .collect()
}

/// A page of a read, with more after it or not.
fn page(rows: &[Row], follows: bool) -> CommitPage {
    CommitPage {
        rows: rows.to_vec(),
        has_more: follows,
        follows,
    }
}

/// Asks for **Refresh** and gives the read it starts its first page, the repositories of the workspace found as before:
/// gives the job, whose next pages come after.
fn refreshed(state: &mut State, first: CommitPage) -> JobId {
    let mut waiting = intent(state, Intent::Refresh);
    loop {
        let mut answered = Vec::new();
        for (job, kind) in started(&waiting) {
            match kind {
                JobKind::FindRepositories { folders } => {
                    let found = folders
                        .into_iter()
                        .map(|folder| Repository {
                            folder,
                            superproject: None,
                            common_folder: None,
                        })
                        .collect();
                    answered.extend(reduce(
                        state,
                        Message::RepositoriesFound {
                            job,
                            found: Found::Repositories(found),
                        },
                    ));
                }
                JobKind::ReadRepository { folder, .. } => {
                    let _effects = reduce(
                        state,
                        Message::RepositoryRead {
                            job,
                            result: Ok(Box::new((
                                repository_state(&folder),
                                Some(first),
                                git::read::worktree::CountedWorktrees::default(),
                            ))),
                            folder,
                            read_at: 0,
                        },
                    );
                    return job;
                }
                _ => {}
            }
        }
        assert!(!answered.is_empty(), "Refresh reads the repository");
        waiting = answered;
    }
}

/// Gives the read its next page.
fn next(state: &mut State, job: JobId, page: CommitPage) {
    let _effects = reduce(state, Message::PagesRead { job, result: Ok(page) });
}

/// A window that shows `/work/app` with the 3,000 commits of a line, all read.
fn listing() -> (State, Vec<Row>) {
    let mut state = window();
    let app = folder("app");
    let _opened = open(&mut state, &app, &[(&app, std::slice::from_ref(&app))]);
    let commits = line(3_000);
    let _job = refreshed(&mut state, page(&commits, false));
    assert_eq!(state.timeline.rows, commits);
    (state, commits)
}

#[test]
fn graph_load_more__a_commit_made_keeps_the_rows_listed_before_under_those_read_until_the_read_reaches_them() {
    let (mut state, before) = listing();
    let lanes = state.timeline.layout.len();
    let moved = line(3_001);
    let job = refreshed(&mut state, page(moved.get(..1_000).unwrap(), true));
    assert_eq!(
        state.timeline.rows.len(),
        1_000,
        "the rows read are those of the first page"
    );
    let awaited = state.timeline.awaited.as_ref().unwrap();
    assert_eq!(awaited.rows, before, "the rows listed before are kept aside");
    assert_eq!(awaited.layout.len(), lanes, "with their layout");
    assert!(awaited.layout.labels.is_empty(), "without their labels");
    next(&mut state, job, page(moved.get(1_000..2_000).unwrap(), true));
    assert_eq!(state.timeline.rows.len(), 2_000);
    assert!(state.timeline.awaited.is_some());
    next(&mut state, job, page(moved.get(2_000..).unwrap(), false));
    assert_eq!(state.timeline.rows, moved);
    assert_eq!(state.timeline.awaited, None, "the read reached them");
}

#[test]
fn graph_load_more__a_read_that_finds_the_same_commits_or_ends_with_fewer_keeps_nothing_aside_after() {
    let (mut state, before) = listing();
    let job = refreshed(&mut state, page(before.get(..1_000).unwrap(), true));
    assert_eq!(state.timeline.rows, before, "the same commits change none of the rows");
    assert_eq!(state.timeline.awaited, None);
    next(&mut state, job, page(before.get(1_000..1_200).unwrap(), false));
    assert_eq!(state.timeline.rows.len(), 1_200, "a shorter history drops the rest");
    assert_eq!(state.timeline.awaited, None);

    let older = line(2_500);
    let job = refreshed(&mut state, page(older.get(..1_000).unwrap(), true));
    assert!(state.timeline.awaited.is_some(), "a reset to an older commit");
    next(&mut state, job, page(older.get(1_000..1_100).unwrap(), false));
    assert_eq!(state.timeline.rows.len(), 1_100);
    assert_eq!(state.timeline.awaited, None, "the read ended");
}
