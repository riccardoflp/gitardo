//! The lines selected in the Diff page (`diff.select-lines`) and what is done with them, in repositories made for each
//! test: rows selected by a drag, by `Shift` with the arrows and by the text selected, cleared by `Escape` and by an
//! edit, and staged, unstaged or reverted from the keys of the page (`diff.stage-lines`, `diff.unstage-lines`,
//! `diff.revert-lines`), git applying the patch of the lines selected (`diff.lines-patch`).

use app::store::State;
use app::store::feature::Ended;
use platform::accesskit::{Role, TreeUpdate};
use render::ThemeName;
use testkit::Repository;
use ui::{InputEvent, Key, KeyEvent, Modifiers, NamedKey};
use views::Intent;
use views::diff::{DiffEditIntent, DiffIntent, DiffTarget};
use views::text::diff as words;

use super::changes_menu::right_click;
use super::diff::{edited, head_to_working_tree, open, shown};
use super::diff_edit::{held_text, on_line, run_operation};
use super::file_view::step;
use super::pointer::{apply, click_at, drag};
use super::showing;
use super::support::{Window, shortcut};

/// A window tall enough for the 30 lines of the files of these tests.
const TALL: (u32, u32) = (1000, 1000);

/// The line of `git apply` that stages or unstages lines.
const APPLY: &str = "git apply --cached --unidiff-zero --whitespace=nowarn -";

/// A repository with `app.rs` of 30 lines committed, and the working tree as given.
fn repository_with(working_tree: &str) -> Repository {
    let mut repository = Repository::new();
    let _ = repository.commit("Add the app", &[("app.rs", &edited(30, &[]))]);
    std::fs::write(repository.path().join("app.rs"), working_tree).unwrap();
    repository
}

/// The page of a target, every row shown, drawn once.
fn page_of(repository: &Repository, target: DiffTarget) -> (State, Window) {
    let mut state = showing(repository.path().to_path_buf());
    state.settings.collapse_unchanged = false;
    open(&mut state, target);
    let mut window = Window::new(ThemeName::Dark, TALL);
    let _ = step(&mut window, &mut state, &[]);
    (state, window)
}

/// The rows selected on a side that is not edited.
fn selected(state: &State) -> Option<std::ops::Range<usize>> {
    shown(state).selection.as_ref().map(|selection| selection.rows.rows())
}

/// The items of the context menu of the rows at a line of the left side, each with whether it can be chosen.
fn menu_at(window: &mut Window, state: &State, line: usize) -> Vec<(String, bool)> {
    let point = on_line(window, state, (false, line));
    let tree: TreeUpdate = right_click(window, state, point);
    let items = tree
        .nodes
        .iter()
        .filter(|(_, node)| node.role() == Role::MenuItem)
        .filter_map(|(_, node)| node.label().map(|label| (label.to_owned(), !node.is_disabled())))
        .collect();
    let _ = window.draw(state, &[key(NamedKey::Escape, Modifiers::NONE)]);
    items
}

/// A key pressed with some modifiers.
fn key(named: NamedKey, modifiers: Modifiers) -> InputEvent {
    InputEvent::KeyPressed(KeyEvent::press(Key::Named(named), modifiers))
}

/// Selects the rows from a line of the left side to another, as `Shift` with a click does.
fn select_left(window: &mut Window, state: &mut State, (from, to): (usize, usize)) {
    let start = on_line(window, state, (false, from));
    let intents = click_at(window, state, start, 1);
    apply(state, intents);
    let end = on_line(window, state, (false, to));
    let _ = window.draw(state, &[super::pointer::moved(end)]);
    let press = ui::PointerButtonEvent {
        button: ui::PointerButton::Primary,
        position: end,
        modifiers: Modifiers::SHIFT,
        click_count: 1,
    };
    let (intents, _) = window.draw(
        state,
        &[InputEvent::PointerPressed(press), InputEvent::PointerReleased(press)],
    );
    apply(state, intents);
}

/// What `git diff --cached` says of the repository, with no line of context.
fn staged(repository: &Repository) -> String {
    repository.git(&["diff", "--cached", "--unified=0"])
}

#[test]
fn diff_select_lines__dragging_over_the_numbers_of_lines_10_to_14_on_the_left_selects_their_rows_and_offers_stage_lines()
 {
    let repository = repository_with(&edited(30, &[12]));
    let (mut state, mut window) = page_of(&repository, head_to_working_tree("app.rs"));
    let from = on_line(&mut window, &state, (false, 9));
    let to = on_line(&mut window, &state, (false, 13));
    let intents = drag(&mut window, &state, from, to);
    apply(&mut state, intents);
    assert_eq!(selected(&state), Some(9..14));
    let items = menu_at(&mut window, &state, 11);
    assert!(items.contains(&(words::STAGE_LINES.to_owned(), true)), "{items:?}");
    assert!(items.contains(&(words::REVERT_LINES.to_owned(), true)), "{items:?}");
    assert_eq!(selected(&state), Some(9..14), "the menu keeps the selection");
}

#[test]
fn diff_select_lines__shift_down_three_times_from_line_20_of_head_selects_four_rows_and_no_text() {
    let repository = repository_with(&edited(30, &[22]));
    let (mut state, mut window) = page_of(&repository, head_to_working_tree("app.rs"));
    let point = on_line(&mut window, &state, (false, 19));
    let intents = click_at(&mut window, &state, point, 1);
    apply(&mut state, intents);
    for _ in 0..3 {
        let _ = step(&mut window, &mut state, &[key(NamedKey::ArrowDown, Modifiers::SHIFT)]);
    }
    assert_eq!(selected(&state), Some(19..23));
    assert_eq!(
        format!("{:?}", shown(&state).caret),
        "Some(Line { file: 0, side: Left, row: 22 })",
        "the line chosen follows, and no text is selected"
    );
}

#[test]
fn diff_select_lines__the_new_text_of_a_changed_line_selected_selects_its_row_and_stages_it() {
    let repository = repository_with(&edited(30, &[5, 15]));
    let (mut state, mut window) = page_of(&repository, head_to_working_tree("app.rs"));
    let point = on_line(&mut window, &state, (true, 14));
    // A double click selects a word of the new text of line 15.
    let point = render::Point::new(point.x + 30.0, point.y);
    let intents = click_at(&mut window, &state, point, 2);
    apply(&mut state, intents);
    let (ended, log) = run_operation(&mut window, &mut state, &[shortcut('s', Modifiers::ALT)]);
    assert!(matches!(ended, Ended::Ran { .. }), "{ended:?}");
    assert_eq!(log, [APPLY]);
    let cached = staged(&repository);
    assert!(cached.contains("-line 15\n+line 15 changed\n"), "{cached}");
    assert!(!cached.contains("line 5 changed"), "{cached}");
}

#[test]
fn diff_select_lines__escape_clears_the_selection_and_an_edit_that_takes_away_a_selected_line_clears_it_too() {
    let repository = repository_with(&edited(30, &[12]));
    let (mut state, mut window) = page_of(&repository, head_to_working_tree("app.rs"));
    select_left(&mut window, &mut state, (9, 13));
    assert_eq!(selected(&state), Some(9..14));
    let _ = step(&mut window, &mut state, &[key(NamedKey::Escape, Modifiers::NONE)]);
    assert_eq!(selected(&state), None);
    select_left(&mut window, &mut state, (9, 13));
    let id = shown(&state).id;
    let line = ui::editor::Position { line: 11, column: 0 };
    let next = ui::editor::Position { line: 12, column: 0 };
    let commands = vec![
        ui::editor::Command::PlaceAt(line, false),
        ui::editor::Command::PlaceAt(next, true),
        ui::editor::Command::DeleteSelection,
    ];
    let edit = DiffEditIntent::Text {
        file: 0,
        commands,
        at: std::time::Duration::from_secs(5),
    };
    apply(&mut state, vec![Intent::Diff(DiffIntent::Edit(id, edit))]);
    assert_eq!(selected(&state), None);
}

#[test]
fn diff_select_lines__rows_with_no_changed_line_give_the_three_items_greyed_out() {
    let repository = repository_with(&edited(30, &[25]));
    let _ = repository.git(&["add", "app.rs"]);
    std::fs::write(repository.path().join("app.rs"), edited(30, &[20, 25])).unwrap();
    let (mut state, mut window) = page_of(&repository, head_to_working_tree("app.rs"));
    select_left(&mut window, &mut state, (2, 5));
    let items = menu_at(&mut window, &state, 3);
    for name in [words::STAGE_LINES, words::UNSTAGE_LINES, words::REVERT_LINES] {
        assert!(items.contains(&(name.to_owned(), false)), "{name} greyed: {items:?}");
    }
}

#[test]
fn diff_stage_lines__staging_the_second_and_third_of_five_lines_added_leaves_them_alone_in_the_index() {
    let mut lines: Vec<String> = edited(30, &[]).lines().map(str::to_owned).collect();
    for (offset, added) in ["one", "two", "three", "four", "five"].iter().enumerate() {
        lines.insert(12 + offset, (*added).to_owned());
    }
    let repository = repository_with(&(lines.join("\n") + "\n"));
    let (mut state, mut window) = page_of(&repository, head_to_working_tree("app.rs"));
    let start = on_line(&mut window, &state, (true, 13));
    let intents = click_at(&mut window, &state, start, 1);
    apply(&mut state, intents);
    // The text selected from the start of `two` to the start of the line after `three` takes their two rows.
    for _ in 0..2 {
        let _ = step(&mut window, &mut state, &[key(NamedKey::ArrowDown, Modifiers::SHIFT)]);
    }
    let (ended, log) = run_operation(&mut window, &mut state, &[shortcut('s', Modifiers::ALT)]);
    assert!(matches!(ended, Ended::Ran { .. }), "{ended:?}");
    assert_eq!(log, [APPLY]);
    let cached = staged(&repository);
    let hunk: Vec<&str> = cached.lines().skip(4).collect();
    assert_eq!(hunk.len(), 3, "{cached}");
    assert!(
        hunk.first()
            .is_some_and(|header| header.starts_with("@@ -12,0 +13,2 @@")),
        "{cached}"
    );
    assert_eq!(hunk.get(1..), Some(&["+two", "+three"][..]));
}

#[test]
fn diff_unstage_lines__the_rows_of_a_staged_change_unstage_from_the_diff_of_the_working_tree() {
    let repository = repository_with(&edited(30, &[8, 20]));
    let _ = repository.git(&["add", "app.rs"]);
    let (mut state, mut window) = page_of(&repository, head_to_working_tree("app.rs"));
    select_left(&mut window, &mut state, (6, 8));
    let (ended, log) = run_operation(&mut window, &mut state, &[shortcut('u', Modifiers::ALT)]);
    assert!(matches!(ended, Ended::Ran { .. }), "{ended:?}");
    assert_eq!(log, [APPLY]);
    let cached = staged(&repository);
    assert!(!cached.contains("line 8 changed"), "{cached}");
    assert!(cached.contains("+line 20 changed"), "{cached}");
    assert_eq!(
        repository
            .git(&["diff", "--unified=0", "--", "app.rs"])
            .matches("line 8 changed")
            .count(),
        1
    );
}

#[test]
fn diff_revert_lines__reverting_rows_across_two_changes_reverts_both_in_one_edit_that_ctrl_z_takes_back() {
    let repository = repository_with(&edited(30, &[10, 12, 20]));
    let (mut state, mut window) = page_of(&repository, head_to_working_tree("app.rs"));
    select_left(&mut window, &mut state, (9, 11));
    let (ended, _) = run_operation(&mut window, &mut state, &[shortcut('r', Modifiers::ALT)]);
    assert!(matches!(ended, Ended::Ran { .. }), "{ended:?}");
    let on_disk = std::fs::read_to_string(repository.path().join("app.rs")).unwrap();
    assert_eq!(on_disk, edited(30, &[20]));
    assert_eq!(selected(&state), None);
    let right = on_line(&mut window, &state, (true, 0));
    let intents = click_at(&mut window, &state, right, 1);
    apply(&mut state, intents);
    let _ = step(&mut window, &mut state, &[shortcut('z', Modifiers::CONTROL)]);
    assert_eq!(
        held_text(&state, "app.rs"),
        edited(30, &[10, 12, 20]),
        "one Ctrl+Z takes both back"
    );
}

#[test]
fn diff_lines_patch__lines_added_after_a_last_line_with_no_line_break_take_its_change_and_git_applies_them() {
    let mut repository = Repository::new();
    let _ = repository.commit("Add the notes", &[("notes.txt", "one\ntwo")]);
    std::fs::write(repository.path().join("notes.txt"), "one\ntwo\nthree\n").unwrap();
    let (mut state, mut window) = page_of(&repository, head_to_working_tree("notes.txt"));
    let point = on_line(&mut window, &state, (true, 2));
    let point = render::Point::new(point.x + 30.0, point.y);
    let intents = click_at(&mut window, &state, point, 2);
    apply(&mut state, intents);
    let (ended, _) = run_operation(&mut window, &mut state, &[shortcut('s', Modifiers::ALT)]);
    assert!(matches!(ended, Ended::Ran { .. }), "{ended:?}");
    assert_eq!(repository.git(&["show", ":notes.txt"]), "one\ntwo\nthree\n");
}
