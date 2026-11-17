//! The icons of the actions in the context menus of the window, on repositories made for each test (`ui.menu-icons`):
//! the menu of a commit of the timeline and the menu of a file of the Files page, drawn in each theme.

use app::store::State;
use render::{Point, Rect, ThemeName};
use testkit::Repository;
use ui::{InputEvent, Modifiers, PointerButton, PointerButtonEvent};

use super::files_page::{files_page, row};
use super::pointer::{moved, rows_drawn};
use super::showing;
use super::support::Window;

/// The themes, each with the end of the names of its snapshots.
const THEMES: [(ThemeName, &str); 4] = [
    (ThemeName::Dark, "dark"),
    (ThemeName::Light, "light"),
    (ThemeName::HighContrastDark, "high-contrast-dark"),
    (ThemeName::HighContrastLight, "high-contrast-light"),
];

/// A repository whose `main`, checked out, has `Fix the parser` over `Add the readme`, and whose `feature` has `Show
/// the branch rail` over `Add the readme`.
fn two_branches() -> Repository {
    let mut repository = Repository::new();
    let _ = repository.commit("Add the readme", &[("README.md", "Gitardo\n")]);
    repository.branch("feature");
    repository.checkout("feature");
    let _ = repository.commit("Show the branch rail", &[("rail.txt", "branches\n")]);
    repository.checkout("main");
    let _ = repository.commit("Fix the parser", &[("parser.txt", "parse\n")]);
    repository
}

/// Presses and lets go the button of context menus at a point, then draws the menu it opened.
fn right_click(window: &mut Window, state: &State, point: Point) {
    let _ = window.draw(state, &[moved(point)]);
    let press = PointerButtonEvent {
        button: PointerButton::Secondary,
        position: point,
        modifiers: Modifiers::NONE,
        click_count: 1,
    };
    let _ = window.draw(
        state,
        &[InputEvent::PointerPressed(press), InputEvent::PointerReleased(press)],
    );
    let _ = window.draw(state, &[]);
}

/// A point on the hash at the end of a row of the timeline, where no label is.
fn on_hash(row: Rect) -> Point {
    Point::new(row.right() - 40.0, row.y + row.height * 0.5)
}

#[test]
fn ui_menu_icons__the_menu_of_a_commit_and_the_menu_of_a_file_match_their_snapshots_in_the_four_themes() {
    let repository = two_branches();
    for (theme, name) in THEMES {
        let state = showing(repository.path().to_path_buf());
        let mut window = Window::new(theme, (1000, 940));
        let (_, tree) = window.draw(&state, &[]);
        let rows = rows_drawn(&tree);
        right_click(&mut window, &state, on_hash(rows[1]));
        window.assert_snapshot(&format!("menu-icons-commit-{name}"));
    }
    for (theme, name) in THEMES {
        let (state, _, _driver) = files_page(&repository, &[]);
        let mut window = Window::new(theme, (1000, 700));
        let (_, tree) = window.draw(&state, &[]);
        let point = row(&tree, "README.md");
        right_click(&mut window, &state, point);
        window.assert_snapshot(&format!("menu-icons-file-{name}"));
    }
}
