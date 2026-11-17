//! Worktrees and submodules in the tab of their repository (`repo.group-tabs`), over a clone `api` with a worktree
//! `api-login` on `feature/login` and a submodule `lib/core`, and a clone `web`, made for each test.

use std::path::{Path, PathBuf};

use app::persisted::same_folder;
use app::store::{Message, State, ViewData};
use platform::accesskit::{Node, Role, TreeUpdate};
use render::{Rect, ThemeName};
use testkit::Repository;
use ui::{InputEvent, Key, KeyEvent, Modifiers, NamedKey};
use views::Intent;
use views::graph::{GraphIntent, Selected, SideSection};
use views::repo::TabEntry;
use views::text::repo::TAB_MEMBERS;

use super::inspector::{ask, row_id};
use super::pointer::{apply, click_at, middle, rect};
use super::side_column::column;
use super::support::{self, Window};
use super::tabs::WIDE;

/// The repositories of a test, kept alive while it runs.
pub(super) struct Repositories {
    _kept: (Repository, Repository, Repository),
    pub(super) api: PathBuf,
    pub(super) login: PathBuf,
    web: PathBuf,
}

/// `api` with its worktree `api-login` on `feature/login` and its submodule `lib/core`, and `web`, all in a folder of
/// their own.
fn repositories() -> Repositories {
    made(true)
}

/// `api` with its worktree, with its submodule or not, and `web`. The file of the submodules names the folder of its
/// repository, which changes at every run, and so the commits after it: a snapshot is drawn without it.
pub(super) fn made(with_submodule: bool) -> Repositories {
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
pub(super) fn window(repositories: &Repositories) -> State {
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

/// The names of the rows of a tab's repository, its worktrees and its submodules.
fn members(tab: &TabEntry) -> Vec<&str> {
    tab.members.iter().map(|member| member.name.as_str()).collect()
}

/// The nodes of the side column of a role and a name, from the top.
fn all_in_column<'tree>(tree: &'tree TreeUpdate, role: Role, name: &str) -> Vec<(Rect, &'tree Node)> {
    let column = column(tree);
    let mut found: Vec<(Rect, &Node)> = tree
        .nodes
        .iter()
        .filter(|(_, node)| node.role() == role && node.label() == Some(name))
        .filter_map(|(_, node)| Some((rect(node.bounds()?), node)))
        .filter(|(at, _)| at.y >= column.y && at.x >= column.x && at.right() <= column.right() + 0.5)
        .collect();
    found.sort_by(|a, b| a.0.y.total_cmp(&b.0.y));
    found
}

/// A node of the side column, by its role and its name, which the test fails without.
fn in_column<'tree>(tree: &'tree TreeUpdate, role: Role, name: &str) -> (Rect, &'tree Node) {
    all_in_column(tree, role, name)
        .into_iter()
        .next()
        .unwrap_or_else(|| panic!("no {name} in the side column"))
}

/// The row of a name in **Repositories**.
fn row(tree: &TreeUpdate, name: &str) -> Rect {
    in_column(tree, Role::Button, name).0
}

/// Where the text of a name drawn in a row starts.
fn text_x(tree: &TreeUpdate, name: &str, within: Rect) -> f32 {
    tree.nodes
        .iter()
        .filter(|(_, node)| node.role() == Role::Label)
        .filter(|(_, node)| node.value() == Some(name) || node.label() == Some(name))
        .filter_map(|(_, node)| node.bounds().map(rect))
        .find(|found| found.y >= within.y && found.bottom() <= within.bottom() + 0.5)
        .unwrap_or_else(|| panic!("no text {name}"))
        .x
}

/// Clicks the row of a name in **Repositories**, and applies what it asked.
fn click_row(window: &mut Window, state: &mut State, name: &str) {
    let (_, tree) = window.draw(state, &[]);
    let intents = click_at(window, state, middle(row(&tree, name)), 1);
    apply(state, intents);
}

/// A key pressed with no modifier.
fn press(key: NamedKey) -> InputEvent {
    InputEvent::KeyPressed(KeyEvent::press(Key::Named(key), Modifiers::NONE))
}

/// Whether the repository shown is the one of a folder.
fn shows(state: &State, folder: &Path) -> bool {
    state.shown_folder().is_some_and(|shown| same_folder(shown, folder))
}

#[test]
fn repo_group_tabs__api_and_its_worktree_both_folders_of_the_workspace_are_one_row_with_the_worktree_under_it() {
    let repositories = repositories();
    let state = window(&repositories);
    let tabs = tabs(&state);
    let names: Vec<&str> = tabs.iter().map(|tab| tab.name.as_str()).collect();
    assert_eq!(names, ["api", "web"], "api-login has no row of its own at the top");
    assert_eq!(members(&tabs[0])[..2], ["api", "api-login"]);
    assert!(tabs[0].members[0].is_shown, "the row shows api");
    let (_, tree) = Window::new(ThemeName::Dark, WIDE).draw(&state, &[]);
    let (api, login) = (row(&tree, "api"), row(&tree, "api-login"));
    assert!(login.y >= api.bottom() - 0.5, "under api: {api:?} {login:?}");
    let (name_of_api, name_of_login) = (text_x(&tree, "api", api), text_x(&tree, "api-login", login));
    assert!(name_of_login > name_of_api, "indented: {name_of_api} {name_of_login}");
    let _ = super::pointer::said_at(&tree, "feature/login", login);
}

#[test]
fn repo_group_tabs__a_submodule_of_api_is_a_row_under_it_and_has_no_row_of_its_own() {
    let repositories = repositories();
    let state = window(&repositories);
    let tabs = tabs(&state);
    assert_eq!(members(&tabs[0]), ["api", "api-login", "lib/core"]);
    assert!(!tabs.iter().any(|tab| tab.name.contains("core")));
    let mut state = state;
    ask(
        &mut state,
        Intent::Graph(GraphIntent::FoldSection(SideSection::Branches)),
    );
    let (_, tree) = Window::new(ThemeName::Dark, WIDE).draw(&state, &[]);
    assert!(row(&tree, "lib/core").y > row(&tree, "api-login").y);
    assert!(
        row(&tree, "lib/core").bottom() <= row(&tree, "web").y + 0.5,
        "before web"
    );
}

#[test]
fn repo_group_tabs__clicking_the_worktree_marks_its_row_and_names_the_tab_after_it_and_clicking_api_shows_api_as_left()
{
    let repositories = repositories();
    let mut state = window(&repositories);
    let commit = row_id(&state, "Read the settings");
    ask(
        &mut state,
        Intent::Graph(GraphIntent::Select(Selected::Commit(commit.clone()))),
    );
    let mut window = Window::new(ThemeName::Dark, WIDE);
    click_row(&mut window, &mut state, "api-login");
    assert!(shows(&state, &repositories.login));
    assert_eq!(tabs(&state)[0].name, "api \u{203a} feature/login");
    let (_, tree) = window.draw(&state, &[]);
    let (_, login) = in_column(&tree, Role::Button, "api-login");
    let (_, api) = in_column(&tree, Role::Button, "api");
    assert_eq!((login.is_selected(), api.is_selected()), (Some(true), Some(false)));
    assert_eq!(state.timeline.selected, None, "the worktree has nothing selected");
    click_row(&mut window, &mut state, "api");
    assert!(shows(&state, &repositories.api));
    assert_eq!(tabs(&state)[0].name, "api");
    assert_eq!(state.timeline.selected, Some(Selected::Commit(commit)));
}

#[test]
fn repo_group_tabs__a_submodule_chosen_names_the_tab_after_its_path() {
    let repositories = repositories();
    let mut state = window(&repositories);
    let mut window = Window::new(ThemeName::Dark, WIDE);
    click_row(&mut window, &mut state, "lib/core");
    let core = tabs(&state)[0].members[2].folder.clone();
    assert!(shows(&state, &core));
    assert_eq!(tabs(&state)[0].name, "api \u{203a} lib/core");
}

#[test]
fn repo_group_tabs__two_repositories_that_share_nothing_are_two_rows_and_one_with_no_worktree_has_no_chevron() {
    let repositories = repositories();
    let state = window(&repositories);
    let tabs = tabs(&state);
    assert_eq!(tabs.len(), 2);
    assert_eq!(tabs[1].members, [], "web has no other worktree and no submodule");
    let (_, tree) = Window::new(ThemeName::Dark, WIDE).draw(&state, &[]);
    assert_eq!(
        all_in_column(&tree, Role::Button, TAB_MEMBERS).len(),
        1,
        "the chevron of api alone"
    );
}

#[test]
fn repo_group_tabs__the_chevron_folds_the_rows_under_api_which_stay_folded_and_api_then_says_the_worktree_shown() {
    let repositories = repositories();
    let mut state = window(&repositories);
    let mut window = Window::new(ThemeName::Dark, WIDE);
    let (_, tree) = window.draw(&state, &[]);
    let (at, chevron) = in_column(&tree, Role::Button, TAB_MEMBERS);
    assert_eq!(chevron.is_expanded(), Some(true));
    let intents = click_at(&mut window, &state, middle(at), 1);
    assert_eq!(intents, [Intent::FoldTab(0)]);
    apply(&mut state, intents);
    let mut again = super::window();
    again.persisted = state.persisted.clone();
    let folders = vec![
        repositories.api.clone(),
        repositories.login.clone(),
        repositories.web.clone(),
    ];
    let _ = support::run(&mut again, Message::Started { folders });
    ask(&mut again, Intent::ShowInTab(0, repositories.login.clone()));
    let (_, tree) = Window::new(ThemeName::Dark, WIDE).draw(&again, &[]);
    assert!(
        all_in_column(&tree, Role::Button, "api-login").is_empty(),
        "folded after the window opens again"
    );
    assert_eq!(in_column(&tree, Role::Button, TAB_MEMBERS).1.is_expanded(), Some(false));
    let _ = row(&tree, "api \u{203a} feature/login");
}

#[test]
fn repo_group_tabs__enter_on_the_chevron_folds_it_and_the_screen_reader_is_told_each_row_and_the_one_shown() {
    let repositories = repositories();
    let mut state = window(&repositories);
    ask(
        &mut state,
        Intent::Graph(GraphIntent::FoldSection(SideSection::Branches)),
    );
    let mut window = Window::new(ThemeName::Dark, WIDE);
    let (_, tree) = window.draw(&state, &[]);
    for name in ["api", "api-login", "lib/core", "web"] {
        let (_, node) = in_column(&tree, Role::Button, name);
        assert_eq!(node.is_selected(), Some(name == "api"), "{name}");
    }
    let (at, _) = in_column(&tree, Role::Button, TAB_MEMBERS);
    let intents = click_at(&mut window, &state, middle(at), 1);
    apply(&mut state, intents);
    let (intents, tree) = window.draw(&state, &[press(NamedKey::Enter)]);
    assert_eq!(intents, [Intent::FoldTab(0)], "the chevron kept the keyboard");
    let focused = tree
        .nodes
        .iter()
        .find(|(id, _)| *id == tree.focus)
        .and_then(|(_, node)| node.label());
    assert_eq!(focused, Some(TAB_MEMBERS));
    apply(&mut state, intents);
    let (_, tree) = window.draw(&state, &[]);
    assert_eq!(
        in_column(&tree, Role::Button, TAB_MEMBERS).1.is_expanded(),
        Some(true),
        "unfolded again"
    );
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
fn the_tab_of_a_repository_with_a_worktree_shown_is_drawn_with_its_rows() {
    let repositories = made(false);
    let mut state = window(&repositories);
    ask(&mut state, Intent::ShowInTab(0, repositories.login.clone()));
    let mut window = Window::new(ThemeName::Dark, WIDE);
    let _ = window.draw(&state, &[]);
    window.assert_snapshot("tabs-group-dark");
}
