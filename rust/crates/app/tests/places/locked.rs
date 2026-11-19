//! Locked worktrees in the window: the lock in the Worktrees list and on the row in **Repositories**, and **Lock…**
//! and **Unlock** in the menu of a worktree (`worktree.locked`, `worktree.lock`, `worktree.unlock`).

use std::path::PathBuf;

use app::store::{Message, State};
use model::contract::{Answer, FeatureId, Target};
use render::ThemeName;
use testkit::Repository;
use views::feature::FeatureIntent;
use views::text::worktree as words;
use views::{Intent, RailIntent, RailList};

use super::changes_menu::menu_of;
use super::inspector::ask;
use super::showing;
use super::support::{OperationsDriver, Window};

/// The size of the window, tall enough for the Worktrees list under the branches.
const WIDE: (u32, u32) = (1000, 760);

/// A repository with a commit, and the worktree `on-usb` of `feature/usb` in the folder of the worktrees beside it,
/// locked for `reason` when one is given.
fn with_worktree(reason: Option<&str>) -> (Repository, PathBuf) {
    let mut repository = Repository::new();
    let _ = repository.commit("Add the readme", &[("README.md", "Gitardo\n")]);
    let folder = repository.path().with_file_name("repository.worktrees").join("on-usb");
    let _ = repository.git(&["worktree", "add", "-b", "feature/usb", &folder.to_string_lossy()]);
    if let Some(reason) = reason {
        let _ = repository.git(&["worktree", "lock", "--reason", reason, &folder.to_string_lossy()]);
    }
    (repository, folder)
}

/// The window on the repository, with the Worktrees list open.
fn with_list_open(repository: &Repository) -> State {
    let mut state = showing(repository.path().to_path_buf());
    ask(&mut state, Intent::Rail(RailIntent::Toggle(RailList::Worktrees)));
    state
}

/// Runs a feature on the worktree with these answers to its questions in turn, and waits until it ended.
fn run_answering(state: &mut State, (feature, folder): (&'static str, &PathBuf), answers: &[Answer]) {
    let driver = OperationsDriver::new();
    let target = Target::Worktree(folder.clone());
    driver.intent(state, Intent::Feature(FeatureIntent::Run(FeatureId(feature), target)));
    let mut answers = answers.iter();
    loop {
        match driver.next(state) {
            Message::FeatureAsks(_) => {
                let answer = answers.next().cloned();
                driver.intent(state, Intent::Feature(FeatureIntent::Answer(answer)));
            }
            Message::FeatureConfirms(_) => driver.intent(state, Intent::Feature(FeatureIntent::Confirm(true))),
            Message::FeatureEnded { .. } => return,
            _ => {}
        }
    }
}

#[test]
fn worktree_lock__the_menu_of_a_worktree_has_lock_which_locks_it_with_the_reason_typed() {
    let (repository, folder) = with_worktree(None);
    let mut state = with_list_open(&repository);
    let mut window = Window::new(ThemeName::Dark, WIDE);
    let items = menu_of(&mut window, &state, "on-usb");
    assert!(items.iter().any(|item| item == words::LOCK), "{items:?}");
    run_answering(
        &mut state,
        ("worktree.lock", &folder),
        &[Answer::Text("On the USB drive".to_owned())],
    );
    let listed = repository.git(&["worktree", "list", "--porcelain"]);
    assert!(listed.contains("locked On the USB drive"), "{listed}");
    let items = menu_of(&mut window, &state, "on-usb");
    assert!(
        items.iter().any(|item| item == words::UNLOCK),
        "the list was read again: {items:?}"
    );
}

#[test]
fn worktree_unlock__unlock_takes_the_lock_away_and_the_menu_offers_lock_again() {
    let (repository, folder) = with_worktree(Some("On the USB drive"));
    let mut state = with_list_open(&repository);
    run_answering(&mut state, ("worktree.unlock", &folder), &[]);
    assert!(!repository.git(&["worktree", "list", "--porcelain"]).contains("locked"));
    let mut window = Window::new(ThemeName::Dark, WIDE);
    let items = menu_of(&mut window, &state, "on-usb");
    assert!(items.iter().any(|item| item == words::LOCK), "{items:?}");
}

#[test]
fn worktree_locked__the_worktree_list_matches_its_snapshots_with_a_lock_in_each_theme() {
    let (repository, _folder) = with_worktree(Some("On the USB drive"));
    for (theme, name) in [(ThemeName::Dark, "dark"), (ThemeName::Light, "light")] {
        let state = with_list_open(&repository);
        let mut window = Window::new(theme, WIDE);
        let _ = window.draw(&state, &[]);
        window.assert_snapshot(&format!("worktrees-locked-{name}"));
    }
}

#[test]
fn worktree_locked__the_row_in_repositories_matches_its_snapshots_with_a_lock_in_each_theme() {
    let repositories = super::group_tabs::made(false);
    let login = repositories.login.to_string_lossy().into_owned();
    let api = repositories.api.to_string_lossy().into_owned();
    // git runs from a throwaway repository of its own, in the folder of `api`.
    let helper = Repository::new();
    let _ = helper.git(&["-C", &api, "worktree", "lock", "--reason", "On the USB drive", &login]);
    for (theme, name) in [(ThemeName::Dark, "dark"), (ThemeName::Light, "light")] {
        let state = super::group_tabs::window(&repositories);
        let mut window = Window::new(theme, super::tabs::WIDE);
        let _ = window.draw(&state, &[]);
        window.assert_snapshot(&format!("tabs-group-locked-{name}"));
    }
}
