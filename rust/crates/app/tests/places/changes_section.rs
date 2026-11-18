//! **Changes**, the section of the side column between **Repositories** and **Branches** (`changes.section`), drawn
//! from the store: its heading and its count, what it lists, what a click on a file of it shows, and how it folds.

use app::store::State;
use platform::accesskit::{Role, TreeUpdate};
use render::{Point, Rect, ThemeName};
use testkit::Repository;
use views::Intent;
use views::changes::ChangesIntent;
use views::graph::{GraphIntent, SideSection};
use views::page::Page;
use views::text::{changes as words, header, repo};

use super::changes_beside::{apply, beside, diff_of, three_changed, until_ended};
use super::pointer::{bounds, middle, rect};
use super::showing;
use super::side_column::column;
use super::support::{OperationsDriver, Window, has, typed};

/// A window wide enough for the side column beside the timeline.
const WIDE: (u32, u32) = (1200, 760);
/// A window narrower than the narrow layout.
const NARROW: (u32, u32) = (800, 640);

/// A window showing the timeline of a repository, with nothing folded.
fn timeline(repository: &Repository) -> (State, OperationsDriver) {
    (showing(repository.path().to_path_buf()), OperationsDriver::new())
}

/// What the **Changes** section shows under its heading; none while it shows nothing.
fn section(tree: &TreeUpdate) -> Option<Rect> {
    super::pointer::changes_section(tree)
}

/// The node of a role and a name inside a rectangle, which the test fails without.
fn within(tree: &TreeUpdate, named: (Role, &str), part: Rect) -> Rect {
    super::pointer::bounds_in(tree, named, part)
}

/// A point on the row of a file in the section, right of its box.
fn row_in_section(tree: &TreeUpdate, path: &str) -> Point {
    let part = section(tree).unwrap_or_else(|| panic!("the section shows the changes"));
    let row = within(tree, (Role::Button, path), part);
    Point::new(row.x + row.width * 0.6, row.y + row.height * 0.5)
}

/// The count the heading of **Changes** says.
fn count_said(tree: &TreeUpdate) -> Option<String> {
    tree.nodes
        .iter()
        .find(|(_, node)| node.role() == Role::Button && node.label() == Some(repo::CHANGES))
        .and_then(|(_, node)| node.value().map(str::to_owned))
}

#[test]
fn changes_section__with_a_file_changed_it_is_between_repositories_and_branches_counts_1_and_lists_the_file_with_its_box()
 {
    let mut repository = Repository::new();
    let _ = repository.commit("Add the file", &[("a.txt", "a\n")]);
    std::fs::write(repository.path().join("a.txt"), "a, changed\n").unwrap();
    let (state, _driver) = timeline(&repository);
    let (_, tree) = Window::new(ThemeName::Dark, WIDE).draw(&state, &[]);
    let side = column(&tree);
    let headings = [repo::REPOSITORIES, repo::CHANGES, repo::BRANCHES].map(|name| bounds(&tree, Role::Button, name));
    assert!(
        headings[0].y < headings[1].y && headings[1].y < headings[2].y,
        "{headings:?}"
    );
    for heading in headings {
        assert!(
            heading.x >= side.x && heading.right() <= side.right() + 0.5,
            "{heading:?}"
        );
    }
    assert_eq!(count_said(&tree).as_deref(), Some("1"));
    let part = section(&tree).unwrap();
    assert!(part.y >= headings[1].bottom() - 0.5 && part.bottom() <= headings[2].y + 0.5);
    let _box = within(&tree, (Role::CheckBox, "a.txt"), part);
    let _row = within(&tree, (Role::Button, "a.txt"), part);
    let _commit = within(&tree, (Role::Button, words::COMMIT), part);
    // With nothing changed the heading says its name alone.
    let _ = repository.git(&["checkout", "--", "a.txt"]);
    let (state, _driver) = timeline(&repository);
    let (_, tree) = Window::new(ThemeName::Dark, WIDE).draw(&state, &[]);
    assert_eq!(count_said(&tree), None);
    assert!(section(&tree).is_some(), "the commit box is still there");
}

#[test]
fn changes_section__with_no_repository_in_the_workspace_it_is_its_heading_alone_with_nothing_under_it() {
    let folder = Repository::new();
    let empty = folder.path().parent().unwrap().join("no repository here");
    std::fs::create_dir_all(&empty).unwrap();
    let state = showing(empty);
    let (_, tree) = Window::new(ThemeName::Dark, WIDE).draw(&state, &[]);
    let changes = bounds(&tree, Role::Button, repo::CHANGES);
    let branches = bounds(&tree, Role::Button, repo::BRANCHES);
    assert_eq!(count_said(&tree), None);
    assert_eq!(section(&tree), None, "nothing under the heading");
    assert!(
        branches.y - changes.bottom() < 2.0,
        "the heading of Branches is right under it: {changes:?} {branches:?}"
    );
}

#[test]
fn changes_section__ticking_the_box_of_a_file_stages_it_and_a_message_typed_there_with_commit_makes_the_commit() {
    let repository = three_changed();
    let (mut state, driver) = timeline(&repository);
    let mut window = Window::new(ThemeName::Dark, WIDE);
    let (_, tree) = window.draw(&state, &[]);
    let part = section(&tree).unwrap();
    let ticked = within(&tree, (Role::CheckBox, "a.txt"), part);
    let intents = window.click_at(&state, middle(ticked), 1);
    apply(&driver, &mut state, intents);
    until_ended(&driver, &mut state);
    assert_eq!(repository.git(&["diff", "--cached", "--name-only"]).trim(), "a.txt");
    let (_, tree) = window.draw(&state, &[]);
    let part = section(&tree).unwrap();
    let message = tree
        .nodes
        .iter()
        .filter(|(_, node)| node.role() == Role::TextInput)
        .filter_map(|(_, node)| node.bounds().map(rect))
        .find(|found| found.y >= part.y && found.bottom() <= part.bottom())
        .unwrap_or_else(|| panic!("the message box of the section"));
    let intents = window.click_at(&state, middle(message), 1);
    apply(&driver, &mut state, intents);
    let (intents, _) = window.draw(&state, &[typed("Change a")]);
    apply(&driver, &mut state, intents);
    assert!(
        state
            .persisted
            .commit_messages
            .iter()
            .any(|kept| kept.message == "Change a"),
        "what is typed in the section is the message of the commit box"
    );
    let (_, tree) = window.draw(&state, &[]);
    let commit = within(&tree, (Role::Button, words::COMMIT), section(&tree).unwrap());
    let intents = window.click_at(&state, middle(commit), 1);
    assert!(intents.contains(&Intent::Changes(ChangesIntent::Commit)), "{intents:?}");
    apply(&driver, &mut state, intents);
    until_ended(&driver, &mut state);
    assert_eq!(repository.git(&["log", "-1", "--format=%s"]).trim(), "Change a");
}

#[test]
fn changes_section__a_click_on_a_file_with_the_timeline_shown_shows_its_diff_where_the_timeline_was_beside_the_side_column()
 {
    let repository = three_changed();
    let (mut state, driver) = timeline(&repository);
    assert_eq!(state.page_shown(), None, "the timeline is shown");
    let mut window = Window::new(ThemeName::Dark, WIDE);
    let (_, tree) = window.draw(&state, &[]);
    let intents = window.click_at(&state, row_in_section(&tree, "b.txt"), 1);
    apply(&driver, &mut state, intents);
    assert_eq!(state.page_shown(), Some(Page::Changes));
    assert_eq!(beside(&state), Some(diff_of("b.txt")));
    let (_, tree) = window.draw(&state, &[]);
    let side = column(&tree);
    let diff = bounds(&tree, Role::Main, views::text::diff::PAGE);
    assert!(
        side.right() <= diff.x,
        "the side column is beside the diff: {side:?} {diff:?}"
    );
    assert!(
        !has(&tree, Role::Complementary, words::COLUMN),
        "the page does not draw its own column of the list"
    );
    assert!(!has(&tree, Role::Main, words::PAGE), "the list is in the section alone");
    let row = tree.nodes.iter().find(|(_, node)| {
        node.role() == Role::Button && node.label() == Some("b.txt") && node.is_selected() == Some(true)
    });
    assert!(row.is_some(), "the row of b.txt is marked in the section");
    // With the section folded, or the side column put away, the page draws its own column again.
    for hides in [
        GraphIntent::FoldSection(SideSection::Changes),
        GraphIntent::ToggleReferences,
    ] {
        let mut hidden = state.clone();
        driver.intent(&mut hidden, Intent::Graph(hides.clone()));
        let (_, tree) = Window::new(ThemeName::Dark, WIDE).draw(&hidden, &[]);
        assert!(has(&tree, Role::Complementary, words::COLUMN), "{hides:?}");
        assert!(has(&tree, Role::Main, views::text::diff::PAGE), "{hides:?}");
    }
}

#[test]
fn changes_section__folding_it_leaves_its_heading_alone_and_it_is_folded_after_the_window_opens_again() {
    let repository = three_changed();
    let (mut state, driver) = timeline(&repository);
    let mut window = Window::new(ThemeName::Dark, WIDE);
    let intents = window.click(&state, Role::Button, repo::CHANGES);
    assert_eq!(intents, [Intent::Graph(GraphIntent::FoldSection(SideSection::Changes))]);
    apply(&driver, &mut state, intents);
    let (_, tree) = window.draw(&state, &[]);
    assert_eq!(section(&tree), None);
    assert_eq!(
        count_said(&tree).as_deref(),
        Some("3"),
        "the heading still counts the files"
    );
    let changes = bounds(&tree, Role::Button, repo::CHANGES);
    let branches = bounds(&tree, Role::Button, repo::BRANCHES);
    assert!(branches.y - changes.bottom() < 2.0, "{changes:?} {branches:?}");
    let text = state.persisted.to_text();
    assert!(text.contains("\"changesFolded\""), "{text}");
    let again = app::persisted::Persisted::from_text(&text);
    assert!(again.panes.iter().any(|panes| panes.folds.is_changes_folded));
    let mut opened = showing(repository.path().to_path_buf());
    opened.persisted = again;
    let (_, tree) = Window::new(ThemeName::Dark, WIDE).draw(&opened, &[]);
    assert_eq!(section(&tree), None, "folded after the window opens again");
}

#[test]
fn changes_section__in_a_window_800_wide_it_is_in_the_side_column_opened_over_the_timeline() {
    let repository = three_changed();
    let (mut state, driver) = timeline(&repository);
    let mut window = Window::new(ThemeName::Dark, NARROW);
    let (_, tree) = window.draw(&state, &[]);
    assert!(!has(&tree, Role::Button, repo::CHANGES), "the side column is put away");
    let intents = window.click(&state, Role::Button, header::SHOW_REFERENCES);
    apply(&driver, &mut state, intents);
    let (_, tree) = window.draw(&state, &[]);
    assert_eq!(count_said(&tree).as_deref(), Some("3"));
    let part = section(&tree).unwrap();
    let _row = within(&tree, (Role::Button, "a.txt"), part);
}

#[test]
fn changes_section__the_side_column_with_its_three_sections_matches_its_snapshots_in_each_theme() {
    let repository = three_changed();
    let _ = repository.git(&["add", "b.txt"]);
    let themes = [
        (ThemeName::Dark, "dark"),
        (ThemeName::Light, "light"),
        (ThemeName::HighContrastDark, "high-contrast-dark"),
        (ThemeName::HighContrastLight, "high-contrast-light"),
    ];
    for (theme, name) in themes {
        let (state, _driver) = timeline(&repository);
        let mut window = Window::new(theme, (900, 640));
        let _ = window.draw(&state, &[]);
        window.assert_snapshot(&format!("side-column-three-sections-{name}"));
    }
}

/// Presses a key with `Alt` and reduces what it asks.
fn alt(driver: &OperationsDriver, state: &mut State, window: &mut Window, key: ui::NamedKey) {
    let pressed = ui::InputEvent::KeyPressed(ui::KeyEvent::press(ui::Key::Named(key), ui::Modifiers::ALT));
    let (intents, _) = window.draw(state, &[pressed]);
    apply(driver, state, intents);
}

#[test]
fn changes_section__alt_down_and_alt_up_show_the_next_and_the_previous_file_and_open_file_of_the_menu_shows_the_file() {
    let repository = three_changed();
    let (mut state, driver) = timeline(&repository);
    let mut window = Window::new(ThemeName::Dark, WIDE);
    let (_, tree) = window.draw(&state, &[]);
    let intents = window.click_at(&state, row_in_section(&tree, "a.txt"), 1);
    apply(&driver, &mut state, intents);
    assert_eq!(beside(&state), Some(diff_of("a.txt")));
    alt(&driver, &mut state, &mut window, ui::NamedKey::ArrowDown);
    assert_eq!(beside(&state), Some(diff_of("b.txt")));
    alt(&driver, &mut state, &mut window, ui::NamedKey::ArrowUp);
    assert_eq!(beside(&state), Some(diff_of("a.txt")));
    // The menu of the row in the section has what the menu of the row of the page has.
    let (_, tree) = window.draw(&state, &[]);
    let tree = super::changes_menu::right_click(&mut window, &state, row_in_section(&tree, "b.txt"));
    let items = super::side_column::menu_items(&tree);
    assert_eq!(
        items.first().map(String::as_str),
        Some(words::OPEN_CHANGES),
        "{items:?}"
    );
    let intents = window.click(&state, Role::MenuItem, words::OPEN_FILE);
    apply(&driver, &mut state, intents);
    assert_eq!(beside(&state), Some(super::changes_beside::file_of("b.txt")));
    let (_, tree) = window.draw(&state, &[]);
    assert!(!has(&tree, Role::Complementary, words::COLUMN));
}

#[test]
fn changes_section__with_the_changes_page_open_too_both_list_the_files_and_a_box_ticked_in_either_stages_its_file() {
    let repository = three_changed();
    let (mut state, driver) = timeline(&repository);
    driver.intent(&mut state, Intent::ShowPage(Page::Changes));
    let mut window = Window::new(ThemeName::Dark, WIDE);
    let (_, tree) = window.draw(&state, &[]);
    let part = section(&tree).unwrap();
    let page = bounds(&tree, Role::Main, words::PAGE);
    assert!(part.right() <= page.x, "{part:?} {page:?}");
    for path in ["a.txt", "b.txt", "src/c.txt"] {
        let _in_section = within(&tree, (Role::CheckBox, path), part);
        let _in_page = within(&tree, (Role::CheckBox, path), page);
    }
    let in_page = within(&tree, (Role::CheckBox, "a.txt"), page);
    let intents = window.click_at(&state, middle(in_page), 1);
    apply(&driver, &mut state, intents);
    until_ended(&driver, &mut state);
    let (_, tree) = window.draw(&state, &[]);
    let in_section = within(&tree, (Role::CheckBox, "b.txt"), section(&tree).unwrap());
    let intents = window.click_at(&state, middle(in_section), 1);
    apply(&driver, &mut state, intents);
    until_ended(&driver, &mut state);
    let staged = repository.git(&["diff", "--cached", "--name-only"]);
    assert_eq!(staged.lines().collect::<Vec<_>>(), ["a.txt", "b.txt"]);
    // Each shows what the other did.
    let (_, tree) = window.draw(&state, &[]);
    let page = bounds(&tree, Role::Main, words::PAGE);
    for (path, part) in [("a.txt", section(&tree).unwrap()), ("b.txt", page)] {
        let ticked = tree
            .nodes
            .iter()
            .filter(|(_, node)| node.role() == Role::CheckBox && node.label() == Some(path))
            .filter(|(_, node)| node.toggled() == Some(platform::accesskit::Toggled::True))
            .filter_map(|(_, node)| node.bounds().map(rect))
            .any(|found| super::pointer::is_inside(found, part));
        assert!(ticked, "{path} is ticked in {part:?}");
    }
}

#[test]
fn changes_beside_file__while_the_side_column_shows_changes_open_the_file_is_drawn_alone_and_beside_the_column_of_the_page_otherwise()
 {
    let repository = three_changed();
    let (mut state, driver) = timeline(&repository);
    driver.intent(&mut state, Intent::ShowPage(Page::Changes));
    let mut window = Window::new(ThemeName::Dark, WIDE);
    let (_, tree) = window.draw(&state, &[]);
    let whole = bounds(&tree, Role::Main, words::PAGE);
    let row = within(&tree, (Role::Button, "a.txt"), whole);
    let intents = window.click_at(&state, middle(row), 1);
    apply(&driver, &mut state, intents);
    assert_eq!(beside(&state), Some(diff_of("a.txt")));
    let (_, tree) = window.draw(&state, &[]);
    assert!(!has(&tree, Role::Complementary, words::COLUMN));
    let alone = bounds(&tree, Role::Main, views::text::diff::PAGE);
    assert!(
        (alone.x - whole.x).abs() < 0.5,
        "the diff starts where the page did: {alone:?}"
    );
    driver.intent(
        &mut state,
        Intent::Graph(GraphIntent::FoldSection(SideSection::Changes)),
    );
    let (_, tree) = window.draw(&state, &[]);
    let column = bounds(&tree, Role::Complementary, words::COLUMN);
    let diff = bounds(&tree, Role::Main, views::text::diff::PAGE);
    assert!(column.right() <= diff.x + 1.0, "{column:?} {diff:?}");
    let _listed = within(&tree, (Role::CheckBox, "a.txt"), column);
}

#[test]
fn graph_resize_panes__dragging_the_line_under_changes_down_gives_it_the_height_branches_loses_and_it_is_kept() {
    let repository = three_changed();
    let (mut state, driver) = timeline(&repository);
    let mut window = Window::new(ThemeName::Dark, WIDE);
    let (_, tree) = window.draw(&state, &[]);
    let rail = |tree: &TreeUpdate| bounds(tree, Role::Navigation, views::text::rail::RAIL);
    let (above, under) = (section(&tree).unwrap(), rail(&tree));
    let line = Point::new(above.x + above.width * 0.5, above.bottom() + 0.5);
    let intents = super::pointer::drag(&mut window, &state, line, Point::new(line.x, line.y + 30.0));
    let is_sized = |intent: &Intent| {
        matches!(
            intent,
            Intent::Graph(GraphIntent::SetPane(views::graph::Pane::ChangesSection, Some(_)))
        )
    };
    assert!(intents.iter().any(is_sized), "{intents:?}");
    apply(&driver, &mut state, intents);
    let (_, tree) = window.draw(&state, &[]);
    let (taller, shorter) = (section(&tree).unwrap(), rail(&tree));
    assert!((taller.height - above.height - 30.0).abs() < 0.5, "{taller:?}");
    assert!((under.height - shorter.height - 30.0).abs() < 0.5, "{shorter:?}");
    let text = state.persisted.to_text();
    assert!(text.contains("\"changesSection\""), "{text}");
    let mut opened = showing(repository.path().to_path_buf());
    opened.persisted = app::persisted::Persisted::from_text(&text);
    let (_, tree) = Window::new(ThemeName::Dark, WIDE).draw(&opened, &[]);
    assert!((section(&tree).unwrap().height - taller.height).abs() < 0.5);
}

#[test]
fn changes_section__the_side_column_with_each_section_folded_a_menu_of_a_row_and_a_diff_opened_from_it_are_drawn() {
    let repository = three_changed();
    let _ = repository.git(&["add", "b.txt"]);
    let size = (900, 640);
    for (folded, name) in [
        (SideSection::Repositories, "repositories"),
        (SideSection::Changes, "changes"),
        (SideSection::Branches, "branches"),
    ] {
        let (mut state, driver) = timeline(&repository);
        driver.intent(&mut state, Intent::Graph(GraphIntent::FoldSection(folded)));
        let mut window = Window::new(ThemeName::Dark, size);
        let _ = window.draw(&state, &[]);
        window.assert_snapshot(&format!("side-column-{name}-folded-dark"));
    }
    let (mut state, driver) = timeline(&repository);
    let mut window = Window::new(ThemeName::Dark, size);
    let (_, tree) = window.draw(&state, &[]);
    let _ = super::changes_menu::right_click(&mut window, &state, row_in_section(&tree, "b.txt"));
    window.assert_snapshot("changes-section-row-menu-dark");
    let mut window = Window::new(ThemeName::Dark, size);
    let (_, tree) = window.draw(&state, &[]);
    let intents = window.click_at(&state, row_in_section(&tree, "a.txt"), 1);
    apply(&driver, &mut state, intents);
    let _ = window.draw(&state, &[]);
    window.assert_snapshot("changes-section-diff-dark");
}
