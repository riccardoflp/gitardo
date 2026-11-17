//! The Changes page beside what it opens, drawn from the store (`changes.beside-file`): a file of the list opens its
//! Diff page or its File page on the right of the list, which stays as a column with the file shown marked.

use app::store::{Message, State, ViewData};
use platform::accesskit::{Role, Toggled, TreeUpdate};
use render::{Point, ThemeName};
use testkit::Repository;
use ui::{InputEvent, Key, KeyEvent, Modifiers, NamedKey};
use views::changes::{BesideShown, ChangesIntent, ChangesState};
use views::diff::{DiffVersion, FileStep};
use views::page::{Page, PageState};
use views::{Content, Intent};

use super::pointer::{bounds, drag, middle, rect};
use super::showing;
use super::support::{self, OperationsDriver, Window, find, has};

/// A window wide enough for the column beside the page of a file.
const WIDE: (u32, u32) = (1200, 640);
/// A window narrower than the narrow layout.
const NARROW: (u32, u32) = (680, 640);

/// What is shown beside the list: the right side of the diff or the path of the file, whether it is a diff, and the
/// path the list marks.
#[derive(Debug, PartialEq, Eq)]
pub(super) struct Beside {
    /// The file shown: the right side of a diff of one file, the path of a File page, or `*` for every file.
    shown: String,
    /// Whether it is a Diff page.
    is_diff: bool,
    /// The path the list marks.
    marked: Option<String>,
}

/// A repository with one commit of `a.txt`, `b.txt` and `src/c.txt`, then each of them changed on disk.
pub(super) fn three_changed() -> Repository {
    let mut repository = Repository::new();
    let _ = repository.git(&["config", "user.name", "Ada"]);
    let _ = repository.git(&["config", "user.email", "ada@example.com"]);
    let _ = repository.commit(
        "Add the files",
        &[("a.txt", "a\n"), ("b.txt", "b\n"), ("src/c.txt", "c\n")],
    );
    for (path, text) in [
        ("a.txt", "a, changed\n"),
        ("b.txt", "b, changed\n"),
        ("src/c.txt", "c, changed\n"),
    ] {
        std::fs::write(repository.path().join(path), text).unwrap();
    }
    repository
}

/// A window showing a repository with its Changes page open.
pub(super) fn changes_page(repository: &Repository) -> (State, OperationsDriver) {
    let mut state = showing(repository.path().to_path_buf());
    let driver = OperationsDriver::new();
    driver.intent(&mut state, Intent::Changes(ChangesIntent::Open));
    (state, driver)
}

/// Reduces the intents of a frame.
pub(super) fn apply(driver: &OperationsDriver, state: &mut State, intents: Vec<Intent>) {
    for intent in intents {
        driver.intent(state, intent);
    }
}

/// What the Changes page shows beside its list; none while it shows the whole page.
pub(super) fn beside(state: &State) -> Option<Beside> {
    let data = ViewData::new(state, Vec::new(), support::NOW);
    let Content::Page(host) = data.window_state(state).content else {
        panic!("a page is shown");
    };
    let beside = match host.page {
        PageState::ChangesBeside(beside) => beside,
        PageState::Changes(_) => return None,
        other => panic!("the Changes page is shown, not {:?}", other.page()),
    };
    let ChangesState::Repository(changes) = beside.changes else {
        panic!("the column shows a repository");
    };
    let (shown, is_diff) = match &beside.shown {
        BesideShown::Diff(diff) => match diff.files {
            [file] => (file.right.clone(), true),
            _ => (String::from("*"), true),
        },
        BesideShown::File(file) => (file.path.to_owned(), false),
    };
    Some(Beside {
        shown,
        is_diff,
        marked: changes.marked.map(str::to_owned),
    })
}

/// What a file shown in a diff beside the list looks like.
pub(super) fn diff_of(path: &str) -> Beside {
    Beside {
        shown: path.to_owned(),
        is_diff: true,
        marked: Some(path.to_owned()),
    }
}

/// The two versions the Diff page beside the list compares.
fn versions(state: &State) -> (DiffVersion, DiffVersion) {
    let page = state
        .diff_pages
        .pages
        .iter()
        .find(|page| page.is_in_changes)
        .unwrap_or_else(|| panic!("a Diff page beside the list"));
    (page.target.left.clone(), page.target.right.clone())
}

/// The button of a role and a name in the row of a file of the list, found by the path the row's box is named.
fn in_row(tree: &TreeUpdate, (role, name): (Role, &str), path: &str) -> Point {
    let row = rect(find(tree, Role::CheckBox, path).bounds().unwrap());
    let found = tree
        .nodes
        .iter()
        .filter(|(_, node)| node.role() == role && node.label() == Some(name))
        .filter_map(|(_, node)| node.bounds().map(rect))
        .find(|button| (button.y - row.y).abs() < row.height)
        .unwrap_or_else(|| panic!("the button in the row"));
    middle(found)
}

/// Clicks a button of the row of a file, and reduces what it asks.
pub(super) fn click_in_row(driver: &OperationsDriver, state: &mut State, window: &mut Window, (name, path): (&str, &str)) {
    let (_, tree) = window.draw(state, &[]);
    let intents = window.click_at(state, in_row(&tree, (Role::Button, name), path), 1);
    apply(driver, state, intents);
}

/// Presses a key with `Alt` and reduces what it asks; gives what it asked.
fn alt(driver: &OperationsDriver, state: &mut State, window: &mut Window, key: NamedKey) -> Vec<Intent> {
    let pressed = InputEvent::KeyPressed(KeyEvent::press(Key::Named(key), Modifiers::ALT));
    let (intents, _) = window.draw(state, &[pressed]);
    apply(driver, state, intents.clone());
    intents
}

/// Waits for the operation running to end.
fn until_ended(driver: &OperationsDriver, state: &mut State) {
    for _ in 0..8 {
        if matches!(driver.next(state), Message::FeatureEnded { .. }) {
            return;
        }
    }
    panic!("the operation does not end");
}

/// Reads the repository again, as after a change on disk.
fn read_again(state: &mut State) {
    let _effects = support::run(state, Message::Intent(Intent::Refresh));
}

/// The step of the trail named **Changes**, not the view of the bar of views.
pub(super) fn trail_changes(tree: &TreeUpdate) -> Point {
    let trail = bounds(tree, Role::Navigation, views::text::pages::TRAIL);
    let found = tree
        .nodes
        .iter()
        .filter(|(_, node)| node.role() == Role::Button && node.label() == Some(views::text::changes::PAGE))
        .filter_map(|(_, node)| node.bounds().map(rect))
        .find(|button| button.y >= trail.y && button.bottom() <= trail.bottom() + 1.0 && button.x >= trail.x);
    middle(found.unwrap_or_else(|| panic!("Changes in the trail")))
}

#[test]
fn changes_beside_file__open_changes_shows_the_diff_on_the_right_and_the_list_stays_on_the_left_with_the_file_marked() {
    let repository = three_changed();
    let (mut state, driver) = changes_page(&repository);
    let mut window = Window::new(ThemeName::Dark, WIDE);
    click_in_row(&driver, &mut state, &mut window, ("Open changes", "a.txt"));
    assert_eq!(state.page_shown(), Some(Page::Changes));
    assert_eq!(beside(&state), Some(diff_of("a.txt")));
    let (_, tree) = window.draw(&state, &[]);
    let column = bounds(&tree, Role::Complementary, views::text::changes::COLUMN);
    let diff = bounds(&tree, Role::Main, views::text::diff::PAGE);
    assert!(
        column.right() <= diff.x + 1.0,
        "the list is on the left: {column:?} {diff:?}"
    );
    assert!(has(&tree, Role::CheckBox, "b.txt"), "the other files are still listed");
}

#[test]
fn changes_beside_file__open_file_then_a_click_set_to_diff_show_each_in_the_same_place_with_no_page_in_ctrl_tab() {
    let repository = three_changed();
    let (mut state, driver) = changes_page(&repository);
    let mut window = Window::new(ThemeName::Dark, WIDE);
    click_in_row(&driver, &mut state, &mut window, ("Open file", "a.txt"));
    let shown = Beside {
        shown: String::from("a.txt"),
        is_diff: false,
        marked: Some(String::from("a.txt")),
    };
    assert_eq!(beside(&state), Some(shown));
    state.settings.open_on_click = model::changes::OpenOnClick::Diff;
    let (_, tree) = window.draw(&state, &[]);
    let name = name_of(&tree, "b.txt");
    let intents = window.click_at(&state, name, 1);
    apply(&driver, &mut state, intents);
    assert_eq!(beside(&state), Some(diff_of("b.txt")));
    assert_eq!(
        state.pages.open,
        [Page::Changes],
        "Ctrl+Tab lists the Changes page alone"
    );
}

#[test]
fn changes_beside_file__ticking_the_box_of_the_file_shown_stages_it_and_the_place_shows_its_staged_changes() {
    let repository = three_changed();
    let (mut state, driver) = changes_page(&repository);
    let mut window = Window::new(ThemeName::Dark, WIDE);
    click_in_row(&driver, &mut state, &mut window, ("Open changes", "a.txt"));
    assert_eq!(versions(&state), (DiffVersion::Index, DiffVersion::WorkingTree));
    let intents = window.click(&state, Role::CheckBox, "a.txt");
    apply(&driver, &mut state, intents);
    until_ended(&driver, &mut state);
    let (_, tree) = window.draw(&state, &[]);
    assert_eq!(find(&tree, Role::CheckBox, "a.txt").toggled(), Some(Toggled::True));
    assert_eq!(beside(&state), Some(diff_of("a.txt")));
    assert_eq!(versions(&state), (DiffVersion::Head, DiffVersion::Index));
}

#[test]
fn changes_beside_file__a_file_shown_whose_changes_are_discarded_leaves_the_whole_page() {
    let repository = three_changed();
    let (mut state, driver) = changes_page(&repository);
    let mut window = Window::new(ThemeName::Dark, WIDE);
    click_in_row(&driver, &mut state, &mut window, ("Open changes", "b.txt"));
    let _ = repository.git(&["checkout", "--", "b.txt"]);
    read_again(&mut state);
    assert_eq!(beside(&state), None);
    let (_, tree) = window.draw(&state, &[]);
    assert!(!has(&tree, Role::CheckBox, "b.txt"));
    assert!(has(&tree, Role::CheckBox, "a.txt"));
    assert!(state.diff_pages.pages.is_empty(), "the Diff page is forgotten");
}

#[test]
fn changes_beside_file__alt_down_and_alt_up_show_the_next_and_the_previous_file_in_the_order_of_the_list() {
    let repository = three_changed();
    let (mut state, driver) = changes_page(&repository);
    let mut window = Window::new(ThemeName::Dark, WIDE);
    click_in_row(&driver, &mut state, &mut window, ("Open changes", "a.txt"));
    let intents = alt(&driver, &mut state, &mut window, NamedKey::ArrowDown);
    assert_eq!(intents, [Intent::Changes(ChangesIntent::StepFile(FileStep::Next))]);
    assert_eq!(beside(&state), Some(diff_of("b.txt")));
    let _ = alt(&driver, &mut state, &mut window, NamedKey::ArrowDown);
    assert_eq!(beside(&state), Some(diff_of("src/c.txt")));
    let _ = alt(&driver, &mut state, &mut window, NamedKey::ArrowDown);
    assert_eq!(beside(&state), Some(diff_of("src/c.txt")), "the last file stays shown");
    let _ = alt(&driver, &mut state, &mut window, NamedKey::ArrowUp);
    assert_eq!(beside(&state), Some(diff_of("b.txt")));
    assert_eq!(state.diff_pages.pages.len(), 1, "one Diff page shows each file in turn");
}

#[test]
fn changes_beside_file__in_the_tree_layout_alt_down_goes_through_the_files_as_the_tree_lists_them() {
    let repository = three_changed();
    let (mut state, driver) = changes_page(&repository);
    let tree = Intent::Changes(ChangesIntent::Files(views::details::FilesIntent::Layout(
        views::details::FileLayout::Tree,
    )));
    driver.intent(&mut state, tree);
    let mut window = Window::new(ThemeName::Dark, WIDE);
    click_in_row(&driver, &mut state, &mut window, ("Open changes", "src/c.txt"));
    let _ = alt(&driver, &mut state, &mut window, NamedKey::ArrowDown);
    assert_eq!(
        beside(&state),
        Some(diff_of("a.txt")),
        "the folder src comes first in the tree"
    );
}

#[test]
fn changes_beside_file__changes_in_the_trail_and_the_changes_view_of_the_bar_of_views_show_the_whole_page_again() {
    let repository = three_changed();
    let (mut state, driver) = changes_page(&repository);
    let mut window = Window::new(ThemeName::Dark, WIDE);
    for way in ["trail", "bar of views"] {
        click_in_row(&driver, &mut state, &mut window, ("Open changes", "a.txt"));
        assert_eq!(beside(&state), Some(diff_of("a.txt")), "{way}");
        let intents = if way == "trail" {
            let (_, tree) = window.draw(&state, &[]);
            let intents = window.click_at(&state, trail_changes(&tree), 1);
            assert_eq!(intents, [Intent::Changes(ChangesIntent::ShowWhole)]);
            intents
        } else {
            vec![Intent::ShowPage(Page::Changes)]
        };
        apply(&driver, &mut state, intents);
        assert_eq!(beside(&state), None, "{way}");
        assert_eq!(state.page_shown(), Some(Page::Changes));
    }
}

#[test]
fn changes_beside_file__its_title_row_starts_with_the_button_of_the_side_column_and_alt_left_shows_the_whole_page() {
    let repository = three_changed();
    // Shown from the bar of views, which leaves the keyboard to the page rather than to the box of the message, which
    // uses `Alt+Left` while it has it.
    let mut state = showing(repository.path().to_path_buf());
    let driver = OperationsDriver::new();
    driver.intent(&mut state, Intent::ShowPage(Page::Changes));
    let mut window = Window::new(ThemeName::Dark, WIDE);
    let (_, tree) = window.draw(&state, &[]);
    let side_button = bounds(&tree, Role::Button, views::text::header::HIDE_REFERENCES);
    click_in_row(&driver, &mut state, &mut window, ("Open changes", "a.txt"));
    let (_, tree) = window.draw(&state, &[]);
    assert_eq!(
        bounds(&tree, Role::Button, views::text::header::HIDE_REFERENCES),
        side_button,
        "the button of the side column stays where it is on the whole page"
    );
    let changes = trail_changes(&tree);
    let diff = bounds(&tree, Role::Main, views::text::diff::PAGE);
    assert!(changes.x < diff.x, "Changes is in the title row, before the diff's name");
    let alt_left = InputEvent::KeyPressed(KeyEvent::press(Key::Named(NamedKey::ArrowLeft), Modifiers::ALT));
    let (intents, _) = window.draw(&state, &[alt_left]);
    assert_eq!(intents, [Intent::Changes(ChangesIntent::ShowWhole)]);
    apply(&driver, &mut state, intents);
    assert_eq!(beside(&state), None);
    assert_eq!(state.page_shown(), Some(Page::Changes));
}

#[test]
fn changes_beside_file__the_line_dragged_resizes_the_column() {
    let repository = three_changed();
    let (mut state, driver) = changes_page(&repository);
    let mut window = Window::new(ThemeName::Dark, WIDE);
    click_in_row(&driver, &mut state, &mut window, ("Open changes", "a.txt"));
    let (_, tree) = window.draw(&state, &[]);
    let column = bounds(&tree, Role::Complementary, views::text::changes::COLUMN);
    let line = Point::new(column.right() + 0.5, column.y + column.height * 0.5);
    let intents = drag(&mut window, &state, line, Point::new(line.x + 60.0, line.y));
    super::pointer::apply(&mut state, intents);
    let (_, tree) = window.draw(&state, &[]);
    let wider = bounds(&tree, Role::Complementary, views::text::changes::COLUMN);
    assert!((wider.width - column.width - 60.0).abs() < 1.0, "{column:?} {wider:?}");
}

#[test]
fn changes_beside_file__in_a_narrow_window_the_page_of_the_file_is_shown_alone() {
    let repository = three_changed();
    let (mut state, driver) = changes_page(&repository);
    let mut window = Window::new(ThemeName::Dark, NARROW);
    click_in_row(&driver, &mut state, &mut window, ("Open changes", "a.txt"));
    assert_eq!(beside(&state), Some(diff_of("a.txt")));
    let (_, tree) = window.draw(&state, &[]);
    assert!(!has(&tree, Role::Complementary, views::text::changes::COLUMN));
    assert!(has(&tree, Role::Main, views::text::diff::PAGE));
}

#[test]
fn changes_beside_file__open_all_changes_shows_every_file_beside_the_list_and_alt_down_the_first() {
    let repository = three_changed();
    let (mut state, driver) = changes_page(&repository);
    let mut window = Window::new(ThemeName::Dark, WIDE);
    let intents = window.click(&state, Role::Button, views::text::changes::OPEN_ALL_CHANGES);
    apply(&driver, &mut state, intents);
    let every = Beside {
        shown: String::from("*"),
        is_diff: true,
        marked: None,
    };
    assert_eq!(beside(&state), Some(every));
    let _ = alt(&driver, &mut state, &mut window, NamedKey::ArrowDown);
    assert_eq!(beside(&state), Some(diff_of("a.txt")));
}

#[test]
fn the_changes_page_beside_a_diff_is_drawn_in_each_theme() {
    let repository = three_changed();
    for (theme, name) in [
        (ThemeName::Dark, "changes-beside-diff-dark"),
        (ThemeName::Light, "changes-beside-diff-light"),
        (ThemeName::HighContrastDark, "changes-beside-diff-high-contrast-dark"),
        (ThemeName::HighContrastLight, "changes-beside-diff-high-contrast-light"),
    ] {
        let (mut state, driver) = changes_page(&repository);
        let mut window = Window::new(theme, WIDE);
        click_in_row(&driver, &mut state, &mut window, ("Open changes", "b.txt"));
        let _ = window.draw(&state, &[]);
        window.assert_snapshot(name);
    }
}

/// The middle of the name of a file of the list: its box is named by its path too, and the name is right of it.
fn name_of(tree: &TreeUpdate, path: &str) -> Point {
    let name = tree
        .nodes
        .iter()
        .filter(|(_, node)| node.role() != Role::CheckBox && (node.label() == Some(path) || node.value() == Some(path)))
        .filter_map(|(_, node)| node.bounds().map(rect))
        .max_by(|one, other| one.x.total_cmp(&other.x))
        .unwrap_or_else(|| panic!("the name of the file"));
    Point::new(name.x + 4.0, name.y + name.height * 0.5)
}

#[test]
fn changes_open__open_all_changes_in_one_diff_page_with_three_changed_files_opens_one_diff_page_with_three_files() {
    let repository = three_changed();
    let _ = repository.git(&["add", "a.txt"]);
    let (mut state, driver) = changes_page(&repository);
    let mut window = Window::new(ThemeName::Dark, WIDE);
    let intents = window.click(&state, Role::Button, views::text::changes::OPEN_ALL_CHANGES);
    apply(&driver, &mut state, intents);
    assert_eq!(
        state.page_shown(),
        Some(Page::Changes),
        "the Diff page is beside the list"
    );
    assert_eq!(state.diff_pages.pages.len(), 1);
    let page = state.diff_pages.pages.first().unwrap();
    let files: Vec<&str> = page.target.files.iter().map(|file| file.right.as_str()).collect();
    assert_eq!(files, ["a.txt", "b.txt", "src/c.txt"]);
    assert_eq!(page.title, "Changes");
}
