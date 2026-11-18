//! The menu of a file of the changes (`changes.row-menu`), drawn from the store: the rows have no buttons, and what the
//! buttons did is in the context menu of the row, by the right button and by the menu key.

use app::store::State;
use platform::accesskit::{Role, Toggled, TreeUpdate};
use render::{Point, ThemeName};
use ui::{InputEvent, Key, KeyEvent, Modifiers, NamedKey, PointerButton, PointerButtonEvent};
use views::Intent;
use views::changes::ChangesIntent;
use views::text::changes as words;

use super::changes_beside::{apply, beside, changes_page, diff_of, file_of, three_changed, until_ended};
use super::pointer::{moved, rect};
use super::side_column::menu_items;
use super::support::{OperationsDriver, Window, find, has};

/// A window wide enough for the whole Changes page.
const WIDE: (u32, u32) = (1200, 640);

/// A point on the row of a file of the list, right of its box: the row is a button named by the path of the file,
/// and the lowest one so named, since the trail may name a page by the same path.
pub(super) fn name_of(tree: &TreeUpdate, path: &str) -> Point {
    let row = tree
        .nodes
        .iter()
        .filter(|(_, node)| node.role() == Role::Button && node.label() == Some(path))
        .filter_map(|(_, node)| node.bounds().map(rect))
        .max_by(|one, other| one.y.total_cmp(&other.y))
        .unwrap_or_else(|| panic!("the row of the file {path}"));
    Point::new(row.x + row.width * 0.5, row.y + row.height * 0.5)
}

/// Presses and lets go the button of context menus at a point, then draws the menu it opened and gives its tree.
pub(super) fn right_click(window: &mut Window, state: &State, point: Point) -> TreeUpdate {
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
    window.draw(state, &[]).1
}

/// Opens the menu of the row of a file, closing first a menu left open, and gives the names of its items.
pub(super) fn menu_of(window: &mut Window, state: &State, path: &str) -> Vec<String> {
    let (_, tree) = window.draw(state, &[]);
    if !menu_items(&tree).is_empty() {
        let _ = window.draw(state, &[key(NamedKey::Escape)]);
    }
    let (_, tree) = window.draw(state, &[]);
    let tree = right_click(window, state, name_of(&tree, path));
    menu_items(&tree)
}

/// Chooses an item of the menu of the row of a file, and reduces what it asks; gives what it asked.
pub(super) fn choose(
    driver: &OperationsDriver,
    state: &mut State,
    window: &mut Window,
    (item, path): (&str, &str),
) -> Vec<Intent> {
    let items = menu_of(window, state, path);
    assert!(
        items.iter().any(|name| name == item),
        "{item} is in the menu of {path}: {items:?}"
    );
    let intents = window.click(state, Role::MenuItem, item);
    apply(driver, state, intents.clone());
    intents
}

/// A key pressed with no modifier.
fn key(named: NamedKey) -> InputEvent {
    InputEvent::KeyPressed(KeyEvent::press(Key::Named(named), Modifiers::NONE))
}

/// The buttons a row of a file had before its actions went into its menu.
const ROW_BUTTONS: [&str; 5] = [
    words::OPEN_CHANGES,
    words::OPEN_FILE,
    words::DISCARD,
    words::STASH_FILE,
    words::SAVE,
];

#[test]
fn changes_row_menu__the_row_of_a_changed_file_has_its_box_and_no_button() {
    let repository = three_changed();
    let (state, _driver) = changes_page(&repository);
    let (_, tree) = Window::new(ThemeName::Dark, WIDE).draw(&state, &[]);
    for path in ["a.txt", "b.txt", "src/c.txt"] {
        assert!(has(&tree, Role::CheckBox, path), "the box of {path}");
    }
    for button in ROW_BUTTONS {
        assert!(!has(&tree, Role::Button, button), "no button {button} on a row");
    }
}

#[test]
fn changes_row_menu__the_menu_of_a_changed_file_has_its_actions_and_unstage_on_a_file_wholly_staged() {
    let repository = three_changed();
    let _ = repository.git(&["add", "b.txt"]);
    let (state, _driver) = changes_page(&repository);
    let mut window = Window::new(ThemeName::Dark, WIDE);
    assert_eq!(
        menu_of(&mut window, &state, "a.txt"),
        [
            words::OPEN_CHANGES,
            words::OPEN_FILE,
            words::STAGE,
            words::DISCARD,
            words::STASH_FILE
        ]
    );
    assert_eq!(
        menu_of(&mut window, &state, "b.txt"),
        [
            words::OPEN_CHANGES,
            words::OPEN_FILE,
            words::UNSTAGE,
            words::DISCARD,
            words::STASH_FILE
        ]
    );
}

#[test]
fn changes_row_menu__stage_discard_stash_open_file_and_open_changes_do_what_the_buttons_did() {
    use model::contract::{FeatureId, Target};
    use views::feature::FeatureIntent;
    let repository = three_changed();
    let (mut state, driver) = changes_page(&repository);
    let mut window = Window::new(ThemeName::Dark, WIDE);
    let _ = choose(&driver, &mut state, &mut window, (words::OPEN_CHANGES, "a.txt"));
    assert_eq!(beside(&state), Some(diff_of("a.txt")));
    let _ = choose(&driver, &mut state, &mut window, (words::OPEN_FILE, "b.txt"));
    let shown = beside(&state).unwrap_or_else(|| panic!("the file is shown beside the list"));
    assert_eq!(shown, file_of("b.txt"));
    driver.intent(&mut state, Intent::Changes(ChangesIntent::ShowWhole));
    let run = |feature: &'static str, target: Target| Intent::Feature(FeatureIntent::Run(FeatureId(feature), target));
    let (mut asking, asking_driver) = changes_page(&repository);
    let _ = asking_driver;
    for (item, asked) in [
        (
            words::DISCARD,
            run("changes.discard", Target::File(String::from("a.txt"))),
        ),
        (
            words::STASH_FILE,
            run("stash.push-file", Target::Files(vec![String::from("a.txt")])),
        ),
    ] {
        let items = menu_of(&mut window, &asking, "a.txt");
        assert!(items.iter().any(|name| name == item), "{items:?}");
        let intents = window.click(&asking, Role::MenuItem, item);
        assert!(intents.contains(&asked), "{item}: {intents:?}");
        asking = changes_page(&repository).0;
    }
    let _ = choose(&driver, &mut state, &mut window, (words::STAGE, "a.txt"));
    until_ended(&driver, &mut state);
    let (_, tree) = window.draw(&state, &[]);
    assert_eq!(find(&tree, Role::CheckBox, "a.txt").toggled(), Some(Toggled::True));
    assert!(repository.git(&["diff", "--cached", "--name-only"]).contains("a.txt"));
}

#[test]
fn changes_row_menu__the_menu_of_a_deleted_file_has_no_open_file_and_that_of_a_file_not_saved_has_save() {
    let repository = three_changed();
    std::fs::remove_file(repository.path().join("b.txt")).unwrap();
    let (state, _driver) = changes_page(&repository);
    let mut window = Window::new(ThemeName::Dark, WIDE);
    let items = menu_of(&mut window, &state, "b.txt");
    assert!(items.iter().any(|name| name == words::OPEN_CHANGES), "{items:?}");
    assert!(!items.iter().any(|name| name == words::OPEN_FILE), "{items:?}");
    assert!(!items.iter().any(|name| name == words::SAVE), "{items:?}");
    let (state, _driver) = super::changes_save::edited(
        &repository,
        &[(
            "a.txt",
            "a, typed
",
        )],
    );
    let items = menu_of(&mut Window::new(ThemeName::Dark, WIDE), &state, "a.txt");
    assert_eq!(items.last().map(String::as_str), Some(words::SAVE), "{items:?}");
    assert!(items.iter().any(|name| name == words::STAGE), "{items:?}");
}

#[test]
fn changes_row_menu__with_the_keyboard_on_a_row_the_menu_key_opens_its_menu_and_enter_on_stage_stages_the_file() {
    let repository = three_changed();
    let (mut state, driver) = changes_page(&repository);
    let mut window = Window::new(ThemeName::Dark, WIDE);
    // The page gives the keyboard to its message box once; `Tab` then goes through its widgets to the row of `a.txt`.
    let (intents, _) = window.draw(&state, &[]);
    apply(&driver, &mut state, intents);
    let mut is_on_row = false;
    for _ in 0..40 {
        let _ = window.draw(&state, &[key(NamedKey::Tab)]);
        let (_, tree) = window.draw(&state, &[]);
        let focused = tree.nodes.iter().find(|(id, _)| *id == tree.focus);
        eprintln!(
            "{:?}",
            focused.map(|(_, node)| (node.role(), node.label().map(str::to_owned)))
        );
        is_on_row = focused.is_some_and(|(_, node)| node.role() == Role::Button && node.label() == Some("a.txt"));
        if is_on_row {
            break;
        }
    }
    assert!(is_on_row, "Tab reaches the row of a.txt");
    let _ = window.draw(&state, &[key(NamedKey::ContextMenu)]);
    let (_, tree) = window.draw(&state, &[]);
    let items = menu_items(&tree);
    assert_eq!(
        items.first().map(String::as_str),
        Some(words::OPEN_CHANGES),
        "{items:?}"
    );
    // The first arrow is on **Open changes**; **Stage** is the third item.
    let mut intents = Vec::new();
    for pressed in [
        NamedKey::ArrowDown,
        NamedKey::ArrowDown,
        NamedKey::ArrowDown,
        NamedKey::Enter,
    ] {
        intents.extend(window.draw(&state, &[key(pressed)]).0);
    }
    apply(&driver, &mut state, intents);
    until_ended(&driver, &mut state);
    assert!(repository.git(&["diff", "--cached", "--name-only"]).contains("a.txt"));
}

#[test]
fn changes_row_menu__the_menu_of_a_row_matches_its_snapshot_in_the_dark_and_the_light_theme() {
    let repository = three_changed();
    let _ = repository.git(&["add", "b.txt"]);
    for (theme, name) in [(ThemeName::Dark, "dark"), (ThemeName::Light, "light")] {
        let (state, _driver) = changes_page(&repository);
        let mut window = Window::new(theme, (900, 520));
        let _ = menu_of(&mut window, &state, "b.txt");
        window.assert_snapshot(&format!("changes-row-menu-{name}"));
    }
}

#[test]
fn changes_row_menu__the_menu_of_a_file_in_conflict_has_mark_as_resolved_and_the_two_sides_and_mark_as_resolved_stages_it()
 {
    let mut repository = testkit::Repository::new();
    repository.conflict();
    let path = testkit::CONFLICT_FILE;
    let mut state = super::showing(repository.path().to_path_buf());
    let driver = OperationsDriver::new();
    driver.intent(&mut state, Intent::Changes(ChangesIntent::Open));
    let mut window = Window::new(ThemeName::Dark, WIDE);
    let (_, tree) = window.draw(&state, &[]);
    for button in [words::MARK_AS_RESOLVED, words::TAKE_INCOMING, words::TAKE_CURRENT] {
        assert!(!has(&tree, Role::Button, button), "no button {button} on the row");
    }
    assert_eq!(
        menu_of(&mut window, &state, path),
        [words::MARK_AS_RESOLVED, words::TAKE_INCOMING, words::TAKE_CURRENT]
    );
    std::fs::write(repository.path().join(path), "both\n").unwrap();
    let intents = window.click(&state, Role::MenuItem, words::MARK_AS_RESOLVED);
    super::conflicts::to_the_end(&driver, &mut state, intents);
    assert_eq!(repository.git(&["diff", "--cached", "--name-only"]).trim(), path);
    assert_eq!(repository.git(&["ls-files", "--unmerged"]).trim(), "");
}

#[test]
fn changes_pane__in_a_column_too_narrow_for_the_buttons_of_the_heading_the_others_are_in_the_menu_at_its_right() {
    use views::details::{FileLayout, FilesIntent};
    use views::graph::{GraphIntent, Pane};
    let repository = three_changed();
    let _ = repository.git(&["add", "b.txt"]);
    let (mut state, driver) = changes_page(&repository);
    let mut window = Window::new(ThemeName::Dark, WIDE);
    let heading = [
        words::DISCARD_ALL,
        words::STASH_ALL,
        words::STASH_STAGED,
        words::OPEN_ALL_CHANGES,
        words::VIEW_AS_TREE,
    ];
    let (_, tree) = window.draw(&state, &[]);
    for button in heading {
        assert!(
            has(&tree, Role::Button, button),
            "{button} is on the heading of the whole page"
        );
    }
    assert!(!has(&tree, Role::Button, words::MORE_FOR_ALL));
    let _ = choose(&driver, &mut state, &mut window, (words::OPEN_CHANGES, "a.txt"));
    driver.intent(
        &mut state,
        Intent::Graph(GraphIntent::SetPane(Pane::ChangesColumn, Some(240.0))),
    );
    let (_, tree) = window.draw(&state, &[]);
    assert!(has(&tree, Role::Button, words::MORE_FOR_ALL));
    assert!(
        has(&tree, Role::Button, words::OPEN_ALL_CHANGES),
        "the last to leave stays"
    );
    let left: Vec<&str> = heading
        .into_iter()
        .filter(|button| !has(&tree, Role::Button, button))
        .collect();
    assert!(left.contains(&words::VIEW_AS_TREE), "the first to leave left: {left:?}");
    let _ = window.click(&state, Role::Button, words::MORE_FOR_ALL);
    let (_, tree) = window.draw(&state, &[]);
    assert_eq!(
        menu_items(&tree),
        left,
        "the menu has the buttons that left, in their order"
    );
    let intents = window.click(&state, Role::MenuItem, words::VIEW_AS_TREE);
    let as_tree = Intent::Changes(ChangesIntent::Files(FilesIntent::Layout(FileLayout::Tree)));
    assert!(intents.contains(&as_tree), "{intents:?}");
}
