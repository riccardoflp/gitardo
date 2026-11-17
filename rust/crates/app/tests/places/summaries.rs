//! What the repositories open and their worktrees are on, read in the background (`repo.worktree-summary`): the
//! worktree `login` of `api`, not shown, read once the window opens, again when a file of it is saved, and not when
//! nothing changed; and its entry in the **▾** of `api` (`repo.group-tabs`).

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use app::store::{Effect, Message, State, ViewData};
use app::work::summaries::{SummaryAsk, read_summaries};
use git::read::worktree::git_folder_changed;
use model::repo::{Change, SummaryHead};
use views::repo::{TabSummary, counts_words};

use super::support::{self, NOW};
use super::where_one_is::api_with_login;

/// What the entry of a folder in the **▾** of the first row says it is on.
fn member_summary(state: &State, folder: &Path) -> Option<TabSummary> {
    let data = ViewData::new(state, Vec::new(), NOW);
    let side = data.window_state(state).side.unwrap();
    side.tabs
        .tabs
        .first()?
        .members
        .iter()
        .find(|member| app::persisted::same_folder(&member.folder, folder))
        .and_then(|member| member.summary.clone())
}

/// A file of the working tree changed, as the watcher of a folder says it.
fn saved(folder: &Path, name: &str) -> Message {
    Message::TabChanged {
        folder: folder.to_path_buf(),
        changes: vec![Change::WorkingTree(PathBuf::from(name))],
        at: Duration::from_secs(60),
    }
}

#[test]
fn repo_worktree_summary__a_worktree_not_shown_is_read_once_the_window_opens_and_watched() {
    let (_kept, _clones, login, state) = api_with_login();
    let summary = member_summary(&state, &login).unwrap();
    assert_eq!(summary.summary.head, SummaryHead::Branch(String::from("feature/login")));
    assert_eq!(counts_words(&summary), "");
    assert!(
        state
            .tabs
            .background
            .watched
            .iter()
            .any(|folder| app::persisted::same_folder(folder, &login)),
        "its folder is watched"
    );
}

#[test]
fn repo_worktree_summary__saving_a_file_in_a_worktree_not_shown_updates_its_count_within_two_seconds() {
    let (_kept, _clones, login, mut state) = api_with_login();
    // The window has been open a while: the first read of the worktree is more than two seconds old.
    std::thread::sleep(Duration::from_secs(2));
    std::fs::write(login.join("notes.txt"), "new\n").unwrap();
    let started = Instant::now();
    let effects = support::run(&mut state, saved(&login, "notes.txt"));
    assert!(effects.iter().any(|effect| matches!(effect, Effect::ReadSummaries(_))));
    assert!(started.elapsed() < Duration::from_secs(2), "{:?}", started.elapsed());
    assert_eq!(member_summary(&state, &login).unwrap().summary.changed, 1);
}

#[test]
fn repo_worktree_summary__nothing_changing_in_a_worktree_runs_no_read_of_it() {
    let (_kept, _clones, login, mut state) = api_with_login();
    let effects = support::run(
        &mut state,
        Message::Wake {
            at: Duration::from_secs(60),
        },
    );
    assert!(
        !effects.iter().any(|effect| matches!(effect, Effect::ReadSummaries(_))),
        "nothing changed, nothing is read"
    );
    let ask = SummaryAsk {
        folder: login.clone(),
        last: Some(git_folder_changed(&login, None).stamp),
        read_at: None,
        is_forced: false,
    };
    let read = read_summaries(&[ask]);
    assert_eq!(
        read[0].result, None,
        "a change in the git folder that leaves HEAD, the index and the refs runs no git"
    );
}
