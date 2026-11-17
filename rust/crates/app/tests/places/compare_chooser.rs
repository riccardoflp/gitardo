//! The comparison chooser (`compare.chooser`), drawn from the store: both sides of a comparison chosen in one dialog,
//! from the bar of views and from a side of a comparison page; and a comparison beside its diffs
//! (`compare.beside-diff`).

use app::store::State;
use app::store::compare::chooser::chooser_state;
use model::compare::CompareMode;
use platform::accesskit::{Role, TreeUpdate};
use render::{Point, Rect, ThemeName};
use testkit::Repository;
use ui::{InputEvent, Key, KeyEvent, Modifiers, NamedKey};
use views::Intent;
use views::compare::{ChooserBox, CompareIntent, offered};
use views::diff::DiffVersion;
use views::page::Page;

use super::compare::{compared, page_id, repository, until_ended};
use super::pointer::{bounds, click_at, drag, middle, rect, wheel};
use super::showing;
use super::support::{OperationsDriver, Window, has, says, says_in, typed};

/// A window as wide as the one of the other tests of comparisons.
const SIZE: (u32, u32) = (1200, 900);

/// The rightmost button of a name: the one of the dialog, the page or the column, not the one of the branch rail or of
/// the bar of views.
fn rightmost(tree: &TreeUpdate, name: &str) -> Rect {
    tree.nodes
        .iter()
        .filter(|(_, node)| node.role() == Role::Button && node.label() == Some(name))
        .filter_map(|(_, node)| node.bounds().map(rect))
        .max_by(|one, other| one.x.total_cmp(&other.x))
        .unwrap_or_else(|| panic!("no button {name}"))
}

/// Clicks the rightmost button of a name, and hands what the click asks to the store.
fn click(window: &mut Window, (driver, state): (&OperationsDriver, &mut State), name: &str) {
    let (_, tree) = window.draw(state, &[]);
    for intent in click_at(window, state, middle(rightmost(&tree, name)), 1) {
        driver.intent(state, intent);
    }
}

/// Draws a frame with events, a second after the one before, hands what they ask to the store, and gives the tree of
/// the next frame.
fn press(window: &mut Window, (driver, state): (&OperationsDriver, &mut State), events: &[InputEvent]) -> TreeUpdate {
    window.now += std::time::Duration::from_secs(1);
    let (intents, _) = window.draw(state, events);
    for intent in intents {
        driver.intent(state, intent);
    }
    window.draw(state, &[]).1
}

/// A key pressed with no modifier.
fn key(named: NamedKey) -> InputEvent {
    InputEvent::KeyPressed(KeyEvent::press(Key::Named(named), Modifiers::NONE))
}

/// The chooser opened from **Compare** in the bar of views, over a window drawn once.
fn opened(repository: &Repository) -> (State, OperationsDriver, Window) {
    let mut state = showing(repository.path().to_path_buf());
    let driver = OperationsDriver::new();
    let mut window = Window::new(ThemeName::Dark, SIZE);
    for intent in window.click(&state, Role::Button, "Compare") {
        driver.intent(&mut state, intent);
    }
    let _ = window.draw(&state, &[]);
    (state, driver, window)
}

/// The names of what the list of the chooser offers.
fn names(state: &State) -> Vec<String> {
    offered(&chooser_state(state).unwrap())
        .into_iter()
        .map(|row| row.side.name)
        .collect()
}

/// The name held by a box of the chooser.
fn held(state: &State, which: ChooserBox) -> Option<String> {
    let chooser = state.compares.chooser.as_ref().unwrap();
    let side = match which {
        ChooserBox::From => &chooser.from,
        ChooserBox::To => &chooser.to,
    };
    side.as_ref().map(|side| side.side.name.clone())
}

/// The names of the two sides of the only comparison page, and the path it is narrowed to.
fn compared_sides(state: &State) -> (String, String, Option<String>) {
    let target = &state.compares.pages.first().unwrap().target;
    (
        target.first.name.clone(),
        target.second.name.clone(),
        target.path.clone(),
    )
}

/// `main` with five commits, and `feature` from its second.
fn five_on_main() -> Repository {
    let mut repository = Repository::new();
    let _ = repository.commit("Add a", &[("a.txt", "a\n")]);
    let _ = repository.commit("Add b", &[("b.txt", "b\n")]);
    repository.branch("feature");
    for name in ["c", "d", "e"] {
        let _ = repository.commit(&format!("Add {name}"), &[(&format!("{name}.txt"), "x\n")]);
    }
    repository
}

#[test]
fn compare_chooser__opened_from_the_bar_of_views_from_holds_the_branch_checked_out_and_the_list_is_that_of_to() {
    let repository = repository();
    let (state, _driver, mut window) = opened(&repository);
    let chooser = state.compares.chooser.as_ref().unwrap();
    assert_eq!(held(&state, ChooserBox::From).as_deref(), Some("main"));
    assert_eq!(chooser.choosing, ChooserBox::To);
    let offered = names(&state);
    assert!(offered.contains(&String::from("feature")), "{offered:?}");
    assert!(!offered.contains(&String::from("main")), "{offered:?}");
    let (_, tree) = window.draw(&state, &[]);
    assert!(has(&tree, Role::Button, "From main"));
    assert!(says(&tree, views::text::compare::CHOOSER_TITLE));
}

#[test]
fn compare_chooser__choosing_feature_fills_to_says_how_far_it_is_and_enter_opens_the_page_of_the_two() {
    let repository = repository();
    let (mut state, driver, mut window) = opened(&repository);
    click(&mut window, (&driver, &mut state), "feature");
    assert_eq!(held(&state, ChooserBox::To).as_deref(), Some("feature"));
    let tree = window.draw(&state, &[]).1;
    assert!(says_in(&tree, "2 commits ahead"), "the summary of the two is said");
    let _ = press(&mut window, (&driver, &mut state), &[key(NamedKey::Enter)]);
    until_ended(&driver, &mut state);
    assert!(state.compares.chooser.is_none());
    let _ = page_id(&state);
    assert_eq!(
        compared_sides(&state),
        (String::from("main"), String::from("feature"), None)
    );
}

#[test]
fn compare_chooser__the_swap_arrows_exchange_from_and_to() {
    let repository = repository();
    let (mut state, driver, mut window) = opened(&repository);
    click(&mut window, (&driver, &mut state), "feature");
    click(&mut window, (&driver, &mut state), views::text::compare::SWAP_TOOLTIP);
    assert_eq!(held(&state, ChooserBox::From).as_deref(), Some("feature"));
    assert_eq!(held(&state, ChooserBox::To).as_deref(), Some("main"));
}

#[test]
fn compare_chooser__head_3_typed_offers_use_head_3_and_choosing_it_fills_the_box() {
    let repository = five_on_main();
    let (mut state, driver, mut window) = opened(&repository);
    let tree = press(&mut window, (&driver, &mut state), &[typed("HEAD~3")]);
    assert_eq!(names(&state).first().map(String::as_str), Some("HEAD~3"));
    assert!(has(&tree, Role::Button, "Use HEAD~3"));
    click(&mut window, (&driver, &mut state), "Use HEAD~3");
    assert_eq!(held(&state, ChooserBox::To).as_deref(), Some("HEAD~3"));
}

#[test]
fn compare_chooser__a_text_git_does_not_know_offers_nothing_and_compare_stays_disabled() {
    let repository = repository();
    let (mut state, driver, mut window) = opened(&repository);
    let tree = press(&mut window, (&driver, &mut state), &[typed("no-such-thing")]);
    assert_eq!(names(&state), Vec::<String>::new());
    assert!(says(&tree, views::text::compare::UNKNOWN_REVISION));
    assert!(!chooser_state(&state).unwrap().can_compare());
    let _ = press(&mut window, (&driver, &mut state), &[key(NamedKey::Enter)]);
    assert!(state.compares.chooser.is_some(), "Enter accepts nothing");
    assert_eq!(state.compares.pages, []);
}

/// A repository with 300 branches more than `main` and `feature`, written as files of `.git/refs/heads`.
fn three_hundred_branches() -> Repository {
    let repository = repository();
    let head = repository.git(&["rev-parse", "HEAD"]);
    let heads = repository.path().join(".git").join("refs").join("heads");
    for number in 0..300 {
        std::fs::write(heads.join(format!("topic-{number:03}")), &head).unwrap();
    }
    repository
}

/// Whether a button is wholly in sight in the list of the chooser.
fn in_sight(tree: &TreeUpdate, name: &str) -> bool {
    let list = bounds(tree, Role::ListBox, views::text::compare::SIDES);
    tree.nodes
        .iter()
        .filter(|(_, node)| node.role() == Role::Button && node.label() == Some(name))
        .filter_map(|(_, node)| node.bounds().map(rect))
        .any(|button| button.y >= list.y && button.bottom() <= list.bottom() + 0.5)
}

#[test]
fn compare_chooser__with_300_branches_the_list_scrolls_to_the_last_one_with_the_bar_and_with_end() {
    let repository = three_hundred_branches();
    let (mut state, driver, mut window) = opened(&repository);
    let last = names(&state).last().cloned().unwrap();
    assert!(
        !in_sight(&window.draw(&state, &[]).1, &last),
        "the last row is out of sight at first"
    );
    let tree = press(&mut window, (&driver, &mut state), &[key(NamedKey::End)]);
    assert!(in_sight(&tree, &last), "End shows {last}");
    let (mut state, driver, mut window) = opened(&repository);
    let tree = window.draw(&state, &[]).1;
    let sides = bounds(&tree, Role::ListBox, views::text::compare::SIDES);
    let from = Point::new(sides.right() - 4.0, sides.y + 4.0);
    let to = Point::new(from.x, sides.bottom() + 200.0);
    for intent in drag(&mut window, &state, from, to) {
        driver.intent(&mut state, intent);
    }
    window.now += std::time::Duration::from_secs(1);
    assert!(
        in_sight(&window.draw(&state, &[]).1, &last),
        "the bar at its foot shows {last}"
    );
}

#[test]
fn compare_chooser__a_click_on_an_entry_of_opened_before_opens_that_comparison_narrowed_as_it_was() {
    let repository = repository();
    let (mut state, driver) = compared(&repository, ("ref:main", "ref:feature"));
    let id = page_id(&state);
    driver.intent(
        &mut state,
        Intent::Compare(CompareIntent::TypePath(id, String::from("src"))),
    );
    driver.intent(&mut state, Intent::Compare(CompareIntent::Narrow(id)));
    driver.intent(&mut state, Intent::ClosePage);
    assert_eq!(state.compares.pages, []);
    let mut window = Window::new(ThemeName::Dark, SIZE);
    for intent in window.click(&state, Role::Button, "Compare") {
        driver.intent(&mut state, intent);
    }
    click(&mut window, (&driver, &mut state), "main \u{2194} feature \u{b7} src");
    until_ended(&driver, &mut state);
    assert_eq!(
        compared_sides(&state),
        (String::from("main"), String::from("feature"), Some(String::from("src")))
    );
}

#[test]
fn compare_chooser__a_click_on_the_second_side_of_a_page_chooses_to_and_release_is_compared_in_the_same_page() {
    let repository = repository();
    repository.branch("release");
    let (mut state, driver) = compared(&repository, ("ref:main", "ref:feature"));
    let id = page_id(&state);
    let mut window = Window::new(ThemeName::Dark, SIZE);
    click(&mut window, (&driver, &mut state), "feature");
    assert_eq!(state.compares.chooser.as_ref().unwrap().choosing, ChooserBox::To);
    assert_eq!(held(&state, ChooserBox::From).as_deref(), Some("main"));
    click(&mut window, (&driver, &mut state), "release");
    click(&mut window, (&driver, &mut state), views::text::compare::COMPARE);
    until_ended(&driver, &mut state);
    assert_eq!(page_id(&state), id);
    assert_eq!(
        compared_sides(&state),
        (String::from("main"), String::from("release"), None)
    );
}

#[test]
fn compare_chooser__escape_opens_nothing() {
    let repository = repository();
    let (mut state, driver, mut window) = opened(&repository);
    click(&mut window, (&driver, &mut state), "feature");
    let _ = press(&mut window, (&driver, &mut state), &[key(NamedKey::Escape)]);
    assert!(state.compares.chooser.is_none());
    assert_eq!(state.compares.pages, []);
    assert_eq!(state.page_shown(), None);
}

#[test]
fn the_comparison_chooser_is_drawn_in_each_theme() {
    let repository = repository();
    for (theme, name) in [
        (ThemeName::Dark, "compare-chooser-dark"),
        (ThemeName::Light, "compare-chooser-light"),
        (ThemeName::HighContrastDark, "compare-chooser-high-contrast-dark"),
        (ThemeName::HighContrastLight, "compare-chooser-high-contrast-light"),
    ] {
        let mut state = showing(repository.path().to_path_buf());
        let driver = OperationsDriver::new();
        let mut window = Window::new(theme, SIZE);
        for intent in window.click(&state, Role::Button, "Compare") {
            driver.intent(&mut state, intent);
        }
        click(&mut window, (&driver, &mut state), "feature");
        let _ = window.draw(&state, &[]);
        window.assert_snapshot(name);
    }
}

/// The comparison of `main` and `feature` with the diff of `src/b.txt` beside it, in a window of a size.
fn beside(size: (u32, u32)) -> (Repository, State, OperationsDriver, Window) {
    let repository = repository();
    let (mut state, driver) = compared(&repository, ("ref:main", "ref:feature"));
    let mut window = Window::new(ThemeName::Dark, size);
    click(&mut window, (&driver, &mut state), "src/b.txt");
    (repository, state, driver, window)
}

/// The right side of the file the Diff page beside the comparison shows.
fn diff_file(state: &State) -> String {
    let page = state.compares.pages.first().unwrap();
    let diff = page.beside.as_ref().unwrap().diff;
    let shown = state.diff_pages.pages.iter().find(|page| page.id == diff).unwrap();
    shown
        .target
        .files
        .first()
        .map(|file| file.right.clone())
        .unwrap_or_default()
}

#[test]
fn compare_beside_diff__a_click_on_a_file_shows_its_diff_on_the_left_and_the_comparison_on_the_right_with_it_marked() {
    let (_repository, state, _driver, mut window) = beside(SIZE);
    let id = page_id(&state);
    assert_eq!(state.page_shown(), Some(Page::Compare(id)));
    assert_eq!(diff_file(&state), "src/b.txt");
    let page = state.compares.pages.first().unwrap();
    assert_eq!(page.beside.as_ref().unwrap().shown.as_deref(), Some("src/b.txt"));
    let tree = window.draw(&state, &[]).1;
    let column = bounds(&tree, Role::Complementary, views::text::compare::COLUMN);
    let file = rightmost(&tree, "src/b.txt");
    assert!(file.x >= column.x, "the file is listed in the column on the right");
    assert!(column.x > 600.0, "the column is on the right");
}

#[test]
fn compare_beside_diff__a_click_on_another_file_in_the_column_shows_it_in_the_same_place_and_ctrl_tab_lists_one_page() {
    let (_repository, mut state, driver, mut window) = beside(SIZE);
    let diff = state.compares.pages.first().unwrap().beside.as_ref().unwrap().diff;
    click(&mut window, (&driver, &mut state), "c.txt");
    assert_eq!(diff_file(&state), "c.txt");
    assert_eq!(
        state.compares.pages.first().unwrap().beside.as_ref().unwrap().diff,
        diff
    );
    let id = page_id(&state);
    assert_eq!(state.pages.open, [Page::Compare(id)], "Ctrl+Tab lists one page");
}

/// The commit of a side of a diff.
fn commit_of(version: &DiffVersion) -> Option<model::CommitId> {
    match version {
        DiffVersion::Commit { commit, .. } => Some(commit.clone()),
        _ => None,
    }
}

#[test]
fn compare_beside_diff__the_arrows_of_the_column_swap_the_sides_and_the_diff_is_the_same_file_the_other_way() {
    let repository = repository();
    let (mut state, driver) = compared(&repository, ("ref:main", "ref:feature"));
    let id = page_id(&state);
    driver.intent(
        &mut state,
        Intent::Compare(CompareIntent::SetMode(id, CompareMode::Everything)),
    );
    let mut window = Window::new(ThemeName::Dark, SIZE);
    click(&mut window, (&driver, &mut state), "src/b.txt");
    let target = |state: &State| {
        let diff = state.compares.pages.first().unwrap().beside.as_ref().unwrap().diff;
        let page = state.diff_pages.pages.iter().find(|page| page.id == diff).unwrap();
        (page.target.left.clone(), page.target.right.clone())
    };
    let (left, right) = target(&state);
    click(&mut window, (&driver, &mut state), views::text::compare::SWAP_TOOLTIP);
    assert_eq!(compared_sides(&state).0, "feature");
    assert_eq!(diff_file(&state), "src/b.txt");
    let (swapped_left, swapped_right) = target(&state);
    assert_eq!(
        (commit_of(&swapped_left), commit_of(&swapped_right)),
        (commit_of(&right), commit_of(&left))
    );
}

#[test]
fn compare_beside_diff__escape_in_the_diff_gives_back_the_comparison_page_scrolled_where_it_was() {
    let repository = repository();
    let (mut state, driver) = compared(&repository, ("ref:main", "ref:feature"));
    let id = page_id(&state);
    let mut window = Window::new(ThemeName::Dark, (1200, 560));
    let (_, tree) = window.draw(&state, &[]);
    let page = bounds(&tree, Role::Button, "Open in the Diff page");
    let intents = wheel(&mut window, &state, Point::new(700.0, page.y), 6.0);
    for intent in intents {
        driver.intent(&mut state, intent);
    }
    let (_, tree) = window.draw(&state, &[]);
    let before = rightmost(&tree, "src/b.txt");
    assert!(before.bottom() <= 560.0, "the file is in sight once scrolled");
    click(&mut window, (&driver, &mut state), "src/b.txt");
    assert!(state.compares.pages.first().unwrap().beside.as_ref().unwrap().is_shown);
    let _ = press(&mut window, (&driver, &mut state), &[key(NamedKey::Escape)]);
    assert_eq!(state.page_shown(), Some(Page::Compare(id)));
    assert!(!state.compares.pages.first().unwrap().beside.as_ref().unwrap().is_shown);
    let (_, tree) = window.draw(&state, &[]);
    let after = rightmost(&tree, "src/b.txt");
    assert!((after.y - before.y).abs() < 0.5, "{before:?} {after:?}");
}

#[test]
fn compare_beside_diff__while_the_diff_is_shown_the_side_column_is_not() {
    let repository = repository();
    let (mut state, driver) = compared(&repository, ("ref:main", "ref:feature"));
    let mut window = Window::new(ThemeName::Dark, SIZE);
    let side = views::text::repo::SIDE_COLUMN;
    assert!(has(&window.draw(&state, &[]).1, Role::Navigation, side));
    click(&mut window, (&driver, &mut state), "src/b.txt");
    assert!(!has(&window.draw(&state, &[]).1, Role::Navigation, side));
}

#[test]
fn the_comparison_beside_its_diff_is_drawn_in_each_theme() {
    for (theme, name) in [
        (ThemeName::Dark, "compare-beside-diff-dark"),
        (ThemeName::Light, "compare-beside-diff-light"),
        (ThemeName::HighContrastDark, "compare-beside-diff-high-contrast-dark"),
        (ThemeName::HighContrastLight, "compare-beside-diff-high-contrast-light"),
    ] {
        let (_repository, state, _driver, _window) = beside(SIZE);
        let mut window = Window::new(theme, SIZE);
        let _ = window.draw(&state, &[]);
        let _ = window.draw(&state, &[]);
        window.assert_snapshot(name);
    }
}
