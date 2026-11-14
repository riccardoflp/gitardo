//! What the Merge page hands over to the File page: `F4` and **Jump to source** open the file at the line of the result a
//! line of a column stands for, and a page closed with choices not saved shows them in the File page.

use app::store::{Message, State};
use model::merge_view::{Choice, Column};
use render::ThemeName;
use testkit::Repository;
use ui::{InputEvent, Key, Modifiers, NamedKey};
use views::Intent;
use views::merge_view::{MergeIntent, MergePageIntent};
use views::page::Page;

use super::diff::{f7, press};
use super::merge::{FILE, MERGE, conflicts_at, first_conflict, on_disk, opened, page_intent, shown};
use super::support::{Window, run};

/// `F4`, with no key held with it.
fn f4() -> InputEvent {
    super::diff::key(Key::Named(NamedKey::Function(4)), Modifiers::NONE)
}

/// Fifty lines, `line 1` to `line 50`, with lines added after some and others written another way.
fn fifty(added_after: &[(usize, &[&str])], changed: &[(usize, &str)]) -> String {
    let mut text = String::new();
    for line in 1..=50 {
        let written = changed
            .iter()
            .find(|(place, _)| *place == line)
            .map(|(_, changed)| (*changed).to_owned());
        text.push_str(&written.unwrap_or_else(|| format!("line {line}")));
        text.push('\n');
        for (_, lines) in added_after.iter().filter(|(place, _)| *place == line) {
            for added in *lines {
                text.push_str(added);
                text.push('\n');
            }
        }
    }
    text
}

/// A merge of `feature` into `main` stopped on line 48 of `src/app.rs`, `feature` having added two lines after line 10.
fn two_lines_added_above() -> Repository {
    let mut repository = Repository::new();
    let _base = repository.commit("Add the app", &[(FILE, &fifty(&[], &[]))]);
    repository.branch("feature");
    let _main = repository.commit(
        "Change the app on main",
        &[(FILE, &fifty(&[], &[(48, "line 48 on main")]))],
    );
    repository.checkout("feature");
    let added: &[&str] = &["added 1", "added 2"];
    let feature = fifty(&[(10, added)], &[(48, "line 48 on feature")]);
    let _feature = repository.commit("Change the app on feature", &[(FILE, &feature)]);
    repository.checkout("main");
    let output = repository
        .command("git")
        .args(["merge", "--no-ff", "--no-edit", "feature"])
        .output()
        .unwrap();
    assert!(!output.status.success(), "the merge stops on a conflict");
    repository
}

/// The File page shown: its path, the line it was brought to, from 1, whether it has edits not saved, and its text.
fn file_page_shown(state: &State) -> (String, Option<usize>, bool, String) {
    let Some(Page::File(id)) = state.page_shown() else {
        panic!("no File page is shown: {:?}", state.page_shown());
    };
    let page = state.file_pages.pages.iter().find(|page| page.id == id).unwrap();
    let text = page.document().unwrap().buffer().text().to_owned();
    (
        page.path.clone(),
        page.reveal.map(|(_, line)| line + 1),
        page.is_unsaved(),
        text,
    )
}

/// Puts the cursor on a line of a column, then presses `F4`, and gives what it asks to the store.
fn f4_on(state: &mut State, at: (Column, usize)) -> Vec<Intent> {
    page_intent(state, MergePageIntent::Cursor(at.0, at.1));
    let mut window = Window::new(ThemeName::Dark, MERGE);
    let (intents, _) = window.draw(state, &[f4()]);
    for intent in intents.clone() {
        let _effects = run(state, Message::Intent(intent));
    }
    intents
}

#[test]
fn merge_view_open_at_line__f4_on_line_40_of_the_current_side_below_two_lines_added_opens_the_file_page_at_line_42() {
    let repository = two_lines_added_above();
    let mut state = opened(&repository);
    let id = shown(&state).id;
    let intents = f4_on(&mut state, (Column::Current, 39));
    let asked = Intent::MergeView(MergeIntent::Page(id, MergePageIntent::OpenAtLine(Column::Current, 39)));
    assert_eq!(intents, [asked]);
    let (path, line, is_unsaved, _) = file_page_shown(&state);
    assert_eq!((path.as_str(), line, is_unsaved), (FILE, Some(42), false));
}

#[test]
fn merge_view_open_at_line__f4_inside_the_incoming_side_of_a_marked_conflict_opens_the_file_page_after_the_separator() {
    let repository = conflicts_at(&[10]);
    let mut state = opened(&repository);
    let _intents = f4_on(&mut state, (Column::Incoming, 9));
    let separator = on_disk(&repository).lines().position(|line| line == "=======").unwrap();
    let (_, line, _, text) = file_page_shown(&state);
    assert_eq!(line, Some(separator + 2));
    assert_eq!(text.lines().nth(separator + 1), Some("line 10 on feature"));
}

#[test]
fn merge_view_open_at_line__without_a_cursor_f4_opens_the_file_at_the_conflict_gone_to() {
    let repository = conflicts_at(&[10]);
    let mut state = opened(&repository);
    let mut window = Window::new(ThemeName::Dark, MERGE);
    let _tree = press(&mut window, &mut state, &[f7(Modifiers::NONE)]);
    let _tree = press(&mut window, &mut state, &[f4()]);
    let marker = on_disk(&repository)
        .lines()
        .position(|line| line.starts_with("<<<<<<<"))
        .unwrap();
    assert_eq!(file_page_shown(&state).1, Some(marker + 1));
}

#[test]
fn merge_view_open_at_line__with_a_choice_not_saved_the_file_page_holds_it_unsaved() {
    let repository = conflicts_at(&[10]);
    let mut state = opened(&repository);
    let place = first_conflict(&state);
    page_intent(&mut state, MergePageIntent::Take(place, Choice::Incoming));
    let result = shown(&state).result.clone();
    let _intents = f4_on(&mut state, (Column::Result, 9));
    let (_, line, is_unsaved, text) = file_page_shown(&state);
    assert_eq!((line, is_unsaved), (Some(10), true));
    assert_eq!(text, result);
    assert!(on_disk(&repository).contains("<<<<<<<"), "nothing is saved");
}

#[test]
fn merge_view_close__choosing_incoming_and_closing_the_page_opens_the_file_page_unsaved_with_the_choice() {
    let repository = conflicts_at(&[10]);
    let mut state = opened(&repository);
    let _tree = press(
        &mut Window::new(ThemeName::Dark, MERGE),
        &mut state,
        &[f7(Modifiers::NONE)],
    );
    let place = first_conflict(&state);
    page_intent(&mut state, MergePageIntent::Take(place, Choice::Incoming));
    let result = shown(&state).result.clone();
    let _effects = run(&mut state, Message::Intent(Intent::ClosePage));
    assert_eq!(state.merge_pages.pages.len(), 0, "the Merge page is closed");
    let (path, line, is_unsaved, text) = file_page_shown(&state);
    assert_eq!((path.as_str(), line, is_unsaved), (FILE, Some(10), true));
    assert_eq!(text, result);
    assert!(
        text.contains("line 10 on feature") && !text.contains("<<<<<<<"),
        "{text}"
    );
    assert!(on_disk(&repository).contains("<<<<<<<"), "nothing is saved");
}

#[test]
fn merge_view_close__closing_a_page_with_nothing_unsaved_opens_nothing() {
    let repository = conflicts_at(&[10]);
    let mut state = opened(&repository);
    let _effects = run(&mut state, Message::Intent(Intent::ClosePage));
    assert_eq!(state.merge_pages.pages.len(), 0, "the Merge page is closed");
    assert_eq!(state.file_pages.pages.len(), 0, "no File page is opened");
    assert!(!matches!(state.page_shown(), Some(Page::File(_))));
}
