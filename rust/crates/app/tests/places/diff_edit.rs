//! The sides of the Diff page as places to type and to point at, in repositories made for each test: the working tree
//! side edited where it is shown and saved with `Ctrl+S`, the same text in the File page of the file, a diff of the
//! index read only, a change reverted as an edit that `Ctrl+Z` takes back, and `F4` opening the File page at the line
//! of the caret.

use std::time::Duration;

use app::store::feature::Ended;
use app::store::{FileContent, Message, State};
use model::diff::LineChangeKind;
use platform::accesskit::{Role, TreeUpdate};
use render::{Point, Rect, ThemeName};
use testkit::Repository;
use ui::{InputEvent, Key, KeyEvent, Modifiers, NamedKey};
use views::Intent;
use views::diff::{DiffFile, DiffTarget, DiffVersion, PageLine};
use views::feature::FeatureIntent;
use views::file_view::{FileTarget, FileViewIntent};
use views::text::diff as words;
use views::text::file_view as file_words;

use super::diff::{edited, head_to_working_tree, open, repository_with_three_changes, shown, text};
use super::file_view::{open as open_file, shown as file_shown, step, text as file_text, type_text};
use super::pointer::{apply, click_at, rect, rows_drawn};
use super::showing;
use super::support::{OperationsDriver, Window, says, shortcut};

/// A window tall enough for the 30 lines of the files of these tests.
const TALL: (u32, u32) = (1000, 1000);

/// The page of a file between `HEAD` and the working tree, drawn once.
fn page_of(repository: &Repository, target: DiffTarget) -> (State, Window) {
    let mut state = showing(repository.path().to_path_buf());
    state.settings.collapse_unchanged = false;
    open(&mut state, target);
    let mut window = Window::new(ThemeName::Dark, TALL);
    let _ = step(&mut window, &mut state, &[]);
    (state, window)
}

/// The two sides as the screen reader is told of them, the left one first.
fn sides(tree: &TreeUpdate) -> (Rect, Rect) {
    let mut found: Vec<Rect> = tree
        .nodes
        .iter()
        .filter(|(_, node)| node.role() == Role::TextInput)
        .filter_map(|(_, node)| node.bounds().map(rect))
        // The message box of the **Changes** section of the side column is an input box too.
        .filter(|found| !super::pointer::is_in_changes_section(tree, *found))
        .collect();
    found.sort_by(|one, other| one.x.total_cmp(&other.x));
    match found.as_slice() {
        [left, right] => (*left, *right),
        other => panic!("two sides, not {other:?}"),
    }
}

/// The point at the start of a line of a side, by its number from 0.
pub(super) fn on_line(window: &mut Window, state: &State, (is_right, line): (bool, usize)) -> Point {
    let (_, tree) = window.draw(state, &[]);
    let (left, right) = sides(&tree);
    let page = shown(state);
    let row = text(page)
        .aligned
        .rows
        .iter()
        .position(|aligned| {
            let side = if is_right { &aligned.right } else { &aligned.left };
            side.as_ref().is_some_and(|shown| shown.line == line)
        })
        .unwrap();
    let place = page
        .lines
        .iter()
        .position(|shown| *shown == PageLine::Row(0, row))
        .unwrap();
    let drawn = rows_drawn(&tree);
    let row_rect = drawn.get(place).unwrap();
    let side = if is_right { right } else { left };
    Point::new(side.x + 1.0, row_rect.y + row_rect.height * 0.5)
}

/// Clicks a line of a side and hands what the click asks to the store.
fn click_line(window: &mut Window, state: &mut State, at: (bool, usize)) {
    let point = on_line(window, state, at);
    let intents = click_at(window, state, point, 1);
    apply(state, intents);
}

/// A key pressed with no modifier.
fn pressed(named: NamedKey) -> InputEvent {
    InputEvent::KeyPressed(KeyEvent::press(Key::Named(named), Modifiers::NONE))
}

/// Draws a frame with some input and runs the operation it asks through the operations thread; gives how it ended and
/// the lines it logged.
pub(super) fn run_operation(window: &mut Window, state: &mut State, events: &[InputEvent]) -> (Ended, Vec<String>) {
    let driver = OperationsDriver::new();
    let (intents, _) = window.draw(state, events);
    for intent in intents {
        driver.intent(state, intent);
    }
    assert!(state.operations.running.is_some(), "an operation runs");
    loop {
        match driver.next(state) {
            Message::FeatureConfirms(_) => driver.intent(state, Intent::Feature(FeatureIntent::Confirm(true))),
            Message::FeatureEnded { ended, log } => {
                return (
                    ended,
                    log.into_iter()
                        .filter(super::support::is_ran)
                        .map(|entry| entry.line)
                        .collect(),
                );
            }
            _ => {}
        }
    }
}

/// The text of the file the File pages hold for a path, shown or not.
pub(super) fn held_text(state: &State, path: &str) -> String {
    let page = state.file_pages.pages.iter().find(|page| page.path == path).unwrap();
    match &page.content {
        FileContent::Text(document) => document.buffer().text().to_owned(),
        other => panic!("the file is held as text, not {other:?}"),
    }
}

#[test]
fn diff_edit__a_line_typed_on_the_right_side_shows_as_an_added_change_within_half_a_second_and_ctrl_s_writes_it() {
    let repository = repository_with_three_changes();
    let (mut state, mut window) = page_of(&repository, head_to_working_tree("app.rs"));
    click_line(&mut window, &mut state, (true, 9));
    let _ = step(
        &mut window,
        &mut state,
        &[pressed(NamedKey::End), pressed(NamedKey::Enter)],
    );
    type_text(&mut window, &mut state, "added");
    assert_eq!(text(shown(&state)).right_line(10), "added", "the line shows at once");
    window.now += Duration::from_millis(300);
    let _ = step(&mut window, &mut state, &[]);
    let changes = &text(shown(&state)).changes;
    assert_eq!(changes.len(), 4, "{changes:?}");
    assert!(
        changes
            .iter()
            .any(|change| change.kind() == LineChangeKind::Added && change.right == (10..11))
    );
    let (ended, log) = run_operation(&mut window, &mut state, &[shortcut('s', Modifiers::CONTROL)]);
    assert!(matches!(ended, Ended::Ran { .. }), "{ended:?}");
    let path = repository.path().join("app.rs");
    assert_eq!(log, [format!("write {}", path.display())]);
    let on_disk = std::fs::read_to_string(&path).unwrap();
    assert!(on_disk.contains("line 10\nadded\nline 11\n"), "{on_disk}");
}

#[test]
fn diff_edit__an_edit_in_the_file_page_shows_in_the_diff_page_and_the_other_way_round() {
    let repository = repository_with_three_changes();
    let (mut state, mut window) = page_of(&repository, head_to_working_tree("app.rs"));
    click_line(&mut window, &mut state, (true, 0));
    type_text(&mut window, &mut state, "x");
    assert_eq!(held_text(&state, "app.rs"), format!("x{}", edited(30, &[5, 15, 25])));
    open_file(&mut state, FileTarget::working_tree("app.rs"));
    let _ = step(&mut window, &mut state, &[]);
    assert_eq!(
        file_text(&state),
        format!("x{}", edited(30, &[5, 15, 25])),
        "the File page has the edit"
    );
    assert!(file_shown(&state).is_unsaved());
    type_text(&mut window, &mut state, "y");
    let diff = state.diff_pages.pages.first().unwrap();
    let Some(views::diff::FileDiff::Text(diff_text)) = diff.diffs.first() else {
        panic!("the diff is read")
    };
    assert!(diff_text.right.starts_with("yxline 1\n"), "{}", diff_text.right);
}

#[test]
fn diff_edit__in_a_diff_of_the_index_against_head_typing_does_nothing() {
    let repository = repository_with_three_changes();
    let _ = repository.git(&["add", "app.rs"]);
    let target = DiffTarget::file(DiffVersion::Head, DiffVersion::Index, DiffFile::same("app.rs"));
    let (mut state, mut window) = page_of(&repository, target);
    click_line(&mut window, &mut state, (true, 0));
    type_text(&mut window, &mut state, "x");
    let _ = step(&mut window, &mut state, &[pressed(NamedKey::Enter)]);
    assert_eq!(text(shown(&state)).right, edited(30, &[5, 15, 25]));
    assert_eq!(state.file_pages.pages.len(), 0, "no text is held");
    let (_, tree) = window.draw(&state, &[]);
    assert!(!says(&tree, file_words::UNSAVED_DOT));
}

/// Clicks the **Revert this change** button of a change, the first from the top being 0, and runs the save it asks.
fn revert(window: &mut Window, state: &mut State, nth: usize) -> (Ended, Vec<String>) {
    let (_, tree) = window.draw(state, &[]);
    let mut buttons: Vec<Rect> = tree
        .nodes
        .iter()
        .filter(|(_, node)| node.role() == Role::Button && node.label() == Some(words::REVERT_CHANGE))
        .filter_map(|(_, node)| node.bounds().map(rect))
        .collect();
    buttons.sort_by(|one, other| one.y.total_cmp(&other.y));
    let button = buttons.get(nth).unwrap();
    let point = Point::new(button.x + button.width * 0.5, button.y + button.height * 0.5);
    let _ = window.draw(state, &[super::pointer::moved(point)]);
    let press = ui::PointerButtonEvent {
        button: ui::PointerButton::Primary,
        position: point,
        modifiers: Modifiers::NONE,
        click_count: 1,
    };
    run_operation(
        window,
        state,
        &[InputEvent::PointerPressed(press), InputEvent::PointerReleased(press)],
    )
}

#[test]
fn diff_revert_change__reverting_a_change_from_the_page_writes_the_file_with_the_line_head_has() {
    let repository = repository_with_three_changes();
    let (mut state, mut window) = page_of(&repository, head_to_working_tree("app.rs"));
    let (ended, log) = revert(&mut window, &mut state, 1);
    assert!(matches!(ended, Ended::Ran { .. }), "{ended:?}");
    assert_eq!(log, [format!("write {}", repository.path().join("app.rs").display())]);
    assert_eq!(
        std::fs::read_to_string(repository.path().join("app.rs")).unwrap(),
        edited(30, &[5, 25])
    );
    assert_eq!(text(shown(&state)).aligned.changes.len(), 2);
}

#[test]
fn diff_revert_change__ctrl_z_right_after_brings_the_change_back_and_the_page_shows_the_file_unsaved() {
    let repository = repository_with_three_changes();
    let (mut state, mut window) = page_of(&repository, head_to_working_tree("app.rs"));
    let _ = revert(&mut window, &mut state, 1);
    click_line(&mut window, &mut state, (true, 0));
    let _ = step(&mut window, &mut state, &[shortcut('z', Modifiers::CONTROL)]);
    assert_eq!(held_text(&state, "app.rs"), edited(30, &[5, 15, 25]));
    let tree = step(&mut window, &mut state, &[]);
    assert!(says(&tree, file_words::UNSAVED_DOT), "the page shows the file unsaved");
    assert_eq!(
        std::fs::read_to_string(repository.path().join("app.rs")).unwrap(),
        edited(30, &[5, 25])
    );
}

/// The File page of `app.rs` in the working tree at a line, from 1.
fn at_line(line: usize) -> FileTarget {
    FileTarget {
        line: Some(line),
        ..FileTarget::working_tree("app.rs")
    }
}

/// The File page `F4` asks for among some intents.
fn file_asked(intents: &[Intent]) -> Option<&FileTarget> {
    intents.iter().find_map(|intent| match intent {
        Intent::FileView(FileViewIntent::Open(target)) => Some(target),
        _ => None,
    })
}

#[test]
fn diff_open_at_line__f4_on_line_30_of_the_right_side_opens_the_file_page_at_line_30() {
    let repository = repository_with_three_changes();
    let (mut state, mut window) = page_of(&repository, head_to_working_tree("app.rs"));
    click_line(&mut window, &mut state, (true, 29));
    let (intents, _) = window.draw(&state, &[pressed(NamedKey::Function(4))]);
    assert_eq!(file_asked(&intents), Some(&at_line(30)));
    apply(&mut state, intents);
    let page = file_shown(&state);
    let FileContent::Text(document) = &page.content else {
        panic!("text")
    };
    assert_eq!(document.buffer().position(page.cursor.head).line, 29);
}

#[test]
fn diff_open_at_line__f4_on_a_removed_line_of_the_left_side_opens_it_at_the_line_above() {
    let mut repository = Repository::new();
    let _ = repository.commit("Add the app", &[("app.rs", &edited(30, &[]))]);
    let without_10 = edited(30, &[]).replace("line 10\n", "");
    std::fs::write(repository.path().join("app.rs"), without_10).unwrap();
    let (mut state, mut window) = page_of(&repository, head_to_working_tree("app.rs"));
    click_line(&mut window, &mut state, (false, 9));
    let (intents, _) = window.draw(&state, &[pressed(NamedKey::Function(4))]);
    assert_eq!(file_asked(&intents), Some(&at_line(9)));
}
