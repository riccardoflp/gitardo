//! A repository shown for the first time is shown in two steps (`repo.read-first`): what the timeline needs with the
//! first commits, then the rest of its state, no row moving; a read that follows a change shows the state once, whole.

use std::collections::VecDeque;
use std::fs;
use std::path::PathBuf;
use std::time::Duration;

use app::store::{Effect, Job, JobKind, Message, State, reduce};
use app::work::jobs;
use git::CancelToken;
use model::repo::Change;
use testkit::Repository;
use views::Intent;

use super::support::{give_held, run, run_holding_rest};

/// A repository of three commits with another worktree, `other`, that has three files not committed.
fn with_another_worktree() -> Repository {
    let mut repository = Repository::new();
    let _ = repository.commit("Add the readme", &[("README.md", "Gitardo\n")]);
    let _ = repository.commit("Read the settings at startup", &[("settings.txt", "theme\n")]);
    let _ = repository.commit("Show the branch rail", &[("rail.txt", "branches\n")]);
    repository.branch("other");
    let _ = repository.git(&["worktree", "add", "--quiet", "../other", "other"]);
    let other = repository.path().parent().unwrap().join("other");
    for name in ["a.txt", "b.txt", "c.txt"] {
        fs::write(other.join(name), "new\n").unwrap();
    }
    repository
}

/// A window that opened the repository and took the first step of its read with its first commits: the rest of its
/// state is held, with the pages after the first.
fn first_step_shown(repository: &Repository) -> (State, VecDeque<Message>) {
    let mut state = super::window();
    let mut held = VecDeque::new();
    let started = Message::Started {
        folders: vec![repository.path().to_path_buf()],
    };
    let _effects = run_holding_rest(&mut state, started, &mut held);
    (state, held)
}

/// The count of what is not committed in the worktree `other`, as the window has it.
fn count_of_other(state: &State) -> Option<usize> {
    let read = state.shown_read().unwrap();
    let other = read
        .state
        .worktrees
        .iter()
        .find(|worktree| worktree.folder.ends_with("other"));
    other.unwrap().uncommitted
}

/// The subjects of the commits of the timeline, from the top.
fn subjects(state: &State) -> Vec<String> {
    let commits = state.timeline.rows.iter().filter_map(model::graph::Row::commit);
    commits.map(|commit| commit.subject.clone()).collect()
}

/// The read of the repository among effects.
fn started_read(effects: &[Effect]) -> Job {
    let read = effects.iter().find_map(|effect| match effect {
        Effect::Start(job) if matches!(job.kind, JobKind::ReadRepository { .. }) => Some(job.clone()),
        _ => None,
    });
    read.unwrap()
}

#[test]
fn repo_read_first__a_repository_opened_shows_its_first_commits_before_the_count_of_another_worktree_and_no_row_moves()
{
    let repository = with_another_worktree();
    let (mut state, mut held) = first_step_shown(&repository);
    assert!(
        held.iter().any(|message| matches!(message, Message::RestRead { .. })),
        "the rest of the state comes after the first commits: {held:?}"
    );
    assert_eq!(subjects(&state).len(), 3, "the first commits are shown");
    assert_eq!(count_of_other(&state), None);
    let rows = state.timeline.rows.clone();
    let layout = state.timeline.layout.clone();
    give_held(&mut state, &mut held);
    assert_eq!(count_of_other(&state), Some(3), "the count is then shown");
    assert_eq!(state.timeline.rows, rows, "no row of the timeline moves");
    assert_eq!(state.timeline.layout, layout);
}

#[test]
fn repo_read_first__until_the_rest_is_read_another_worktree_shows_no_count_and_the_recent_branches_are_not_listed() {
    let repository = with_another_worktree();
    let (mut state, mut held) = first_step_shown(&repository);
    let read = state.shown_read().unwrap().state.clone();
    assert_eq!(count_of_other(&state), None, "no count, not a count of zero");
    assert!(read.reflog_subjects.is_empty(), "the recent branches wait for the rest");
    assert_eq!(read.head.branch.as_deref(), Some("refs/heads/main"));
    assert_eq!(read.branches.len(), 2, "the branch rail has its branches");
    give_held(&mut state, &mut held);
    let read = state.shown_read().unwrap().state.clone();
    assert_eq!(count_of_other(&state), Some(3));
    assert!(!read.reflog_subjects.is_empty(), "the rest came");
    assert_eq!(read.identity.name.as_deref(), Some("Test Author"));
}

#[test]
fn repo_read_first__after_a_change_to_a_file_the_read_that_follows_shows_the_state_once_whole() {
    let repository = with_another_worktree();
    let (mut state, mut held) = first_step_shown(&repository);
    give_held(&mut state, &mut held);
    fs::write(repository.path().join("README.md"), "Gitardo, changed\n").unwrap();
    let changed = Message::Changed {
        changes: vec![Change::GitFolder(PathBuf::from("index"))],
        at: Duration::from_secs(1),
    };
    let mut effects = reduce(&mut state, changed);
    let wake = Message::Wake {
        at: Duration::from_secs(10),
    };
    effects.extend(reduce(&mut state, wake));
    let job = started_read(&effects);
    assert!(matches!(job.kind, JobKind::ReadRepository { in_steps: false, .. }));
    let messages = jobs::messages(&job, &CancelToken::new());
    let [Message::RepositoryRead { result: Ok(read), .. }] = messages.as_slice() else {
        panic!("the state comes once: {messages:?}");
    };
    assert!(!read.0.reflog_subjects.is_empty(), "the state is whole");
    assert_eq!(read.0.working_tree.unstaged.len(), 1);
    let other = read
        .0
        .worktrees
        .iter()
        .find(|worktree| worktree.folder.ends_with("other"));
    assert_eq!(other.unwrap().uncommitted, Some(3));
}

#[test]
fn repo_read_first__refresh_reads_the_state_whole_and_the_rest_of_the_read_before_it_is_not_taken() {
    let repository = with_another_worktree();
    let (mut state, mut held) = first_step_shown(&repository);
    let effects = run(&mut state, Message::Intent(Intent::Refresh));
    let job = started_read(&effects);
    assert!(matches!(job.kind, JobKind::ReadRepository { in_steps: false, .. }));
    assert_eq!(count_of_other(&state), Some(3), "Refresh shows the state whole");
    let generation = state.shown_read().unwrap().generation;
    give_held(&mut state, &mut held);
    assert_eq!(
        state.shown_read().unwrap().generation,
        generation,
        "the rest of the read before is late"
    );
}
