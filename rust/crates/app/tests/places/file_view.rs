//! The File page drawn from the store, on files of repositories made for each test: a file of the working tree opened
//! and edited, a file at a revision read only, the page shown again at another line, the text read again when the file
//! changes on disk, the edits shown in the Diff page of the file, Undo of what was typed, line endings kept, and the
//! question asked before closing a page with edits not saved.

use std::time::Duration;

use app::store::{FileContent, FilePage, Message, State};
use model::repo::Change;
use platform::accesskit::{Role, TreeUpdate};
use render::ThemeName;
use testkit::Repository;
use ui::{InputEvent, Key, KeyEvent, Modifiers, NamedKey};
use views::Intent;
use views::file_view::{FileTarget, FileViewIntent};
use views::page::Page;
use views::text::file_view as words;

use super::diff::{head_to_working_tree, open as open_diff};
use super::inspector::ask;
use super::showing;
use super::support::{self, Window, says, shortcut, typed};

/// The size of the window of the File pages.
pub const FILE: (u32, u32) = (1000, 600);

/// Lines `line 1` to `line <count>`, each ended by `\n`.
pub fn numbered(count: usize) -> String {
    let mut text = String::new();
    for line in 1..=count {
        text.push_str("line ");
        text.push_str(&line.to_string());
        text.push('\n');
    }
    text
}

/// A repository with `src/app.rs` committed with some text.
pub fn repository_with(text: &str) -> Repository {
    let mut repository = Repository::new();
    let _ = repository.commit("Add the app", &[("src/app.rs", text)]);
    repository
}

/// Opens the File page of a file, as a click on it in the Files page asks, and runs its read.
pub fn open(state: &mut State, target: FileTarget) {
    ask(state, Intent::FileView(FileViewIntent::Open(target)));
}

/// The File page shown, on its own or beside the tree of the Files page, which the test fails without.
pub fn shown(state: &State) -> &FilePage {
    let Some(id) = state.file_in_front() else {
        panic!("a File page is shown, not {:?}", state.page_shown());
    };
    state.file_pages.pages.iter().find(|page| page.id == id).unwrap()
}

/// The text of the File page shown.
pub fn text(state: &State) -> String {
    match &shown(state).content {
        FileContent::Text(document) => document.buffer().text().to_owned(),
        other => panic!("the file is read as text, not {other:?}"),
    }
}

/// Draws the window with some input and reduces what it asked, with the jobs it starts; gives the tree drawn.
pub fn step(window: &mut Window, state: &mut State, events: &[InputEvent]) -> TreeUpdate {
    let (intents, tree) = window.draw(state, events);
    for intent in intents {
        let _ = support::run(state, Message::Intent(intent));
    }
    window.now += Duration::from_millis(16);
    tree
}

/// A key pressed with some modifiers.
pub fn key(named: NamedKey, modifiers: Modifiers) -> InputEvent {
    InputEvent::KeyPressed(KeyEvent::press(Key::Named(named), modifiers))
}

/// Types a text one character at a time, as the keyboard does.
pub fn type_text(window: &mut Window, state: &mut State, text: &str) {
    for character in text.chars() {
        let _ = step(window, state, &[typed(&character.to_string())]);
    }
}

/// The working tree and the window of a page of `src/app.rs`, drawn once so that its editor has the keyboard.
fn page_of(text: &str) -> (Repository, State, Window) {
    let repository = repository_with(text);
    let mut state = showing(repository.path().to_path_buf());
    open(&mut state, FileTarget::working_tree("src/app.rs"));
    let mut window = Window::new(ThemeName::Dark, FILE);
    let _ = step(&mut window, &mut state, &[]);
    (repository, state, window)
}

#[test]
fn file_view_page__opening_src_app_rs_shows_a_page_app_rs_with_the_file_editable() {
    let (_repository, mut state, mut window) = page_of("fn main() {}\n");
    assert_eq!(shown(&state).title, "app.rs");
    let tree = step(&mut window, &mut state, &[]);
    assert!(says(&tree, "app.rs"));
    type_text(&mut window, &mut state, "pub ");
    assert_eq!(text(&state), "pub fn main() {}\n");
}

#[test]
fn file_view_page__a_file_at_a_revision_is_titled_with_its_short_hash_and_typing_does_nothing() {
    let mut repository = repository_with("fn main() {}\n");
    let commit = repository.commit("Print a greeting", &[("src/app.rs", "fn main() { hello(); }\n")]);
    let mut state = showing(repository.path().to_path_buf());
    let commit = model::CommitId::parse(&commit).unwrap();
    let short = commit.short().to_owned();
    open(
        &mut state,
        FileTarget {
            path: String::from("src/app.rs"),
            revision: Some(commit),
            line: None,
        },
    );
    assert_eq!(shown(&state).title, format!("app.rs ({short})"));
    let mut window = Window::new(ThemeName::Dark, FILE);
    let _ = step(&mut window, &mut state, &[]);
    type_text(&mut window, &mut state, "x");
    let _ = step(&mut window, &mut state, &[key(NamedKey::Enter, Modifiers::NONE)]);
    assert_eq!(text(&state), "fn main() { hello(); }\n");
    let tree = step(&mut window, &mut state, &[]);
    assert!(!support::has(&tree, Role::Button, words::OPEN_EXTERNAL), "read only");
}

#[test]
fn file_view_page__opening_the_same_file_at_line_120_shows_its_page_scrolled_to_that_line() {
    let (_repository, mut state, mut window) = page_of(&numbered(300));
    let id = shown(&state).id;
    open(
        &mut state,
        FileTarget {
            line: Some(120),
            ..FileTarget::working_tree("src/app.rs")
        },
    );
    assert_eq!(state.file_pages.pages.len(), 1, "one page per file");
    let page = shown(&state);
    assert_eq!(page.id, id);
    let FileContent::Text(document) = &page.content else {
        panic!("text")
    };
    assert_eq!(page.cursor.head, document.buffer().line_range(119).start);
    let tree = step(&mut window, &mut state, &[]);
    assert!(support::says_in(&tree, "line 120"), "line 120 is in sight");
    assert!(!support::says_in(&tree, "line 1\n"), "the first line is out of sight");
}

#[test]
fn file_view_page__a_file_changed_in_another_program_with_no_edits_shows_its_new_text() {
    let (repository, mut state, _window) = page_of("fn main() {}\n");
    std::fs::write(repository.path().join("src/app.rs"), "fn main() { changed(); }\n").unwrap();
    let changes = vec![Change::WorkingTree("src/app.rs".into())];
    let _ = support::run(
        &mut state,
        Message::Changed {
            changes,
            at: Duration::from_secs(10),
        },
    );
    let _ = support::run(
        &mut state,
        Message::Wake {
            at: Duration::from_secs(20),
        },
    );
    assert_eq!(text(&state), "fn main() { changed(); }\n");
}

#[test]
fn file_view_edit__typing_in_the_file_shows_the_text_in_its_diff_page_with_a_dot_on_both() {
    let (repository, mut state, mut window) = page_of("fn main() {}\n");
    open_diff(&mut state, head_to_working_tree("src/app.rs"));
    let _ = repository;
    let file = Page::File(shown_file_id(&state));
    let _ = support::run(&mut state, Message::Intent(Intent::ShowPage(file)));
    let _ = step(&mut window, &mut state, &[]);
    type_text(&mut window, &mut state, "// ");
    assert!(shown(&state).is_unsaved());
    let diff = state.diff_pages.pages.first().unwrap();
    let Some(views::diff::FileDiff::Text(diff_text)) = diff.diffs.first() else {
        panic!("the diff is read")
    };
    assert_eq!(diff_text.right, "// fn main() {}\n");
    let diff_id = diff.id;
    let _ = support::run(&mut state, Message::Intent(Intent::ShowPage(Page::Diff(diff_id))));
    let tree = step(&mut window, &mut state, &[]);
    assert!(says(&tree, words::UNSAVED_DOT), "the Diff page has the dot");
    let _ = support::run(&mut state, Message::Intent(Intent::ShowPage(file)));
    let tree = step(&mut window, &mut state, &[]);
    assert!(says(&tree, words::UNSAVED_DOT), "the File page has the dot");
}

/// The number of the File page open.
fn shown_file_id(state: &State) -> views::file_view::FilePageId {
    state.file_pages.pages.first().unwrap().id
}

#[test]
fn file_view_edit__three_typed_words_and_ctrl_z_three_times_give_back_the_saved_text_and_no_dot() {
    let (_repository, mut state, mut window) = page_of("fn main() {}\n");
    type_text(&mut window, &mut state, "one two three");
    assert_eq!(text(&state), "one two threefn main() {}\n");
    for _ in 0..3 {
        let _ = step(&mut window, &mut state, &[shortcut('z', Modifiers::CONTROL)]);
    }
    assert_eq!(text(&state), "fn main() {}\n");
    assert!(!shown(&state).is_unsaved());
    let tree = step(&mut window, &mut state, &[]);
    assert!(!says(&tree, words::UNSAVED_DOT));
}

#[test]
fn file_view_edit__a_file_with_crlf_line_endings_keeps_crlf_in_the_lines_typed() {
    let (_repository, mut state, mut window) = page_of("fn a() {}\r\nfn b() {}\r\n");
    let _ = step(&mut window, &mut state, &[key(NamedKey::End, Modifiers::NONE)]);
    let _ = step(&mut window, &mut state, &[key(NamedKey::Enter, Modifiers::NONE)]);
    type_text(&mut window, &mut state, "fn c() {}");
    let FileContent::Text(document) = &shown(&state).content else {
        panic!("text")
    };
    assert_eq!(document.buffer().to_bytes(), b"fn a() {}\r\nfn c() {}\r\nfn b() {}\r\n");
}

#[test]
fn file_view_edit__closing_the_page_of_an_unsaved_file_asks_and_dont_save_leaves_the_disk_as_it_was() {
    let (repository, mut state, mut window) = page_of("fn main() {}\n");
    type_text(&mut window, &mut state, "x");
    let _ = support::run(&mut state, Message::Intent(Intent::ClosePage));
    assert_eq!(state.page_shown().map(|page| matches!(page, Page::File(_))), Some(true));
    let tree = step(&mut window, &mut state, &[]);
    assert!(says(&tree, &words::save_changes("app.rs")));
    let intents = window.click(&state, Role::Button, words::DONT_SAVE);
    for intent in intents {
        let _ = support::run(&mut state, Message::Intent(intent));
    }
    assert_eq!(state.file_pages.pages.len(), 0, "the page is closed");
    let on_disk = std::fs::read_to_string(repository.path().join("src/app.rs")).unwrap();
    assert_eq!(on_disk, "fn main() {}\n");
}
