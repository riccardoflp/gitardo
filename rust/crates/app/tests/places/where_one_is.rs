//! The header that says where one is (`graph.where`): the repository, the worktree or the submodule, and the branch,
//! across the window above the side column, over clones made for each test.

use std::path::PathBuf;

use app::store::{Message, State};
use platform::accesskit::Role;
use render::ThemeName;
use testkit::Repository;
use ui::{InputEvent, Key, KeyEvent, Modifiers, NamedKey};
use views::Intent;
use views::page::Page;
use views::text::header;

use super::inspector::ask;
use super::side_column::{HEADER, column, git_in, header_button, header_nodes, in_header, menu_items, sync};
use super::support::{self, Window};
use super::tabs::{WIDE, clone_three, three_clones};
use super::{repository_with_history, showing};

#[test]
fn graph_where__the_header_goes_across_the_window_right_of_the_bar_of_views_above_the_side_column_with_no_island() {
    let (_kept, _clones, state) = three_clones();
    let mut window = Window::new(ThemeName::Dark, WIDE);
    let (_, tree) = window.draw(&state, &[]);
    let column = column(&tree);
    let repository = header_button(&tree, "api");
    let more = header_button(&tree, "More");
    assert!(
        (repository.x - column.x).abs() <= 12.0,
        "it starts where the side column does: {repository:?}"
    );
    #[allow(clippy::cast_precision_loss, reason = "the window is a few hundred pixels wide")]
    let width = WIDE.0 as f32;
    assert!(more.right() > width - 24.0, "it goes across the whole window: {more:?}");
    let pixels = window.frame.to_rgba();
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "inside the window"
    )]
    let (x, y) = ((repository.right() + 40.0) as usize, (HEADER * 0.5) as usize);
    let at = (y * WIDE.0 as usize + x) * 4;
    assert_eq!(
        pixels[at..at + 3],
        [0x0E, 0x0F, 0x12],
        "no border nor background of an island"
    );
}

#[test]
fn graph_where__with_the_settings_page_shown_the_header_says_the_repository_and_the_branch() {
    let (_kept, _clones, mut state) = three_clones();
    ask(&mut state, Intent::ShowPage(Page::Settings));
    let (_, tree) = Window::new(ThemeName::Dark, WIDE).draw(&state, &[]);
    let _ = header_button(&tree, "api");
    let _ = in_header(&tree, Role::Label, "main");
}

/// The three clones, `api` with a worktree `login` on `feature/login`, and a window showing them.
pub(super) fn api_with_login() -> ((Repository, Repository), [PathBuf; 3], PathBuf, State) {
    let ((origin, holder), clones) = clone_three();
    let login = holder.path().join("clones").join("login");
    git_in(
        &origin,
        &clones[0],
        &[
            "worktree",
            "add",
            "--quiet",
            "-b",
            "feature/login",
            login.to_str().unwrap(),
        ],
    );
    let mut state = super::window();
    let _ = support::run(
        &mut state,
        Message::Started {
            folders: clones.to_vec(),
        },
    );
    ((origin, holder), clones, login, state)
}

#[test]
fn graph_where__in_the_worktree_login_of_api_the_header_says_api_login_feature_login_and_in_api_api_main() {
    let (_kept, clones, login, mut state) = api_with_login();
    let mut window = Window::new(ThemeName::Dark, WIDE);
    let (_, tree) = window.draw(&state, &[]);
    let _ = header_button(&tree, "api");
    assert!(!header_nodes(&tree).iter().any(|node| node.contains("\"login\"")));
    let _ = in_header(&tree, Role::Label, "main");
    ask(&mut state, Intent::ShowInTab(0, login));
    let (_, tree) = window.draw(&state, &[]);
    let api = header_button(&tree, "api");
    let place = header_button(&tree, "login");
    let branch = in_header(&tree, Role::Label, "feature/login");
    assert!(api.x < place.x && place.x < branch.x, "in that order");
    let separator = header::WHERE_SEPARATOR.trim();
    assert!(header_nodes(&tree).iter().any(|node| node.contains(separator)));
    drop(clones);
}

#[test]
fn graph_where__a_click_on_api_lists_the_repositories_open_and_one_on_login_the_worktrees_of_api() {
    let (_kept, _clones, login, mut state) = api_with_login();
    ask(&mut state, Intent::ShowInTab(0, login));
    let mut window = Window::new(ThemeName::Dark, WIDE);
    let (_, tree) = window.draw(&state, &[]);
    let api = header_button(&tree, "api");
    let _ = super::pointer::click_at(&mut window, &state, super::pointer::middle(api), 1);
    let (_, tree) = window.draw(&state, &[]);
    assert_eq!(menu_items(&tree), ["api \u{203a} feature/login", "web", "docs"]);
    let _ = window.draw(
        &state,
        &[InputEvent::KeyPressed(KeyEvent::press(
            Key::Named(NamedKey::Escape),
            Modifiers::NONE,
        ))],
    );
    let (_, tree) = window.draw(&state, &[]);
    let place = header_button(&tree, "login");
    let _ = super::pointer::click_at(&mut window, &state, super::pointer::middle(place), 1);
    let (_, tree) = window.draw(&state, &[]);
    let items = menu_items(&tree);
    assert!(
        items.starts_with(&[String::from("api"), String::from("feature/login")]),
        "{items:?}"
    );
}

#[test]
fn graph_where__with_main_two_ahead_the_branch_in_the_header_has_no_2_and_sync_has_it() {
    let (mut repository, _remote) = repository_with_history();
    let _ahead = repository.commit("Show the drawer", &[("drawer.txt", "activity\n")]);
    let state = showing(repository.path().to_path_buf());
    let (_, tree) = Window::new(ThemeName::Dark, WIDE).draw(&state, &[]);
    let _ = in_header(&tree, Role::Label, "main");
    assert!(!header_nodes(&tree).iter().any(|node| node.contains("main \u{2191}")));
    assert_eq!(sync(&tree).0, "\u{2193}0  \u{2191}2   Sync");
}
