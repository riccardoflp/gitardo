//! A read of the repository shown that a change asks for keeps the commits of the timeline when the commit checked out,
//! the references and the stashes are those of the read before: no `git log` runs (`repo.schedule-reads`, M15).

use std::path::PathBuf;
use std::time::Duration;

use app::store::{Effect, Job, JobKind, Message, State, reduce};
use app::work::jobs;
use git::CancelToken;
use model::repo::Change;
use views::Intent;

use super::support::run;
use super::{repository_with_history, showing};

/// The read of the repository a change asks for, once its delay is over.
fn read_for(state: &mut State, change: Change) -> Job {
    let _waits = reduce(
        state,
        Message::Changed {
            changes: vec![change],
            at: Duration::from_secs(1),
        },
    );
    let effects = reduce(
        state,
        Message::Wake {
            at: Duration::from_secs(10),
        },
    );
    started_read(&effects)
}

/// The read of the repository among effects.
fn started_read(effects: &[Effect]) -> Job {
    effects
        .iter()
        .find_map(|effect| match effect {
            Effect::Start(job) if matches!(job.kind, JobKind::ReadRepository { .. }) => Some(job.clone()),
            _ => None,
        })
        .unwrap()
}

/// Whether a read of the repository keeps the state of the read before, to compare the history with.
fn keeps(job: &Job) -> bool {
    matches!(&job.kind, JobKind::ReadRepository { kept: Some(_), .. })
}

/// Runs the read as a worker would, and gives whether it read the commits: a page came with the state.
fn read_commits(state: &mut State, job: &Job) -> bool {
    let mut messages = jobs::messages(job, &CancelToken::new()).into_iter();
    let first = messages.next().unwrap();
    let Message::RepositoryRead { result: Ok(read), .. } = &first else {
        panic!("the state is read: {first:?}");
    };
    let has_page = read.1.is_some();
    let _effects = run(state, first);
    for message in messages {
        let _effects = run(state, message);
    }
    has_page
}

#[test]
fn repo_schedule_reads__a_file_changed_and_saved_leads_to_a_read_that_runs_no_git_log_and_the_timeline_keeps_its_rows()
{
    let (repository, _remote) = repository_with_history();
    let mut state = showing(repository.path().to_path_buf());
    let rows = state.timeline.rows.clone();
    std::fs::write(repository.path().join("README.md"), "Gitardo, changed\n").unwrap();
    let job = read_for(&mut state, Change::WorkingTree(PathBuf::from("README.md")));
    assert!(keeps(&job));
    assert!(!read_commits(&mut state, &job), "no git log ran");
    assert!(matches!(
        state.timeline.rows.first(),
        Some(model::graph::Row::Uncommitted { files: 1, .. })
    ));
    assert_eq!(
        state.timeline.rows.get(1..),
        Some(rows.as_slice()),
        "the rows are kept, the draft on top"
    );
}

#[test]
fn repo_schedule_reads__a_commit_made_in_a_terminal_leads_to_a_read_that_reads_the_commits_again() {
    let (mut repository, _remote) = repository_with_history();
    let mut state = showing(repository.path().to_path_buf());
    let _ = repository.commit("Made in a terminal", &[("terminal.txt", "made\n")]);
    let job = read_for(&mut state, Change::GitFolder(PathBuf::from("refs/heads/main")));
    assert!(
        keeps(&job),
        "a change of the git folder may keep the rows, when nothing moved"
    );
    assert!(
        read_commits(&mut state, &job),
        "the history changed: the commits are read again"
    );
    let subjects: Vec<&str> = state
        .timeline
        .rows
        .iter()
        .filter_map(model::graph::Row::commit)
        .map(|commit| commit.subject.as_str())
        .collect();
    assert!(subjects.contains(&"Made in a terminal"), "{subjects:?}");
}

#[test]
fn repo_schedule_reads__refresh_reads_the_commits_again() {
    let (repository, _remote) = repository_with_history();
    let mut state = showing(repository.path().to_path_buf());
    let effects = run(&mut state, Message::Intent(Intent::Refresh));
    let job = started_read(&effects);
    assert!(!keeps(&job), "Refresh reads the commits again: {:?}", job.kind);
}
