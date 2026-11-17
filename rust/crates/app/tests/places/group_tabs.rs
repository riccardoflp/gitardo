//! Worktrees and submodules in the tab of their repository (`repo.group-tabs`), over a clone `api` with a worktree
//! `api-login` on `feature/login` and a submodule `lib/core`, and a clone `web`, made for each test.

use std::path::{Path, PathBuf};

use app::persisted::same_folder;
use app::store::{Message, State, ViewData};
use platform::accesskit::Role;
use render::ThemeName;
use testkit::Repository;
use views::Intent;
use views::graph::{GraphIntent, Selected};
use views::repo::TabEntry;

use super::inspector::{ask, row_id};
use super::support::{self, Window, find};
use super::tabs::WIDE;

/// The repositories of a test, kept alive while it runs.
struct Repositories {
    _kept: (Repository, Repository, Repository),
    api: PathBuf,
    login: PathBuf,
    web: PathBuf,
}

/// `api` with its worktree `api-login` on `feature/login` and its submodule `lib/core`, and `web`, all in a folder of
/// their own.
fn repositories() -> Repositories {
    made(true)
}

/// `api` with its worktree, with its submodule or not, and `web`. The file of the submodules names the folder of its
/// repository, which changes at every run, and so the commits after it: a snapshot is drawn without it.
fn made(with_submodule: bool) -> Repositories {
    let mut origin = Repository::new();
    let _ = origin.commit("Add the readme", &[("README.md", "Gitardo\n")]);
    let _ = origin.commit("Read the settings at startup", &[("settings.txt", "theme\n")]);
    let mut library = Repository::new();
    let _ = library.commit("Add the core", &[("core.rs", "1\n")]);
    let holder = Repository::new();
    let api = holder.path().join("clones").join("api");
    let login = holder.path().join("clones").join("api-login");
    let web = holder.path().join("clones").join("web");
    for clone in [&api, &web] {
        let _ = origin.git(&["clone", "--quiet", ".", clone.to_str().unwrap()]);
        let clone = clone.to_str().unwrap();
        let _ = origin.git(&["-C", clone, "config", "user.name", "Test Author"]);
        let _ = origin.git(&["-C", clone, "config", "user.email", "author@example.com"]);
    }
    let in_api = |arguments: &[&str]| {
        let mut all = vec!["-C", api.to_str().unwrap()];
        all.extend_from_slice(arguments);
        let _ = origin.git(&all);
    };
    if with_submodule {
        in_api(&[
            "-c",
            "protocol.file.allow=always",
            "submodule",
            "add",
            "--quiet",
            library.path().to_str().unwrap(),
            "lib/core",
        ]);
        in_api(&["commit", "--quiet", "-m", "Add the core library"]);
    }
    in_api(&[
        "worktree",
        "add",
        "--quiet",
        "-b",
        "feature/login",
        login.to_str().unwrap(),
    ]);
    Repositories {
        _kept: (origin, library, holder),
        api,
        login,
        web,
    }
}

/// A window on the folders `api`, `api-login` and `web`, every tab offered opened.
fn window(repositories: &Repositories) -> State {
    let mut state = super::window();
    let folders = vec![
        repositories.api.clone(),
        repositories.login.clone(),
        repositories.web.clone(),
    ];
    let _ = support::run(&mut state, Message::Started { folders });
    state
}

/// The tabs as the island is given them.
fn tabs(state: &State) -> Vec<TabEntry> {
    let data = ViewData::new(state, Vec::new(), 0);
    data.window_state(state)
        .side
        .map_or_default(|side| side.tabs.tabs.to_vec())
}

/// The labels of the **▾** of a tab.
fn members(tab: &TabEntry) -> Vec<&str> {
    tab.members.iter().map(|member| member.label.as_str()).collect()
}

/// Whether the repository shown is the one of a folder.
fn shows(state: &State, folder: &Path) -> bool {
    state.shown_folder().is_some_and(|shown| same_folder(shown, folder))
}

#[test]
fn repo_group_tabs__api_and_its_worktree_both_folders_of_the_workspace_are_one_tab_whose_menu_lists_both() {
    let repositories = repositories();
    let state = window(&repositories);
    let tabs = tabs(&state);
    let names: Vec<&str> = tabs.iter().map(|tab| tab.name.as_str()).collect();
    assert_eq!(names, ["api", "web"], "api-login has no tab of its own");
    assert_eq!(members(&tabs[0])[..2], ["api", "feature/login"]);
    assert!(tabs[0].members[0].is_shown, "the tab shows api, which has a check");
    let (_, tree) = Window::new(ThemeName::Dark, WIDE).draw(&state, &[]);
    let _ = find(&tree, Role::Button, views::text::repo::TAB_MEMBERS);
}

#[test]
fn repo_group_tabs__a_submodule_of_api_is_in_its_menu_and_has_no_tab_of_its_own() {
    let repositories = repositories();
    let state = window(&repositories);
    let tabs = tabs(&state);
    assert_eq!(members(&tabs[0]), ["api", "feature/login", "lib/core"]);
    assert!(!tabs.iter().any(|tab| tab.name.contains("core")));
}

#[test]
fn repo_group_tabs__choosing_the_worktree_names_the_tab_after_it_and_choosing_api_again_shows_api_as_it_was_left() {
    let repositories = repositories();
    let mut state = window(&repositories);
    let commit = row_id(&state, "Read the settings");
    ask(
        &mut state,
        Intent::Graph(GraphIntent::Select(Selected::Commit(commit.clone()))),
    );
    ask(&mut state, Intent::ShowInTab(0, repositories.login.clone()));
    assert!(shows(&state, &repositories.login));
    assert_eq!(tabs(&state)[0].name, "api \u{203a} feature/login");
    assert_eq!(state.timeline.selected, None, "the worktree has nothing selected");
    ask(&mut state, Intent::ShowInTab(0, repositories.api.clone()));
    assert!(shows(&state, &repositories.api));
    assert_eq!(tabs(&state)[0].name, "api");
    assert_eq!(state.timeline.selected, Some(Selected::Commit(commit)));
}

#[test]
fn repo_group_tabs__a_submodule_chosen_names_the_tab_after_its_path() {
    let repositories = repositories();
    let mut state = window(&repositories);
    let core = tabs(&state)[0].members[2].folder.clone();
    ask(&mut state, Intent::ShowInTab(0, core.clone()));
    assert!(shows(&state, &core));
    assert_eq!(tabs(&state)[0].name, "api \u{203a} lib/core");
}

#[test]
fn repo_group_tabs__two_repositories_that_share_nothing_are_two_tabs_and_one_with_no_worktree_has_no_menu() {
    let repositories = repositories();
    let state = window(&repositories);
    let tabs = tabs(&state);
    assert_eq!(tabs.len(), 2);
    assert_eq!(tabs[1].members, [], "web has no other worktree and no submodule");
}

#[test]
fn repo_open_tab__opening_a_worktree_of_api_that_is_a_folder_of_the_workspace_shows_it_in_the_tab_of_api() {
    let repositories = repositories();
    let mut state = window(&repositories);
    ask(&mut state, Intent::SwitchTab(1));
    ask(&mut state, Intent::OpenTab(repositories.login.clone()));
    assert_eq!(tabs(&state).len(), 2, "no tab opens");
    assert_eq!(state.tabs.shown, Some(0));
    assert!(shows(&state, &repositories.login));
}

#[test]
fn repo_close_tab__a_page_not_saved_in_a_worktree_the_tab_does_not_show_asks_with_that_worktree_shown() {
    let repositories = repositories();
    let mut state = window(&repositories);
    let driver = super::support::OperationsDriver::new();
    driver.intent(&mut state, Intent::ShowInTab(0, repositories.login.clone()));
    let id = super::close_tabs::edit_readme(&driver, &mut state);
    driver.intent(&mut state, Intent::ShowInTab(0, repositories.api.clone()));
    assert!(shows(&state, &repositories.api));
    driver.intent(&mut state, Intent::CloseTab(0));
    assert!(
        shows(&state, &repositories.login),
        "the worktree with the page is shown"
    );
    assert_eq!(state.file_pages.closing, Some(id));
    super::close_tabs::answer(&driver, &mut state, views::file_view::CloseAnswer::DontSave);
    assert_eq!(tabs(&state).len(), 1);
    assert!(shows(&state, &repositories.web));
}

#[test]
fn the_tab_of_a_repository_with_a_worktree_shown_is_drawn_with_its_menu() {
    let repositories = made(false);
    let mut state = window(&repositories);
    ask(&mut state, Intent::ShowInTab(0, repositories.login.clone()));
    let mut window = Window::new(ThemeName::Dark, WIDE);
    let _ = window.draw(&state, &[]);
    window.assert_snapshot("tabs-group-dark");
}
