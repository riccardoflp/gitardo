//! `repo.choose-tabs`, `repo.open-tab` and `repo.close-tab`: which repositories a workspace opens in tabs, one more
//! opened from **+**, and tabs closed.

use std::path::{Path, PathBuf};

use model::contract::FeatureId;
use model::repo::Repository;
use views::Intent;
use views::repo::TabsPickerKind;

use super::{act, answer, answer_asking, folder, intent, open, started, timeline_folder, view_data, window};
use crate::store::feature::{Running, Said};
use crate::store::{Dialog, Found, JobKind, Message, State, reduce};

/// The folder `work` and the repositories it holds, by their names.
fn work_of(names: &[&str]) -> (PathBuf, Vec<PathBuf>) {
    let work = folder("work");
    let repositories = names.iter().map(|name| work.join(name)).collect();
    (work, repositories)
}

/// The folders of the tabs open, in their order.
fn tab_folders(state: &State) -> Vec<PathBuf> {
    state.tabs.open.iter().map(|tab| tab.folder.clone()).collect()
}

/// Opens a folder in a window with no workspace and leaves the picker of the tabs open.
fn open_asking(state: &mut State, work: &Path, found: &[(&Path, &[PathBuf])]) {
    let effects = reduce(state, Message::FolderChosen(Some(work.to_path_buf())));
    let _asked = answer_asking(state, effects, found);
}

/// A window showing `api`, `web` and `docs` of `work`, each in its tab, `api` shown.
fn three_tabs() -> (State, PathBuf, Vec<PathBuf>) {
    let mut state = window();
    let (work, repositories) = work_of(&["api", "web", "docs"]);
    let _opened = open(&mut state, &work, &[(&work, &repositories)]);
    (state, work, repositories)
}

/// A new window, with what the one before kept between runs, opening the same folder.
fn opened_again(state: &State, work: &Path, found: &[(&Path, &[PathBuf])]) -> State {
    let mut next_time = window();
    next_time.persisted = state.persisted.clone();
    open_asking(&mut next_time, work, found);
    next_time
}

#[test]
fn repo_choose_tabs__a_folder_of_eight_repositories_asks_with_all_offered_and_opens_eight_tabs_the_first_shown() {
    let mut state = window();
    let (work, repositories) = work_of(&["api", "web", "docs", "app", "lib", "site", "cli", "tools"]);
    let found = [(work.as_path(), &repositories[..])];
    open_asking(&mut state, &work, &found);
    let Some(Dialog::ChooseTabs(choosing)) = &state.dialog else {
        panic!("the repositories to open are asked");
    };
    assert_eq!(choosing.offered, repositories);
    let data = view_data(&state);
    let picker = data.window_state(&state).tabs_picker.unwrap();
    assert_eq!(picker.kind, TabsPickerKind::Choose);
    assert_eq!(picker.repositories.len(), 8);
    let _opened = act(&mut state, Intent::TabsChosen(Some((0..8).collect())), &found);
    assert_eq!(state.dialog, None);
    assert_eq!(tab_folders(&state), repositories);
    assert_eq!(timeline_folder(&state), Some(repositories[0].clone()));
}

#[test]
fn repo_choose_tabs__api_and_web_alone_checked_open_those_two_tabs() {
    let mut state = window();
    let (work, repositories) = work_of(&["api", "web", "docs"]);
    let found = [(work.as_path(), &repositories[..])];
    open_asking(&mut state, &work, &found);
    let _opened = act(&mut state, Intent::TabsChosen(Some(vec![0, 1])), &found);
    assert_eq!(tab_folders(&state), repositories[..2]);
    assert_eq!(timeline_folder(&state), Some(repositories[0].clone()));
}

#[test]
fn repo_choose_tabs__a_repository_with_two_submodules_opens_one_tab_and_asks_nothing() {
    let mut state = window();
    let app = folder("app");
    let effects = reduce(&mut state, Message::FolderChosen(Some(app.clone())));
    let (job, _) = started(&effects)
        .into_iter()
        .find(|(_, kind)| matches!(kind, JobKind::FindRepositories { .. }))
        .unwrap();
    let submodule = |name: &str| Repository {
        folder: app.join(name),
        superproject: Some(app.clone()),
    };
    let found = Found::Repositories(vec![
        Repository {
            folder: app.clone(),
            superproject: None,
        },
        submodule("ui"),
        submodule("vendor"),
    ]);
    let effects = reduce(&mut state, Message::RepositoriesFound { job, found });
    assert_eq!(state.dialog, None, "nothing is asked");
    let _read = answer(&mut state, effects, &[]);
    assert_eq!(tab_folders(&state), std::slice::from_ref(&app));
    assert_eq!(timeline_folder(&state), Some(app));
}

#[test]
fn repo_choose_tabs__escape_opens_nothing_and_leaves_the_window_as_it_was() {
    let mut state = window();
    let (work, repositories) = work_of(&["api", "web"]);
    let found = [(work.as_path(), &repositories[..])];
    open_asking(&mut state, &work, &found);
    let _closed = intent(&mut state, Intent::TabsChosen(None));
    assert_eq!(state.dialog, None);
    assert!(state.workspace.is_none(), "the Welcome page stays");
    assert_eq!(state.persisted.recent, [], "nothing was opened");

    let other = folder("other");
    let found = [
        (work.as_path(), &repositories[..]),
        (other.as_path(), std::slice::from_ref(&other)),
    ];
    let _opened = open(&mut state, &other, &found);
    let _asked = reduce(&mut state, Message::FolderChosen(Some(work.clone())));
    let effects = intent(&mut state, Intent::AnswerOpenFolder(views::FolderAnswer::Replace));
    let _found = answer_asking(&mut state, effects, &found);
    assert!(matches!(state.dialog, Some(Dialog::ChooseTabs(_))));
    let _closed = intent(&mut state, Intent::TabsChosen(None));
    assert_eq!(state.workspace.as_ref().unwrap().folders, std::slice::from_ref(&other));
    assert_eq!(tab_folders(&state), std::slice::from_ref(&other));
    assert_eq!(timeline_folder(&state), Some(other));
}

#[test]
fn repo_choose_tabs__a_workspace_opened_before_opens_the_tabs_it_had_without_asking() {
    let (mut state, work, repositories) = three_tabs();
    let _closed = intent(&mut state, Intent::CloseTab(1));
    let other = folder("other");
    let found = [
        (work.as_path(), &repositories[..]),
        (other.as_path(), std::slice::from_ref(&other)),
    ];
    let _asked = reduce(&mut state, Message::FolderChosen(Some(other.clone())));
    let _replaced = act(
        &mut state,
        Intent::AnswerOpenFolder(views::FolderAnswer::Replace),
        &found,
    );
    assert_eq!(state.persisted.recent[1].folders, std::slice::from_ref(&work));
    let effects = intent(&mut state, Intent::OpenRecent(1));
    let _found = answer_asking(&mut state, effects, &found);
    assert_eq!(state.dialog, None, "nothing is asked");
    assert_eq!(tab_folders(&state), [repositories[0].clone(), repositories[2].clone()]);
}

#[test]
fn repo_open_tab__with_only_api_open_plus_lists_web_and_docs_and_web_opens_after_api_and_is_shown() {
    let mut state = window();
    let (work, repositories) = work_of(&["api", "web", "docs"]);
    let found = [(work.as_path(), &repositories[..])];
    open_asking(&mut state, &work, &found);
    let _opened = act(&mut state, Intent::TabsChosen(Some(vec![0])), &found);
    let data = view_data(&state);
    let island = data.window_state(&state).tabs.unwrap();
    let offered: Vec<PathBuf> = island.not_open.iter().map(|entry| entry.folder.clone()).collect();
    assert_eq!(offered, repositories[1..]);
    let _web = act(&mut state, Intent::OpenTab(repositories[1].clone()), &found);
    assert_eq!(tab_folders(&state), repositories[..2]);
    assert_eq!(timeline_folder(&state), Some(repositories[1].clone()));
}

#[test]
fn repo_open_tab__a_repository_opens_after_the_tab_shown() {
    let mut state = window();
    let (work, repositories) = work_of(&["api", "web", "docs"]);
    let found = [(work.as_path(), &repositories[..])];
    open_asking(&mut state, &work, &found);
    let _opened = act(&mut state, Intent::TabsChosen(Some(vec![0, 2])), &found);
    let _web = act(&mut state, Intent::OpenTab(repositories[1].clone()), &found);
    assert_eq!(tab_folders(&state), repositories);
}

#[test]
fn repo_open_tab__a_repository_open_already_has_its_tab_shown_and_no_other_opens() {
    let (mut state, work, repositories) = three_tabs();
    let _docs = act(
        &mut state,
        Intent::OpenTab(repositories[2].clone()),
        &[(&work, &repositories)],
    );
    assert_eq!(tab_folders(&state), repositories);
    assert_eq!(timeline_folder(&state), Some(repositories[2].clone()));
}

#[test]
fn repo_open_tab__open_repository_in_a_tab_asks_among_the_repositories_with_no_tab() {
    let mut state = window();
    let (work, repositories) = work_of(&["api", "web", "docs"]);
    let found = [(work.as_path(), &repositories[..])];
    open_asking(&mut state, &work, &found);
    let _opened = act(&mut state, Intent::TabsChosen(Some(vec![0])), &found);
    let _asked = intent(&mut state, Intent::PickTabToOpen);
    let data = view_data(&state);
    let picker = data.window_state(&state).tabs_picker.unwrap();
    assert_eq!(picker.kind, TabsPickerKind::Open);
    let offered: Vec<PathBuf> = picker.repositories.iter().map(|entry| entry.folder.clone()).collect();
    assert_eq!(offered, repositories[1..]);
    let _docs = act(&mut state, Intent::OpenTab(repositories[2].clone()), &found);
    assert_eq!(state.dialog, None);
    assert_eq!(tab_folders(&state), [repositories[0].clone(), repositories[2].clone()]);
}

#[test]
fn repo_open_tab__the_tabs_open_are_the_same_once_the_window_opens_again() {
    let mut state = window();
    let (work, repositories) = work_of(&["api", "web", "docs"]);
    let found = [(work.as_path(), &repositories[..])];
    open_asking(&mut state, &work, &found);
    let _opened = act(&mut state, Intent::TabsChosen(Some(vec![0])), &found);
    let _web = act(&mut state, Intent::OpenTab(repositories[1].clone()), &found);
    let next_time = opened_again(&state, &work, &found);
    assert_eq!(next_time.dialog, None);
    assert_eq!(tab_folders(&next_time), repositories[..2]);
}

#[test]
fn repo_close_tab__closing_web_between_api_and_docs_shows_docs() {
    let (mut state, work, repositories) = three_tabs();
    let found = [(work.as_path(), &repositories[..])];
    let _web = act(&mut state, Intent::SwitchTab(1), &found);
    let _closed = act(&mut state, Intent::CloseTab(1), &found);
    assert_eq!(tab_folders(&state), [repositories[0].clone(), repositories[2].clone()]);
    assert_eq!(timeline_folder(&state), Some(repositories[2].clone()));
}

#[test]
fn repo_close_tab__the_tab_on_the_right_end_gives_way_to_its_left_and_the_only_tab_to_the_welcome_page() {
    let (mut state, work, repositories) = three_tabs();
    let found = [(work.as_path(), &repositories[..])];
    let _docs = act(&mut state, Intent::SwitchTab(2), &found);
    let _closed = act(&mut state, Intent::CloseTab(2), &found);
    assert_eq!(timeline_folder(&state), Some(repositories[1].clone()));
    let _closed = act(&mut state, Intent::CloseTabShown, &found);
    assert_eq!(timeline_folder(&state), Some(repositories[0].clone()));
    let _closed = act(&mut state, Intent::CloseTab(0), &found);
    assert!(state.workspace.is_none(), "the Welcome page is shown");
    assert_eq!(state.tabs.open, []);
}

#[test]
fn repo_close_tab__the_tab_where_an_operation_runs_is_refused_and_another_tab_is_not() {
    let (mut state, _work, repositories) = three_tabs();
    state.is_operation_running = true;
    state.operations.running = Some(Running {
        feature: FeatureId("branch.create"),
        asking: None,
        title: Some(String::from("Create branch feature")),
    });
    assert_eq!(intent(&mut state, Intent::CloseTab(0)), []);
    assert_eq!(tab_folders(&state), repositories);
    assert_eq!(
        state.operations.said,
        Some(Said::error(String::from(
            "Gitardo: wait for Create branch feature to end before closing this tab."
        )))
    );
    let _closed = intent(&mut state, Intent::CloseTab(2));
    assert_eq!(tab_folders(&state), repositories[..2]);
}

#[test]
fn repo_close_tab__a_tab_closed_is_not_there_once_the_window_opens_again() {
    let (mut state, work, repositories) = three_tabs();
    let found = [(work.as_path(), &repositories[..])];
    let _closed = act(&mut state, Intent::CloseTab(1), &found);
    let next_time = opened_again(&state, &work, &found);
    assert_eq!(
        tab_folders(&next_time),
        [repositories[0].clone(), repositories[2].clone()]
    );
}

#[test]
fn repo_close_tab__close_other_tabs_on_web_leaves_web_alone() {
    let (mut state, work, repositories) = three_tabs();
    let found = [(work.as_path(), &repositories[..])];
    let _closed = act(&mut state, Intent::CloseOtherTabs(1), &found);
    assert_eq!(tab_folders(&state), [repositories[1].clone()]);
    assert_eq!(timeline_folder(&state), Some(repositories[1].clone()));
}
