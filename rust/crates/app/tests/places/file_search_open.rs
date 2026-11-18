//! The ways to the Search page (`file-search.open`): `Ctrl+Shift+F` and Quick search from the timeline, the text
//! selected in the File page in front, **Search here** in the context menu of the Files page; and what was searched
//! there kept for the repository (`file-search.remember`), after the window was closed too.

use app::settings::Settings;
use app::store::{Message, State};
use model::file_search::{Mode, Place, Scope};
use render::ThemeName;
use ui::Modifiers;
use ui::editor::{Command, Position};
use views::Intent;
use views::file_search::FileSearchIntent;
use views::file_view::{FilePageIntent, FileTarget};
use views::page::Page;
use views::search::{QuickChoice, QuickCommand, SearchIntent};
use views::text::file_search as words;

use super::blame::ask_page;
use super::file_search::{ask, sockets};
use super::file_view::{open, repository_with};
use super::files_page::{choose, files_page};
use super::showing;
use super::support::{self, Window, shortcut};

/// The size of the window.
const SIZE: (u32, u32) = (1000, 700);

/// The key that shows the Search page: `Ctrl+Shift+F`, `Cmd+Shift+F` on macOS, as the window listens for it.
fn search_key() -> ui::InputEvent {
    let command = if cfg!(target_os = "macos") {
        Modifiers::META
    } else {
        Modifiers::CONTROL
    };
    shortcut('F', command | Modifiers::SHIFT)
}

/// Draws the window with some input and gives the store what the frame asks.
fn press(window: &mut Window, state: &mut State, events: &[ui::InputEvent]) {
    let (intents, _) = window.draw(state, events);
    for intent in intents {
        let _effects = support::run(state, Message::Intent(intent));
    }
}

#[test]
fn file_search_open__ctrl_shift_f_shows_the_page_from_the_timeline_and_search_in_files_of_quick_search_does_too() {
    let repository = sockets();
    let mut state = showing(repository.path().to_path_buf());
    let mut window = Window::new(ThemeName::Dark, SIZE);
    assert_eq!(state.page_shown(), None, "the timeline is shown");
    press(&mut window, &mut state, &[search_key()]);
    assert_eq!(state.page_shown(), Some(Page::Search));
    let mut state = showing(repository.path().to_path_buf());
    let chosen = Intent::Search(SearchIntent::ChooseQuick(QuickChoice::Command(
        QuickCommand::SearchInFiles,
    )));
    let _effects = support::run(&mut state, Message::Intent(chosen));
    assert_eq!(state.page_shown(), Some(Page::Search));
}

#[test]
fn file_search_open__parse_log_selected_in_the_file_page_in_front_is_what_the_box_holds_after_the_key() {
    let repository = repository_with("fn parse_log() {}\n");
    let mut state = showing(repository.path().to_path_buf());
    open(&mut state, FileTarget::working_tree("src/app.rs"));
    ask_page(
        &mut state,
        FilePageIntent::Edit(vec![Command::SelectWord(Position { line: 0, column: 5 })]),
    );
    let mut window = Window::new(ThemeName::Dark, SIZE);
    press(&mut window, &mut state, &[search_key()]);
    assert_eq!(state.page_shown(), Some(Page::Search));
    assert_eq!(state.file_search.search.query, "parse_log");
}

#[test]
fn file_search_open__search_here_on_src_sockets_in_the_files_page_names_its_scope_and_keeps_what_is_typed() {
    let repository = sockets();
    let (mut state, mut window, _driver) = files_page(&repository, &["src"]);
    ask(&mut state, FileSearchIntent::Type(String::from("socket")));
    let intents = choose(&mut window, &state, ("sockets", words::SEARCH_HERE));
    for intent in intents {
        let _effects = support::run(&mut state, Message::Intent(intent));
    }
    assert_eq!(state.page_shown(), Some(Page::Search));
    let search = &state.file_search.search;
    assert_eq!(search.query, "socket");
    assert_eq!(search.scope, Scope::Named);
    let named = Place {
        path: String::from("src/sockets"),
        is_file: false,
    };
    assert_eq!(search.named, Some(named));
    assert_eq!(
        super::file_search::lines(&state),
        ["src/sockets/listen.rs:1", "src/sockets/listen.rs:3"]
    );
}

#[test]
fn file_search_remember__after_the_window_is_closed_and_opened_again_on_the_workspace_the_page_shows_what_was_typed() {
    let repository = sockets();
    let folder = repository.path().to_path_buf();
    let mut state = showing(folder.clone());
    ask(&mut state, FileSearchIntent::Show);
    ask(&mut state, FileSearchIntent::Type(String::from("socket")));
    ask(&mut state, FileSearchIntent::Mode(Mode::Text));
    let mut opened = State::new(state.persisted.clone(), Settings::default(), None);
    let _effects = support::run(&mut opened, Message::Started { folders: vec![folder] });
    ask(&mut opened, FileSearchIntent::Show);
    assert_eq!(opened.file_search.search.query, "socket");
    assert_eq!(opened.file_search.search.mode, Mode::Text);
    assert!(!opened.file_search.rows.is_empty(), "the text is searched again");
}
