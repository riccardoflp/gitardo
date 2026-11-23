//! The upkeep of a repository from the menu of its row in **Repositories** (`upkeep.clean-up`,
//! `upkeep.start-background`, `upkeep.stop-background`). Nothing is run: `git maintenance start` would schedule git in
//! the system's scheduler, so the tests look at the menu and at what it asks for.

use model::contract::{FeatureId, Target};
use platform::accesskit::Role;
use render::ThemeName;
use views::Intent;
use views::feature::FeatureIntent;
use views::text::repo as words;

use super::changes_menu::menu_of;
use super::support::Window;

#[test]
fn upkeep_clean_up__the_menu_of_the_row_shown_has_clean_up_and_look_after_it_in_the_background() {
    let repositories = super::group_tabs::made(false);
    let state = super::group_tabs::window(&repositories);
    let mut window = Window::new(ThemeName::Dark, super::tabs::WIDE);
    let items = menu_of(&mut window, &state, "api");
    assert!(items.iter().any(|item| item == words::CLEAN_UP), "{items:?}");
    assert!(items.iter().any(|item| item == words::LOOK_AFTER), "{items:?}");
    let intents = window.click(&state, Role::MenuItem, words::CLEAN_UP);
    let clean_up = Intent::Feature(FeatureIntent::Run(FeatureId("upkeep.clean-up"), Target::Head));
    assert!(intents.contains(&clean_up), "{intents:?}");
    let items = menu_of(&mut window, &state, "web");
    assert!(
        !items.iter().any(|item| item == words::CLEAN_UP),
        "only the row shown: {items:?}"
    );
}

/// The menu of the row of `api` in a window on `api`, its worktree and `web`, once the window read `api`.
fn api_menu(window: &mut Window, repositories: &super::group_tabs::Repositories) -> (Vec<String>, app::store::State) {
    let state = super::group_tabs::window(repositories);
    let items = menu_of(window, &state, "api");
    (items, state)
}

#[test]
fn upkeep_write_commit_graph__the_row_shown_offers_speed_up_reading_the_history_after_clean_up_while_there_is_no_file()
{
    let repositories = super::group_tabs::made(false);
    let mut window = Window::new(ThemeName::Dark, super::tabs::WIDE);
    let (items, state) = api_menu(&mut window, &repositories);
    let clean_up = items.iter().position(|item| item == words::CLEAN_UP).unwrap();
    assert_eq!(
        items.get(clean_up + 1).map(String::as_str),
        Some(words::SPEED_UP_HISTORY),
        "{items:?}"
    );
    let intents = window.click(&state, Role::MenuItem, words::SPEED_UP_HISTORY);
    let write = Intent::Feature(FeatureIntent::Run(FeatureId("upkeep.write-commit-graph"), Target::Head));
    assert!(intents.contains(&write), "{intents:?}");
    let api = repositories.api.to_str().unwrap();
    let _ = std::process::Command::new("git")
        .args(["-C", api, "commit-graph", "write", "--reachable"])
        .output()
        .unwrap();
    let mut window = Window::new(ThemeName::Dark, super::tabs::WIDE);
    let (items, _) = api_menu(&mut window, &repositories);
    assert!(
        !items.iter().any(|item| item == words::SPEED_UP_HISTORY),
        "not once git keeps the file: {items:?}"
    );
}

#[test]
fn upkeep_write_commit_graph__with_core_commit_graph_false_the_entry_is_not_in_the_menu() {
    let repositories = super::group_tabs::made(false);
    let api = repositories.api.to_str().unwrap();
    let _ = std::process::Command::new("git")
        .args(["-C", api, "config", "core.commitGraph", "false"])
        .output()
        .unwrap();
    let mut window = Window::new(ThemeName::Dark, super::tabs::WIDE);
    let (items, _) = api_menu(&mut window, &repositories);
    assert!(items.iter().any(|item| item == words::CLEAN_UP), "{items:?}");
    assert!(!items.iter().any(|item| item == words::SPEED_UP_HISTORY), "{items:?}");
}
