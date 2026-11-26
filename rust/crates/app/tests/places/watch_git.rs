//! Looking again at the git folder of the repository shown, on a real repository (`repo.watch-git`, D93): a commit
//! made while the watcher gives nothing is found within three seconds, one the watcher said is not found again, and a
//! watcher that lost changes asks for a read (`repo.watch-changes`).

use std::path::Path;
use std::sync::mpsc::{self, Receiver};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use app::store::{Effect, JobKind, Message, State};
use app::work::{Deliver, GitFolderWatch, RepositoryWatch, change_of, common_git_folder};
use model::repo::{Change, ReadCause, watch_changes};
use platform::WatchEvent;

use super::support::run;
use super::watch::reads_after;
use super::{repository_with_history, showing};

/// No change said.
const NOTHING: [(Vec<Change>, Duration); 0] = [];

/// Where the messages of the watchers of a test go, and where they come.
fn channel() -> (Deliver, Receiver<Message>) {
    let (sender, receiver) = mpsc::channel();
    let sender = Mutex::new(sender);
    let deliver: Deliver = Arc::new(move |message| {
        let _sent = sender.lock().unwrap().send(message);
    });
    (deliver, receiver)
}

/// The changes said within some time, each with its moment.
fn said_within(receiver: &Receiver<Message>, time: Duration) -> Vec<(Vec<Change>, Duration)> {
    let until = Instant::now() + time;
    let mut said = Vec::new();
    while let Some(left) = until.checked_duration_since(Instant::now()) {
        if let Ok(Message::Changed { changes, at }) = receiver.recv_timeout(left) {
            said.push((changes, at));
        }
    }
    said
}

/// How many reads of the repository the store starts for changes said at their moments, the timer of each read going
/// off when it is due, as in the window.
fn reads_for(state: &mut State, said: Vec<(Vec<Change>, Duration)>) -> usize {
    let mut effects = Vec::new();
    let mut last = Duration::ZERO;
    for (changes, at) in said {
        effects.extend(run(state, Message::Wake { at }));
        effects.extend(run(state, Message::Changed { changes, at }));
        last = at;
    }
    effects.extend(run(
        state,
        Message::Wake {
            at: last + Duration::from_secs(10),
        },
    ));
    effects
        .iter()
        .filter(|effect| matches!(effect, Effect::Start(job) if matches!(job.kind, JobKind::ReadRepository { .. })))
        .count()
}

/// Makes a commit in a repository as a terminal would.
fn commit_outside(folder: &Path) {
    std::fs::write(folder.join("outside.txt"), "made outside\n").unwrap();
    for arguments in [
        &["add", "outside.txt"][..],
        &["commit", "--quiet", "-m", "Made outside"],
    ] {
        let done = std::process::Command::new("git")
            .args(arguments)
            .current_dir(folder)
            .output()
            .unwrap();
        assert!(done.status.success(), "{}", String::from_utf8_lossy(&done.stderr));
    }
}

#[test]
fn repo_watch_git__with_the_watcher_giving_nothing_a_commit_made_outside_is_shown_within_three_seconds() {
    let (repository, _remote) = repository_with_history();
    let folder = repository.path().to_path_buf();
    let mut state = showing(folder.clone());
    let (deliver, receiver) = channel();
    // No watcher of the repository is started: only the thread that looks again.
    let looks = GitFolderWatch::new(Instant::now(), deliver);
    looks.show(Some(&folder));
    assert_eq!(said_within(&receiver, Duration::from_millis(300)), NOTHING);
    commit_outside(&folder);
    let said = said_within(&receiver, Duration::from_secs(3));
    assert_eq!(said.len(), 1, "{said:?}");
    assert_eq!(said[0].0, vec![Change::GitFolder("".into())]);
    let request = watch_changes(&said[0].0[0]).unwrap();
    assert_eq!(model::activity::cause_words(&request.cause), ".git changed");
    assert_eq!(reads_for(&mut state, said), 1);
    assert_eq!(
        said_within(&receiver, Duration::from_millis(2_500)),
        NOTHING,
        "it is found once"
    );
}

#[test]
fn repo_watch_git__a_change_both_the_watcher_and_the_comparison_saw_asks_for_one_read() {
    let (repository, _remote) = repository_with_history();
    let folder = repository.path().to_path_buf();
    let mut state = showing(folder.clone());
    let (deliver, receiver) = channel();
    let origin = Instant::now();
    let looks = GitFolderWatch::new(origin, Arc::clone(&deliver));
    let watch = RepositoryWatch::telling(&folder, origin, &deliver, looks.said()).unwrap();
    looks.show(Some(&folder));
    let _setup = said_within(&receiver, Duration::from_millis(500));
    commit_outside(&folder);
    let said = said_within(&receiver, Duration::from_millis(4_500));
    let of_the_watcher = said
        .iter()
        .flat_map(|(changes, _)| changes)
        .any(|change| matches!(change, Change::GitFolder(path) if !path.as_os_str().is_empty()));
    assert!(of_the_watcher, "the watcher said the commit: {said:?}");
    let found = said
        .iter()
        .flat_map(|(changes, _)| changes)
        .filter(|change| **change == Change::GitFolder("".into()))
        .count();
    assert_eq!(found, 0, "what the watcher said is not found again: {said:?}");
    assert_eq!(reads_for(&mut state, said), 1);
    drop(watch);
}

#[test]
fn repo_watch_changes__a_watcher_that_lost_changes_asks_for_a_read() {
    let (repository, _remote) = repository_with_history();
    let folder = repository.path().to_path_buf();
    let git_folder = common_git_folder(&folder).unwrap();
    let of_the_tree = change_of(&folder, Some(&git_folder), &WatchEvent::Overflowed(folder.clone())).unwrap();
    assert_eq!(of_the_tree, Change::WorkingTree(".".into()));
    let request = watch_changes(&of_the_tree).unwrap();
    assert_eq!(request.delay, Duration::from_millis(1_000));
    assert_eq!(model::activity::cause_words(&request.cause), "files changed");
    let mut state = showing(folder.clone());
    assert_eq!(reads_after(&mut state, vec![of_the_tree]), 1);
    // The git folder of a worktree is outside its working tree, and watched on its own.
    let of_the_git_folder = change_of(&folder, Some(&git_folder), &WatchEvent::Overflowed(git_folder.clone())).unwrap();
    assert_eq!(of_the_git_folder, Change::GitFolder("".into()));
    let request = watch_changes(&of_the_git_folder).unwrap();
    assert_eq!(request.delay, Duration::from_millis(400));
    assert_eq!(request.cause, ReadCause::GitFile("".into()));
    assert_eq!(reads_after(&mut state, vec![of_the_git_folder]), 1);
}
