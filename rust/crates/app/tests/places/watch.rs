//! The watcher of the repository shown, on a real repository (`repo.watch-changes`): a file written asks for a read,
//! and the reads of the window, with the daemon of `core.fsmonitor` writing its cookies, ask for none.

use std::path::Path;
use std::sync::mpsc::{self, Receiver};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use app::store::{Effect, Message, State};
use app::work::{Deliver, RepositoryWatch, change_of, common_git_folder};
use model::repo::{Change, watch_changes};
use platform::WatchEvent;

use super::support::run;
use super::{repository_with_history, showing};

/// A watcher of a working tree whose messages come to a channel.
pub(super) fn watched(tree: &Path) -> (RepositoryWatch, Receiver<Message>) {
    let (sender, receiver) = mpsc::channel();
    let sender = Mutex::new(sender);
    let deliver: Deliver = Arc::new(move |message| {
        let _sent = sender.lock().unwrap().send(message);
    });
    let watch = RepositoryWatch::new(tree, Instant::now(), &deliver).unwrap();
    (watch, receiver)
}

/// The changes the watcher sent within some time.
pub(super) fn changes_within(receiver: &Receiver<Message>, time: Duration) -> Vec<Change> {
    let until = Instant::now() + time;
    let mut changes = Vec::new();
    while let Some(left) = until.checked_duration_since(Instant::now()) {
        if let Ok(Message::Changed { changes: more, .. }) = receiver.recv_timeout(left) {
            changes.extend(more);
        }
    }
    changes
}

/// Reduces the changes into the store, then the moment every read they ask for is due; gives the reads started.
pub(super) fn reads_after(state: &mut State, changes: Vec<Change>) -> usize {
    let mut effects = run(
        state,
        Message::Changed {
            changes,
            at: Duration::from_secs(10),
        },
    );
    effects.extend(run(
        state,
        Message::Wake {
            at: Duration::from_secs(20),
        },
    ));
    effects
        .iter()
        .filter(|effect| matches!(effect, Effect::Start(_)))
        .count()
}

#[test]
fn repo_watch_changes__with_core_fsmonitor_and_nothing_changing_the_repository_is_read_once_and_then_not_again() {
    let (repository, _remote) = repository_with_history();
    let _set = repository.git(&["config", "core.fsmonitor", "true"]);
    let folder = repository.path().to_path_buf();
    let (watch, receiver) = watched(&folder);
    let mut state = showing(folder.clone());
    assert!(state.shown_read().is_some(), "the repository is read once");
    let changes = changes_within(&receiver, Duration::from_millis(1_500));
    let asking: Vec<&Change> = changes
        .iter()
        .filter(|change| watch_changes(change).is_some())
        .collect();
    assert_eq!(asking, [] as [&Change; 0], "the read asks for no read");
    assert_eq!(reads_after(&mut state, changes), 0);
    drop(watch);
    let _stopped = std::process::Command::new("git")
        .args(["fsmonitor--daemon", "stop"])
        .current_dir(&folder)
        .output();
}

#[test]
fn repo_watch_changes__a_file_written_in_the_working_tree_asks_for_a_read() {
    let (repository, _remote) = repository_with_history();
    let folder = repository.path().to_path_buf();
    let mut state = showing(folder.clone());
    let (watch, receiver) = watched(&folder);
    std::fs::write(folder.join("notes.txt"), "written by another program\n").unwrap();
    let changes = changes_within(&receiver, Duration::from_millis(1_500));
    assert!(
        changes.contains(&Change::WorkingTree("notes.txt".into())),
        "{changes:?}"
    );
    assert_eq!(reads_after(&mut state, changes), 1);
    drop(watch);
}

#[test]
fn repo_watch_changes__paths_are_made_relative_to_the_git_folder_or_the_working_tree() {
    let repository = testkit::Repository::new();
    let tree = repository.path();
    let git_folder = common_git_folder(tree).unwrap();
    assert_eq!(git_folder, tree.join(".git"));
    let in_git = change_of(
        tree,
        Some(&git_folder),
        &WatchEvent::Changed(git_folder.join("refs/heads/main")),
    );
    assert_eq!(in_git, Some(Change::GitFolder("refs/heads/main".into())));
    let in_tree = change_of(tree, Some(&git_folder), &WatchEvent::Changed(tree.join("src/a.rs")));
    assert_eq!(in_tree, Some(Change::WorkingTree("src/a.rs".into())));
    let outside = change_of(tree, Some(&git_folder), &WatchEvent::Changed("/elsewhere/a.rs".into()));
    assert_eq!(outside, None);
}
