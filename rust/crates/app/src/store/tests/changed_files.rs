//! The files of the working tree asked of git before they ask for a read (`repo.watch-changes`, M15): what git
//! ignores and what is in a nested worktree ask for nothing, a `.gitignore` asks at once.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use model::repo::Change;
use model::state::Worktree;

use super::{folder, open, reads, started, window};
use crate::store::{Effect, JobId, JobKind, Message, Shown, State, reduce};

/// A window showing the repository `/work/app`, read, with a worktree nested in it at `.claude/worktrees/feature`.
fn showing() -> (State, PathBuf) {
    let mut state = window();
    let app = folder("app");
    let _opened = open(&mut state, &app, &[(&app, std::slice::from_ref(&app))]);
    let Some(Shown::Read(read)) = state.workspace.as_mut().map(|workspace| &mut workspace.shown) else {
        panic!("the repository is read");
    };
    let mut repository = (*read.state).clone();
    repository.worktrees = vec![
        Worktree {
            folder: app.clone(),
            commit: None,
            branch: Some(String::from("refs/heads/main")),
            is_main: true,
            is_current: true,
            is_missing: false,
            locked: None,
            uncommitted: Some(0),
        },
        Worktree {
            folder: app.join(".claude").join("worktrees").join("feature"),
            commit: None,
            branch: Some(String::from("refs/heads/feature")),
            is_main: false,
            is_current: false,
            is_missing: false,
            locked: None,
            uncommitted: Some(0),
        },
    ];
    read.state = Arc::new(repository);
    // The window asks once what the nested worktree is on, and watches it, as after its first read
    // (`repo.worktree-summary`); that read in the background is under way for the rest of the test.
    let summaries = reduce(&mut state, Message::SummariesRead(Vec::new()));
    assert!(
        summaries
            .iter()
            .any(|effect| matches!(effect, Effect::ReadSummaries(_)))
    );
    (state, app)
}

/// Changes of files of the working tree at a moment, in milliseconds since the window opened.
fn files_changed(state: &mut State, paths: &[&str], at: u64) -> Vec<Effect> {
    let changes = paths
        .iter()
        .map(|path| Change::WorkingTree(PathBuf::from(path)))
        .collect();
    reduce(
        state,
        Message::Changed {
            changes,
            at: Duration::from_millis(at),
        },
    )
}

/// The moment the window is woken at.
fn woken(state: &mut State, at: u64) -> Vec<Effect> {
    reduce(
        state,
        Message::Wake {
            at: Duration::from_millis(at),
        },
    )
}

/// The questions to git of the effects: the job and the files asked.
fn questions(effects: &[Effect]) -> Vec<(JobId, Vec<String>)> {
    started(effects)
        .into_iter()
        .filter_map(|(job, kind)| match kind {
            JobKind::ReadIgnored { paths, .. } => Some((job, paths)),
            _ => None,
        })
        .collect()
}

/// Git's answer to a question: the files it ignores.
fn git_says(state: &mut State, job: JobId, ignored: &[&str]) -> Vec<Effect> {
    let result = Ok(ignored.iter().map(|path| (*path).to_owned()).collect());
    reduce(state, Message::IgnoredRead { job, result })
}

#[test]
fn repo_watch_changes__cargo_build_writing_in_target_asks_for_nothing_and_changing_gitignore_asks_for_a_read() {
    let (mut state, app) = showing();
    let first = files_changed(&mut state, &["target/debug/app.exe", "target/debug/app.d"], 1_000);
    assert_eq!(reads(&first), [] as [PathBuf; 0]);
    assert_eq!(first, [Effect::WakeAt(Duration::from_millis(2_000))]);
    let _more = files_changed(&mut state, &["target/debug/deps/a.rlib"], 1_500);
    let woke = woken(&mut state, 2_000);
    let asked = questions(&woke);
    assert_eq!(asked.len(), 1, "one question for the burst: {woke:?}");
    let (job, paths) = &asked[0];
    assert_eq!(paths.len(), 3);
    let answered = git_says(
        &mut state,
        *job,
        &["target/debug/app.exe", "target/debug/app.d", "target/debug/deps/a.rlib"],
    );
    assert_eq!(answered, [], "what git ignores asks for nothing");

    let gitignore = files_changed(&mut state, &[".gitignore"], 5_000);
    assert_eq!(questions(&gitignore), []);
    assert_eq!(gitignore, [Effect::WakeAt(Duration::from_millis(6_000))]);
    assert_eq!(reads(&woken(&mut state, 6_000)), [app]);
}

#[test]
fn repo_watch_changes__a_file_git_does_not_ignore_asks_for_a_read_once_git_said_so() {
    let (mut state, app) = showing();
    let _waits = files_changed(&mut state, &["target/a.o", "src/main.rs"], 0);
    let woke = woken(&mut state, 1_000);
    let (job, _) = questions(&woke)[0].clone();
    let answered = git_says(&mut state, job, &["target/a.o"]);
    assert_eq!(reads(&answered), [app]);
    let job = started(&answered)[0].0;
    assert_eq!(
        state.reads.reason(job).map(|read| read.cause.clone()),
        Some(model::repo::ReadCause::File(PathBuf::from("src/main.rs")))
    );
}

#[test]
fn repo_watch_changes__a_file_written_in_a_nested_worktree_asks_for_nothing_and_a_commit_there_asks_for_a_read() {
    let (mut state, app) = showing();
    let written = files_changed(&mut state, &[".claude/worktrees/feature/src/a.rs"], 0);
    assert_eq!(written, []);
    assert_eq!(woken(&mut state, 5_000), []);
    let committed = reduce(
        &mut state,
        Message::Changed {
            changes: vec![Change::GitFolder(PathBuf::from("worktrees/feature/HEAD"))],
            at: Duration::from_secs(10),
        },
    );
    assert_eq!(committed, [Effect::WakeAt(Duration::from_millis(10_400))]);
    assert_eq!(reads(&woken(&mut state, 10_400)), [app]);
}

#[test]
fn repo_watch_changes__files_waiting_for_git_are_forgotten_by_a_read_that_starts_after_them() {
    let (mut state, app) = showing();
    let _file = files_changed(&mut state, &["src/a.rs"], 0);
    let head = reduce(
        &mut state,
        Message::Changed {
            changes: vec![Change::GitFolder(PathBuf::from("HEAD"))],
            at: Duration::from_millis(100),
        },
    );
    assert_eq!(
        head,
        [Effect::WakeAt(Duration::from_millis(500))],
        "the first moment of the two"
    );
    let woke = woken(&mut state, 500);
    assert_eq!(reads(&woke), [app]);
    assert_eq!(
        woken(&mut state, 1_000),
        [],
        "the read sees the file: nothing is asked of git"
    );
}
