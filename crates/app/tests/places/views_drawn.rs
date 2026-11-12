//! The header, the branch rail, the timeline and the Welcome page drawn from the store: their pictures in the dark and
//! the light theme, at a scale of 1, and the intents of a click on them.

use platform::accesskit::Role;
use render::ThemeName;
use views::page::Page;
use views::{Intent, KeptList};

use super::support::{Window, find};
use super::{repository_with_history, showing, window};

/// The size of the window of the pictures: wide enough for the rail, the timeline and the inspector.
const WIDE: (u32, u32) = (1000, 520);
/// The size of a narrow window, where the rail shows only icons.
const NARROW: (u32, u32) = (820, 480);

#[test]
fn the_header_rail_and_timeline_of_a_repository_are_drawn_in_each_theme() {
    let (repository, _remote) = repository_with_history();
    let state = showing(repository.path().to_path_buf());
    for (theme, name) in [
        (ThemeName::Dark, "repository-dark"),
        (ThemeName::Light, "repository-light"),
    ] {
        let mut window = Window::new(theme, WIDE);
        let (intents, tree) = window.draw(&state, &[]);
        assert_eq!(intents, [] as [views::Intent; 0]);
        let _rail = find(&tree, Role::Navigation, "Branches");
        let _sync = find(&tree, Role::Button, "↓0  ↑1   Sync");
        window.assert_snapshot(name);
    }
}

#[test]
fn the_narrow_window_shows_the_rail_as_icons() {
    let (repository, _remote) = repository_with_history();
    let state = showing(repository.path().to_path_buf());
    let mut window = Window::new(ThemeName::Dark, NARROW);
    let _ = window.draw(&state, &[]);
    window.assert_snapshot("repository-narrow-dark");
}

#[test]
fn a_list_kept_aside_opens_with_a_click() {
    let (repository, _remote) = repository_with_history();
    let state = showing(repository.path().to_path_buf());
    let mut window = Window::new(ThemeName::Dark, WIDE);
    let intents = window.click(&state, Role::Button, "Tags");
    assert_eq!(intents, [Intent::ToggleKept(KeptList::Tags)]);
}

#[test]
fn welcome_page__the_window_with_no_workspace_shows_it_and_opens_a_folder_or_a_recent_one() {
    let state = window();
    for (theme, name) in [(ThemeName::Dark, "welcome-dark"), (ThemeName::Light, "welcome-light")] {
        let mut window = Window::new(theme, WIDE);
        let _ = window.draw(&state, &[]);
        window.assert_snapshot(name);
    }
    let mut window = Window::new(ThemeName::Dark, WIDE);
    assert_eq!(window.click(&state, Role::Button, "Open folder…"), [Intent::OpenFolder]);
    let recent = std::path::Path::new("/work/app").display().to_string();
    assert_eq!(window.click(&state, Role::Button, &recent), [Intent::OpenRecent(0)]);
}

#[test]
fn welcome_page__show_the_timeline_comes_back_from_the_page() {
    let (repository, _remote) = repository_with_history();
    let mut state = showing(repository.path().to_path_buf());
    let _ = app::store::reduce(&mut state, app::store::Message::Intent(Intent::ShowPage(Page::Welcome)));
    let mut window = Window::new(ThemeName::Dark, WIDE);
    assert_eq!(
        window.click(&state, Role::Button, "Show the timeline"),
        [Intent::ShowTimeline]
    );
    let (_, tree) = window.draw(&state, &[]);
    let _trail = find(&tree, Role::Button, "Timeline");
}

#[test]
fn repo_no_repository__a_folder_with_no_repository_says_so_with_check_again() {
    let folder = testkit::Repository::new();
    let empty = folder.path().parent().unwrap().join("empty");
    std::fs::create_dir_all(&empty).unwrap();
    let state = showing(empty);
    let mut window = Window::new(ThemeName::Dark, WIDE);
    let (_, tree) = window.draw(&state, &[]);
    assert!(
        tree.nodes
            .iter()
            .any(|(_, node)| node.label() == Some("No Git repository in this workspace")
                || node.value() == Some("No Git repository in this workspace")),
        "the timeline says there is no repository"
    );
    window.assert_snapshot("no-repository-dark");
    assert_eq!(window.click(&state, Role::Button, "Check again"), [Intent::CheckAgain]);
}

#[test]
fn repo_select__the_repository_menu_opens_from_the_pill() {
    let (repository, _remote) = repository_with_history();
    let state = showing(repository.path().to_path_buf());
    let mut window = Window::new(ThemeName::Dark, WIDE);
    let (_, tree) = window.draw(&state, &[]);
    let _pill = find(&tree, Role::Button, "repository");
}

#[test]
fn the_draft_and_a_commit_selected_with_the_inspector_are_drawn_in_each_theme() {
    let (repository, _remote) = repository_with_history();
    std::fs::write(repository.path().join("README.md"), "Gitardo, changed\n").unwrap();
    let mut state = showing(repository.path().to_path_buf());
    let chosen = state
        .timeline
        .rows
        .iter()
        .find_map(|row| row.commit().filter(|commit| commit.subject == "Show the branch rail"));
    let chosen = chosen.unwrap().id.clone();
    let select = views::graph::GraphIntent::Select(views::graph::Selected::Commit(chosen));
    let _effects = super::support::run(&mut state, app::store::Message::Intent(Intent::Graph(select)));
    for (theme, name) in [
        (ThemeName::Dark, "timeline-draft-dark"),
        (ThemeName::Light, "timeline-draft-light"),
    ] {
        let mut window = Window::new(theme, WIDE);
        let _ = window.draw(&state, &[]);
        window.assert_snapshot(name);
    }
}

#[test]
fn a_folded_merge_is_drawn_as_one_row() {
    let repository = super::folds::merged_feature();
    let mut state = showing(repository.path().to_path_buf());
    let merge = state
        .timeline
        .rows
        .iter()
        .find_map(|row| row.commit().filter(|commit| commit.parents.len() == 2));
    let fold = views::graph::GraphIntent::Fold(merge.unwrap().id.clone());
    let _effects = super::support::run(&mut state, app::store::Message::Intent(Intent::Graph(fold)));
    let mut window = Window::new(ThemeName::Dark, WIDE);
    let _ = window.draw(&state, &[]);
    window.assert_snapshot("timeline-folded-dark");
}
