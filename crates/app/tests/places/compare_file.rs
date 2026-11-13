//! A file of the inspector compared with a branch from its context menu (`details.compare-file-with`): the Diff page
//! it opens, and the picker put away with `Escape`.

use app::store::{Message, State};
use model::contract::Answer;
use platform::accesskit::{Role, TreeUpdate};
use render::{Point, ThemeName};
use ui::{InputEvent, Key, KeyEvent, Modifiers, NamedKey, PointerButton, PointerButtonEvent};
use views::Intent;
use views::feature::FeatureIntent;
use views::page::Page;

use super::compare::{repository, until_ended};
use super::inspector::{TALL, select};
use super::pointer::{middle, moved, rect as rect_of};
use super::showing;
use super::support::{OperationsDriver, Window};

/// The middle of the lowest thing of the window that says some words.
fn row(tree: &TreeUpdate, name: &str) -> Point {
    let found = tree
        .nodes
        .iter()
        .filter(|(_, node)| node.value() == Some(name) || node.label() == Some(name))
        .filter_map(|(_, node)| node.bounds().map(rect_of))
        .max_by(|first, second| first.y.total_cmp(&second.y));
    middle(found.unwrap_or_else(|| panic!("nothing says {name}")))
}

/// Opens the context menu of the file of the inspector named so, and chooses **Compare with…**; then waits for the
/// picker of the side.
fn compare_file(window: &mut Window, state: &mut State, driver: &OperationsDriver, name: &str) {
    let (_, tree) = window.draw(state, &[]);
    let point = row(&tree, name);
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
    let intents = window.click(state, Role::MenuItem, views::text::details::COMPARE_WITH);
    assert!(!intents.is_empty(), "the entry can be used");
    for intent in intents {
        driver.intent(state, intent);
    }
    loop {
        if let Message::FeatureAsks(_) = driver.next(state) {
            break;
        }
    }
}

#[test]
fn details_compare_file_with__the_menu_of_a_file_of_the_inspector_opens_the_diff_page_of_main_and_the_commit() {
    let repository = repository();
    let mut state = showing(repository.path().to_path_buf());
    let commit = select(&mut state, "Add c");
    let mut window = Window::new(ThemeName::Dark, TALL);
    let driver = OperationsDriver::new();
    compare_file(&mut window, &mut state, &driver, "c.txt");
    let answer = Answer::Choice(String::from("ref:main"));
    driver.intent(&mut state, Intent::Feature(FeatureIntent::Answer(Some(answer))));
    until_ended(&driver, &mut state);
    assert!(
        matches!(state.page_shown(), Some(Page::Diff(_))),
        "{:?}",
        state.page_shown()
    );
    let page = state.diff_pages.pages.last().expect("a Diff page");
    assert_eq!(page.title, format!("c.txt (main \u{2194} {})", commit.short()));
}

#[test]
fn details_compare_file_with__escape_in_the_picker_opens_nothing() {
    let repository = repository();
    let mut state = showing(repository.path().to_path_buf());
    let _commit = select(&mut state, "Add c");
    let mut window = Window::new(ThemeName::Dark, TALL);
    let driver = OperationsDriver::new();
    compare_file(&mut window, &mut state, &driver, "c.txt");
    let escape = InputEvent::KeyPressed(KeyEvent::press(Key::Named(NamedKey::Escape), Modifiers::NONE));
    let (intents, _) = window.draw(&state, &[escape]);
    assert!(
        intents.contains(&Intent::Feature(FeatureIntent::Answer(None))),
        "{intents:?}"
    );
    for intent in intents {
        driver.intent(&mut state, intent);
    }
    until_ended(&driver, &mut state);
    assert_eq!(state.page_shown(), None);
    assert_eq!(state.diff_pages.pages.len(), 0);
}
