//! The Diff page drawn from the store, its files read and compared by the workers in repositories made for each test:
//! what it opens and refuses, the question before a very large file, the page read again when the index changes, and
//! `F7` from change to change and from file to file.

use std::time::Duration;

use app::store::{DiffPage, Message, State};
use model::CommitId;
use model::repo::Change;
use platform::accesskit::{Role, TreeUpdate};
use render::ThemeName;
use testkit::Repository;
use ui::{InputEvent, Key, KeyEvent, Modifiers, NamedKey};
use views::diff::{DiffFile, DiffIntent, DiffPageIntent, DiffTarget, DiffText, DiffVersion, FileDiff};
use views::page::Page;
use views::{Intent, Notice};

use super::commit_page::lines;
use super::inspector::ask;
use super::pointer::apply;
use super::showing;
use super::support::{self, Window, find, says};

/// The size of the window of the Diff pages.
pub const DIFF: (u32, u32) = (1000, 600);

/// Opens the Diff page of a target and runs the reads it starts.
pub fn open(state: &mut State, target: DiffTarget) {
    ask(state, Intent::Diff(DiffIntent::Open(Box::new(target))));
}

/// The diff of a file between `HEAD` and the working tree.
pub fn head_to_working_tree(path: &str) -> DiffTarget {
    DiffTarget::file(DiffVersion::Head, DiffVersion::WorkingTree, DiffFile::same(path))
}

/// The Diff page shown, which the test fails without.
pub fn shown(state: &State) -> &DiffPage {
    let Some(Page::Diff(id)) = state.page_shown() else {
        panic!("a Diff page is shown, not {:?}", state.page_shown());
    };
    state.diff_pages.pages.iter().find(|page| page.id == id).unwrap()
}

/// The diff of the first file of a page, read as text.
pub fn text(page: &DiffPage) -> &DiffText {
    let Some(FileDiff::Text(text)) = page.diffs.first() else {
        panic!("the file is read as text, not {:?}", page.diffs.first());
    };
    text
}

/// A key pressed with no modifier or with Shift.
pub fn key(key: Key, modifiers: Modifiers) -> InputEvent {
    InputEvent::KeyPressed(KeyEvent::press(key, modifiers))
}

/// `F7`, or `Shift+F7`.
pub fn f7(modifiers: Modifiers) -> InputEvent {
    key(Key::Named(NamedKey::Function(7)), modifiers)
}

/// Presses keys in a frame of the window, a second later than the frame before, and hands what they ask to the store.
pub fn press(window: &mut Window, state: &mut State, events: &[InputEvent]) -> TreeUpdate {
    window.now += Duration::from_secs(1);
    let (intents, _) = window.draw(state, events);
    apply(state, intents);
    window.draw(state, &[]).1
}

/// A text of so many lines, `line 1` to `line N`, with some of them written another way.
pub fn edited(count: usize, changed: &[usize]) -> String {
    (1..=count)
        .map(|line| {
            if changed.contains(&line) {
                format!("line {line} changed\n")
            } else {
                format!("line {line}\n")
            }
        })
        .collect()
}

/// A repository with `app.rs` of 30 lines committed, and lines 5, 15 and 25 changed in the working tree.
pub fn repository_with_three_changes() -> Repository {
    let mut repository = Repository::new();
    let _ = repository.commit("Add the app", &[("app.rs", &edited(30, &[]))]);
    std::fs::write(repository.path().join("app.rs"), edited(30, &[5, 15, 25])).unwrap();
    repository
}

/// Tells the store the index changed, and lets the read it asks for run.
fn index_changed(state: &mut State) {
    let changes = vec![Change::GitFolder("index".into())];
    let at = Duration::from_secs(10);
    let _ = support::run(state, Message::Changed { changes, at });
    let _ = support::run(
        state,
        Message::Wake {
            at: Duration::from_secs(20),
        },
    );
}

#[test]
fn diff_page__the_changes_of_an_edited_file_show_head_and_the_working_tree_titled_with_both() {
    let repository = repository_with_three_changes();
    let mut state = showing(repository.path().to_path_buf());
    open(&mut state, head_to_working_tree("app.rs"));
    let page = shown(&state);
    assert_eq!(page.title, "app.rs (HEAD \u{2194} Working tree)");
    assert_eq!(text(page).aligned.changes.len(), 3);
    let (_, tree) = Window::new(ThemeName::Dark, DIFF).draw(&state, &[]);
    assert!(says(&tree, "HEAD"));
    assert!(says(&tree, "Working tree"));
}

#[test]
fn diff_page__an_untracked_file_of_10_lines_shows_10_added_lines_and_an_empty_left_side() {
    let repository = repository_with_three_changes();
    std::fs::write(repository.path().join("notes.txt"), lines(10, "note")).unwrap();
    let mut state = showing(repository.path().to_path_buf());
    open(&mut state, head_to_working_tree("notes.txt"));
    let read = text(shown(&state));
    assert_eq!(read.aligned.rows.len(), 10);
    assert!(
        read.aligned
            .rows
            .iter()
            .all(|row| row.left.is_none() && row.right.is_some())
    );
    assert_eq!(read.aligned.changes.len(), 1);
    assert_eq!(read.aligned.changes[0].kind, model::diff::LineChangeKind::Added);
    let (_, tree) = Window::new(ThemeName::Dark, DIFF).draw(&state, &[]);
    assert!(says(&tree, "Empty"));
}

#[test]
fn diff_page__a_file_with_a_nul_byte_refuses_with_this_file_looks_binary_and_opens_no_page() {
    let mut repository = Repository::new();
    let _ = repository.commit("Add the picture", &[("picture.bin", "head\0er\n")]);
    std::fs::write(repository.path().join("picture.bin"), "head\0er, changed\n").unwrap();
    let mut state = showing(repository.path().to_path_buf());
    open(&mut state, head_to_working_tree("picture.bin"));
    assert!(!matches!(state.page_shown(), Some(Page::Diff(_))));
    assert_eq!(state.diff_pages.pages, []);
    assert_eq!(state.notice, Some(Notice::DiffLooksBinary));
    let (_, tree) = Window::new(ThemeName::Dark, DIFF).draw(&state, &[]);
    assert!(says(&tree, "This file looks binary."));
}

#[test]
fn diff_page__two_files_of_60000_lines_ask_before_computing_continue_shows_and_cancel_opens_nothing() {
    let mut repository = Repository::new();
    let _ = repository.commit("Add the log", &[("log.txt", &edited(60_000, &[]))]);
    std::fs::write(repository.path().join("log.txt"), edited(60_000, &[10, 30_000, 59_990])).unwrap();
    let mut state = showing(repository.path().to_path_buf());
    let mut window = Window::new(ThemeName::Dark, DIFF);
    open(&mut state, head_to_working_tree("log.txt"));
    assert!(!matches!(state.page_shown(), Some(Page::Diff(_))));
    let (_, tree) = window.draw(&state, &[]);
    assert!(says(
        &tree,
        "This file is very large and the diff may be slow. Continue?"
    ));
    let intents = window.click(&state, Role::Button, "Continue");
    apply(&mut state, intents);
    assert_eq!(text(shown(&state)).aligned.changes.len(), 3);
    ask(&mut state, Intent::ClosePage);
    open(&mut state, head_to_working_tree("log.txt"));
    let intents = window.click(&state, Role::Button, "Cancel");
    apply(&mut state, intents);
    assert!(!matches!(state.page_shown(), Some(Page::Diff(_))));
    assert_eq!(state.diff_pages.pages, []);
    let (_, tree) = window.draw(&state, &[]);
    assert!(!says(&tree, "Continue"));
}

#[test]
fn diff_page__opening_the_same_diff_twice_shows_one_page() {
    let repository = repository_with_three_changes();
    let mut state = showing(repository.path().to_path_buf());
    open(&mut state, head_to_working_tree("app.rs"));
    ask(&mut state, Intent::ShowTimeline);
    open(&mut state, head_to_working_tree("app.rs"));
    let pages = state
        .pages
        .open
        .iter()
        .filter(|page| matches!(page, Page::Diff(_)))
        .count();
    assert_eq!(pages, 1);
    assert_eq!(state.diff_pages.pages.len(), 1);
    assert!(matches!(state.page_shown(), Some(Page::Diff(_))));
}

#[test]
fn diff_page__staging_a_file_while_its_diff_against_the_index_is_open_redraws_it_with_no_change() {
    let repository = repository_with_three_changes();
    let mut state = showing(repository.path().to_path_buf());
    let target = DiffTarget::file(DiffVersion::Index, DiffVersion::WorkingTree, DiffFile::same("app.rs"));
    open(&mut state, target);
    assert_eq!(text(shown(&state)).aligned.changes.len(), 3);
    let _ = repository.git(&["add", "app.rs"]);
    index_changed(&mut state);
    assert_eq!(text(shown(&state)).aligned.changes, []);
    let (_, tree) = Window::new(ThemeName::Dark, DIFF).draw(&state, &[]);
    assert!(says(&tree, "No changes"));
}

/// The position the bar says.
fn position(tree: &TreeUpdate) -> String {
    let said = |text: &str| says(tree, text);
    for count in 1..=5 {
        for current in 1..=count {
            let text = format!("{current} / {count}");
            if said(&text) {
                return text;
            }
        }
        if said(&format!("\u{2013} / {count}")) {
            return format!("\u{2013} / {count}");
        }
    }
    String::from("none")
}

#[test]
fn diff_navigate__in_a_diff_with_3_changes_f7_three_times_shows_1_2_and_3_of_3() {
    let repository = repository_with_three_changes();
    let mut state = showing(repository.path().to_path_buf());
    open(&mut state, head_to_working_tree("app.rs"));
    let mut window = Window::new(ThemeName::Dark, DIFF);
    let (_, tree) = window.draw(&state, &[]);
    assert_eq!(position(&tree), "\u{2013} / 3");
    let mut seen = Vec::new();
    for _ in 0..3 {
        seen.push(position(&press(&mut window, &mut state, &[f7(Modifiers::NONE)])));
    }
    assert_eq!(seen, ["1 / 3", "2 / 3", "3 / 3"]);
    let _ = find(&window.draw(&state, &[]).1, Role::Button, "Next change (F7)");
}

/// A repository with `a.rs`, `b10.rs` and `b9.rs` committed and each changed once in the working tree.
fn repository_with_three_files() -> Repository {
    let mut repository = Repository::new();
    let files = [
        ("a.rs", edited(5, &[])),
        ("b10.rs", edited(5, &[])),
        ("b9.rs", edited(5, &[])),
    ];
    let files: Vec<(&str, &str)> = files.iter().map(|(path, text)| (*path, text.as_str())).collect();
    let _ = repository.commit("Add the files", &files);
    for path in ["a.rs", "b10.rs", "b9.rs"] {
        std::fs::write(repository.path().join(path), edited(5, &[3])).unwrap();
    }
    repository
}

/// The path of the file the page shown is of.
fn file_shown(state: &State) -> String {
    shown(state).target.files.first().unwrap().right.clone()
}

/// `F7` pressed until the page goes on to another file, from its last change: once to say so, once to go.
fn go_on(window: &mut Window, state: &mut State, modifiers: Modifiers) -> TreeUpdate {
    let tree = press(window, state, &[f7(modifiers)]);
    let hint = if modifiers == Modifiers::NONE {
        "Press F7 again to go to the next file"
    } else {
        "Press Shift+F7 again to go to the previous file"
    };
    assert!(says(&tree, hint), "the bar says {hint}");
    press(window, state, &[f7(modifiers)])
}

#[test]
fn diff_navigate__with_a_b10_and_b9_changed_going_on_from_a_opens_b9_then_b10() {
    let repository = repository_with_three_files();
    let mut state = showing(repository.path().to_path_buf());
    open(&mut state, head_to_working_tree("a.rs"));
    let mut window = Window::new(ThemeName::Dark, DIFF);
    assert_eq!(
        position(&press(&mut window, &mut state, &[f7(Modifiers::NONE)])),
        "1 / 1"
    );
    let tree = go_on(&mut window, &mut state, Modifiers::NONE);
    assert_eq!(file_shown(&state), "b9.rs");
    assert_eq!(position(&tree), "1 / 1");
    let _ = go_on(&mut window, &mut state, Modifiers::NONE);
    assert_eq!(file_shown(&state), "b10.rs");
    assert_eq!(state.diff_pages.pages.len(), 1, "the same page shows the next file");
}

#[test]
fn diff_navigate__on_the_last_changed_file_going_on_opens_the_first_one() {
    let repository = repository_with_three_files();
    let mut state = showing(repository.path().to_path_buf());
    open(&mut state, head_to_working_tree("b10.rs"));
    let mut window = Window::new(ThemeName::Dark, DIFF);
    let _ = press(&mut window, &mut state, &[f7(Modifiers::NONE)]);
    let _ = go_on(&mut window, &mut state, Modifiers::NONE);
    assert_eq!(file_shown(&state), "a.rs");
    let _ = go_on(&mut window, &mut state, Modifiers::SHIFT);
    assert_eq!(file_shown(&state), "b10.rs");
}

#[test]
fn diff_navigate__against_a_commit_f7_on_the_last_change_goes_to_1_of_3_without_a_hint() {
    let mut repository = Repository::new();
    let parent = repository.commit("Add the app", &[("app.rs", &edited(30, &[]))]);
    let commit = repository.commit("Change the app", &[("app.rs", &edited(30, &[5, 15, 25]))]);
    let mut state = showing(repository.path().to_path_buf());
    let version = |hash: &str| DiffVersion::commit(CommitId::parse(hash.trim()).unwrap());
    open(
        &mut state,
        DiffTarget::file(version(&parent), version(&commit), DiffFile::same("app.rs")),
    );
    let mut window = Window::new(ThemeName::Dark, DIFF);
    for _ in 0..3 {
        let _ = press(&mut window, &mut state, &[f7(Modifiers::NONE)]);
    }
    let tree = press(&mut window, &mut state, &[f7(Modifiers::NONE)]);
    assert_eq!(position(&tree), "1 / 3");
    assert_eq!(shown(&state).hint, None);
    assert!(!says(&tree, "Press F7 again to go to the next file"));
}

#[test]
fn diff_navigate__another_key_between_the_two_presses_keeps_the_hint_and_shift_f7_cancels_it() {
    let repository = repository_with_three_files();
    let mut state = showing(repository.path().to_path_buf());
    open(&mut state, head_to_working_tree("a.rs"));
    let mut window = Window::new(ThemeName::Dark, DIFF);
    let _ = press(&mut window, &mut state, &[f7(Modifiers::NONE)]);
    let _ = press(&mut window, &mut state, &[f7(Modifiers::NONE)]);
    let tree = press(
        &mut window,
        &mut state,
        &[key(Key::Named(NamedKey::ArrowDown), Modifiers::NONE)],
    );
    assert!(says(&tree, "Press F7 again to go to the next file"));
    let _ = press(&mut window, &mut state, &[f7(Modifiers::NONE)]);
    assert_eq!(file_shown(&state), "b9.rs");
    let _ = press(&mut window, &mut state, &[f7(Modifiers::NONE)]);
    let tree = press(&mut window, &mut state, &[f7(Modifiers::SHIFT)]);
    assert!(!says(&tree, "Press F7 again to go to the next file"));
    let _ = press(&mut window, &mut state, &[f7(Modifiers::NONE)]);
    assert_eq!(
        file_shown(&state),
        "b9.rs",
        "the hint was cancelled, so F7 says it again"
    );
    let _ = press(&mut window, &mut state, &[f7(Modifiers::NONE)]);
    assert_eq!(file_shown(&state), "b10.rs");
}

#[test]
fn a_page_asked_to_go_to_the_next_change_with_no_page_does_nothing() {
    let repository = repository_with_three_changes();
    let mut state = showing(repository.path().to_path_buf());
    let intent = DiffPageIntent::Next(Duration::from_secs(1));
    ask(
        &mut state,
        Intent::Diff(DiffIntent::Page(views::diff::DiffPageId(7), intent)),
    );
    assert_eq!(state.diff_pages.pages, []);
}
