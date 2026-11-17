//! The title row of every page: the button that puts the side column away and brings it back, at the same place as in
//! the top row of the timeline (`repo.side-column`, `graph.toggle-references`).

use app::store::State;
use platform::accesskit::Role;
use render::ThemeName;
use views::Intent;
use views::graph::GraphIntent;
use views::page::Page;
use views::text::header::{HIDE_REFERENCES, SHOW_REFERENCES};
use views::text::repo::SIDE_COLUMN;

use super::beside::{click_words, three_files};
use super::diff::{head_to_working_tree, open};
use super::inspector::{ask, select};
use super::pointer::{apply, bounds};
use super::showing;
use super::support::{Window, has};

/// A window wide enough for the side column beside the pages.
const WIDE: (u32, u32) = (1200, 700);

/// Shows the settings page, the Changes page or a Diff page of the file changed in the working tree.
fn show(state: &mut State, page: &str) {
    match page {
        "settings" => ask(state, Intent::ShowPage(Page::Settings)),
        "changes" => ask(state, Intent::ShowPage(Page::Changes)),
        _ => open(state, head_to_working_tree("src/a.rs")),
    }
}

/// A repository with a change in the working tree, shown in a window.
fn changed() -> (testkit::Repository, State) {
    let repository = three_files();
    std::fs::write(repository.path().join("src/a.rs"), "a, changed again\n").unwrap();
    let state = showing(repository.path().to_path_buf());
    (repository, state)
}

#[test]
fn repo_side_column__the_settings_page_the_changes_page_and_a_diff_page_put_it_away_and_bring_it_back() {
    for page in ["settings", "changes", "diff"] {
        let (_repository, mut state) = changed();
        show(&mut state, page);
        let mut window = Window::new(ThemeName::Dark, WIDE);
        let (_, tree) = window.draw(&state, &[]);
        let beside = bounds(&tree, Role::Navigation, SIDE_COLUMN);
        let intents = window.click(&state, Role::Button, HIDE_REFERENCES);
        assert_eq!(intents, [Intent::Graph(GraphIntent::ToggleReferences)], "{page}");
        apply(&mut state, intents);
        let (_, tree) = window.draw(&state, &[]);
        assert!(!has(&tree, Role::Navigation, SIDE_COLUMN), "{page}");
        assert!(state.page_shown().is_some(), "the page stays: {page}");
        let intents = window.click(&state, Role::Button, SHOW_REFERENCES);
        apply(&mut state, intents);
        let (_, tree) = window.draw(&state, &[]);
        assert_eq!(bounds(&tree, Role::Navigation, SIDE_COLUMN), beside, "{page}");
    }
}

#[test]
fn graph_toggle_references__the_button_of_the_title_row_of_a_page_is_where_the_first_button_of_the_timeline_is() {
    let (_repository, mut state) = changed();
    let mut window = Window::new(ThemeName::Dark, WIDE);
    let (_, tree) = window.draw(&state, &[]);
    let in_timeline = bounds(&tree, Role::Button, HIDE_REFERENCES);
    for page in ["settings", "changes", "diff"] {
        show(&mut state, page);
        let (_, tree) = window.draw(&state, &[]);
        assert_eq!(bounds(&tree, Role::Button, HIDE_REFERENCES), in_timeline, "{page}");
    }
    apply(&mut state, vec![Intent::Graph(GraphIntent::ToggleReferences)]);
    let (_, tree) = window.draw(&state, &[]);
    let hidden = bounds(&tree, Role::Button, SHOW_REFERENCES);
    ask(&mut state, Intent::ShowTimeline);
    let (_, tree) = window.draw(&state, &[]);
    assert_eq!(bounds(&tree, Role::Button, SHOW_REFERENCES), hidden, "put away too");
}

#[test]
fn repo_side_column__in_a_window_800_wide_the_button_of_the_title_row_of_a_page_opens_it_over_the_page() {
    let (_repository, mut state) = changed();
    show(&mut state, "settings");
    let mut window = Window::new(ThemeName::Dark, (800, 580));
    let (_, tree) = window.draw(&state, &[]);
    assert!(!has(&tree, Role::Navigation, SIDE_COLUMN));
    let intents = window.click(&state, Role::Button, SHOW_REFERENCES);
    assert_eq!(intents, [Intent::Graph(GraphIntent::OpenSideColumn)]);
    apply(&mut state, intents);
    let (_, tree) = window.draw(&state, &[]);
    assert!(has(&tree, Role::Navigation, SIDE_COLUMN));
    assert_eq!(state.page_shown(), Some(Page::Settings), "over the page");
}

#[test]
fn repo_side_column__the_diff_page_beside_the_inspector_which_puts_it_away_has_no_button_for_it() {
    let repository = three_files();
    let mut state = showing(repository.path().to_path_buf());
    let _ = select(&mut state, "Change three files");
    let mut window = Window::new(ThemeName::Dark, WIDE);
    click_words(&mut window, &mut state, "a.rs");
    let (_, tree) = window.draw(&state, &[]);
    assert!(!has(&tree, Role::Button, HIDE_REFERENCES));
    assert!(!has(&tree, Role::Button, SHOW_REFERENCES));
}
