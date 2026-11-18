//! The Search page drawn from the store, on a repository made for each test: the names and the lines of text found as
//! it is typed, in each mode and scope, narrowed by the patterns and the switches, and a result opened.

use app::store::{Effect, Message, State, reduce};
use app::work::jobs;
use git::CancelToken;
use model::file_search::{Mode, ResultRow, Section};
use platform::accesskit::{Role, Toggled, TreeUpdate};
use render::ThemeName;
use testkit::Repository;
use ui::{InputEvent, Key, KeyEvent, Modifiers, NamedKey};
use views::Intent;
use views::file_search::{FileSearchIntent, KEYS_HELD};
use views::page::Page;
use views::text::file_search as words;

use super::file_view::shown;
use super::pointer::{bounds, middle};
use super::showing;
use super::support::{self, Window, find, has, says, says_in, shortcut, typed};

/// The size of the window of the Search page.
const SIZE: (u32, u32) = (1000, 700);

/// A repository whose files say `socket` here and there: in a hook, in a folder `src/sockets`, in a document changed
/// since the commit, in an untracked file, in a file git ignores and in one under a folder whose name starts with a
/// dot.
pub(super) fn sockets() -> Repository {
    let mut repository = Repository::new();
    let hook = "import { Socket } from \"net\";\n\nexport function useSocket(): Socket {\n  const socket = open();\n  \
                return socket;\n}\n";
    let _ = repository.commit(
        "Write the sockets",
        &[
            ("src/hooks/useSocket.ts", hook),
            (
                "src/sockets/listen.rs",
                "// Listens on a Socket.\npub fn listen() {\n    let socket = bind();\n}\n",
            ),
            ("docs/guide.md", "# Guide\n\nA socket is opened by the hook.\n"),
            ("README.md", "Gitardo\n"),
            (".gitignore", "node_modules/\n"),
            (".github/workflows/ci.yml", "name: socket checks\n"),
        ],
    );
    let write = |path: &str, text: &str| {
        let on_disk = repository.path().join(path);
        std::fs::create_dir_all(on_disk.parent().unwrap()).unwrap();
        std::fs::write(on_disk, text).unwrap();
    };
    write("node_modules/pkg/index.js", "module.exports = socket;\n");
    write("notes.txt", "a socket to try\n");
    write(
        "docs/guide.md",
        "# Guide\n\nA socket is opened by the hook.\nAnother socket.\n",
    );
    repository
}

/// Asks something of the Search page, and runs the reads it starts.
pub(super) fn ask(state: &mut State, intent: FileSearchIntent) {
    let _effects = support::run(state, Message::Intent(Intent::FileSearch(intent)));
}

/// A window showing the Search page of a repository in a mode, drawn once: its box has the keyboard.
pub(super) fn search_page(repository: &Repository, mode: Mode) -> (State, Window) {
    let mut state = showing(repository.path().to_path_buf());
    ask(&mut state, FileSearchIntent::Show);
    ask(&mut state, FileSearchIntent::Mode(mode));
    let mut window = Window::new(ThemeName::Dark, SIZE);
    let _ = window.draw(&state, &[]);
    (state, window)
}

/// Draws the window with some input, gives the store what the frame asks, and draws it again: the tree it says then.
pub(super) fn act(window: &mut Window, state: &mut State, events: &[InputEvent]) -> TreeUpdate {
    let (intents, _) = window.draw(state, events);
    for intent in intents {
        let _effects = support::run(state, Message::Intent(intent));
    }
    window.draw(state, &[]).1
}

/// A key pressed with no modifier held.
pub(super) fn key(named: NamedKey) -> InputEvent {
    InputEvent::KeyPressed(KeyEvent::press(Key::Named(named), Modifiers::NONE))
}

/// Clicks the widget of a role and a name that is furthest to the right: a mode or a scope of the page, not the
/// button of the same name in the bar of views.
fn click(window: &mut Window, state: &mut State, (role, name): (Role, &str)) -> TreeUpdate {
    let (_, tree) = window.draw(state, &[]);
    let place = tree
        .nodes
        .iter()
        .filter(|(_, node)| node.role() == role && node.label() == Some(name))
        .filter_map(|(_, node)| node.bounds().map(super::pointer::rect))
        .max_by(|first, second| first.x.total_cmp(&second.x))
        .unwrap_or_else(|| panic!("no {role:?} named {name}"));
    let intents = super::pointer::click_at(window, state, middle(place), 1);
    for intent in intents {
        let _effects = support::run(state, Message::Intent(intent));
    }
    window.draw(state, &[]).1
}

/// Clicks a box of the page, named by what it says while it is empty, and types in it.
fn type_in(window: &mut Window, state: &mut State, (name, text): (&str, &str)) -> TreeUpdate {
    let (_, tree) = window.draw(state, &[]);
    let place = middle(bounds(&tree, Role::TextInput, name));
    let _ = super::pointer::click_at(window, state, place, 1);
    act(window, state, &[typed(text)])
}

/// The rows of the results, as words: a heading after `#`, a folder with `/` after it, a file by its path, and a
/// line of text as its path, its number and its text.
pub(super) fn listed(state: &State) -> Vec<String> {
    let page = &state.file_search;
    let found = &page.found;
    let said = |row: &ResultRow| match *row {
        ResultRow::Heading(section) => format!("# {}", words::section(section)),
        ResultRow::Folder(index) => format!("{}/", found.folders.get(index).unwrap().path),
        ResultRow::File(index) => found.files.get(index).unwrap().path.clone(),
        ResultRow::TextFile { first, .. } => found.text.get(first).unwrap().path.clone(),
        ResultRow::Line(index) => {
            let line = found.text.get(index).unwrap();
            words::line_said(&line.path, line.line, &line.text)
        }
    };
    page.rows.iter().map(said).collect()
}

/// The lines of text listed, each as its path and its number.
pub(super) fn lines(state: &State) -> Vec<String> {
    let page = &state.file_search;
    let lines = page.rows.iter().filter_map(|row| match row {
        ResultRow::Line(index) => page.found.text.get(*index),
        _ => None,
    });
    lines.map(|line| format!("{}:{}", line.path, line.line)).collect()
}

#[test]
fn file_search_page__usock_typed_in_files_lists_the_hook_and_the_box_had_the_keyboard_without_a_click() {
    let repository = sockets();
    let (mut state, mut window) = search_page(&repository, Mode::Files);
    let tree = act(&mut window, &mut state, &[typed("usock")]);
    assert_eq!(state.file_search.search.query, "usock");
    assert_eq!(listed(&state), ["src/hooks/useSocket.ts"]);
    assert!(has(&tree, Role::ListBoxOption, "src/hooks/useSocket.ts"));
    assert!(says(&tree, &words::files_found(1, 1)));
}

#[test]
fn file_search_page__socket_typed_in_text_lists_the_lines_under_their_files_with_their_numbers_and_counts_them() {
    let repository = sockets();
    let (mut state, mut window) = search_page(&repository, Mode::Text);
    let tree = window.draw(&state, &[]).1;
    assert!(says(&tree, words::TYPE_TO_SEARCH));
    let tree = act(&mut window, &mut state, &[typed("socket")]);
    let hook = "src/hooks/useSocket.ts";
    let rows = listed(&state);
    let of_hook = rows.iter().position(|row| row == hook).unwrap();
    assert_eq!(
        rows[of_hook + 1..of_hook + 5],
        [
            words::line_said(hook, 1, "import { Socket } from \"net\";"),
            words::line_said(hook, 3, "export function useSocket(): Socket {"),
            words::line_said(hook, 4, "const socket = open();"),
            words::line_said(hook, 5, "return socket;"),
        ]
    );
    let mut found = lines(&state);
    found.sort();
    assert_eq!(
        found,
        [
            "docs/guide.md:3",
            "docs/guide.md:4",
            "notes.txt:1",
            "src/hooks/useSocket.ts:1",
            "src/hooks/useSocket.ts:3",
            "src/hooks/useSocket.ts:4",
            "src/hooks/useSocket.ts:5",
            "src/sockets/listen.rs:1",
            "src/sockets/listen.rs:3",
        ]
    );
    assert!(has(
        &tree,
        Role::ListBoxOption,
        &words::line_said(hook, 4, "const socket = open();")
    ));
    assert!(says(&tree, "9 lines in 4 files"));
}

#[test]
fn file_search_page__in_all_the_folders_the_files_and_the_lines_found_are_under_their_headings() {
    let repository = sockets();
    let (mut state, mut window) = search_page(&repository, Mode::All);
    let tree = act(&mut window, &mut state, &[typed("socket")]);
    let rows = listed(&state);
    let heading = |section: Section| format!("# {}", words::section(section));
    assert_eq!(
        rows[..6],
        [
            heading(Section::Folders),
            String::from("src/sockets/"),
            heading(Section::Files),
            String::from("src/hooks/useSocket.ts"),
            String::from("src/sockets/listen.rs"),
            heading(Section::Text),
        ]
    );
    assert_eq!(lines(&state).len(), 9);
    for section in [Section::Folders, Section::Files, Section::Text] {
        assert!(says(&tree, words::section(section)));
    }
    assert!(has(&tree, Role::ListBoxOption, "src/sockets"));
}

#[test]
fn file_search_page__alt_2_chooses_text_and_alt_c_turns_match_case_on() {
    let repository = sockets();
    let (mut state, mut window) = search_page(&repository, Mode::All);
    let _ = act(&mut window, &mut state, &[shortcut('2', KEYS_HELD)]);
    assert_eq!(state.file_search.search.mode, Mode::Text);
    let _ = act(&mut window, &mut state, &[typed("socket")]);
    assert!(lines(&state).contains(&String::from("src/sockets/listen.rs:1")));
    let tree = act(&mut window, &mut state, &[shortcut('c', KEYS_HELD)]);
    assert!(state.file_search.search.match_case);
    assert_eq!(
        find(&tree, Role::CheckBox, words::MATCH_CASE).toggled(),
        Some(Toggled::True)
    );
    assert_eq!(state.file_search.search.query, "socket", "the key typed nothing");
    let found = lines(&state);
    assert!(!found.contains(&String::from("src/sockets/listen.rs:1")), "{found:?}");
    assert!(found.contains(&String::from("src/sockets/listen.rs:3")), "{found:?}");
}

#[test]
fn file_search_page__with_changes_chosen_only_the_lines_of_the_files_that_differ_and_of_the_untracked_are_listed() {
    let repository = sockets();
    let (mut state, mut window) = search_page(&repository, Mode::Text);
    let _ = act(&mut window, &mut state, &[typed("socket")]);
    let _ = click(&mut window, &mut state, (Role::Button, words::CHANGES));
    assert_eq!(state.file_search.search.scope, model::file_search::Scope::Changes);
    let mut found = lines(&state);
    found.sort();
    assert_eq!(found, ["docs/guide.md:3", "docs/guide.md:4", "notes.txt:1"]);
}

#[test]
fn file_search_page__include_leaves_the_rust_files_alone_and_exclude_takes_out_those_under_a_folder() {
    let repository = sockets();
    let (mut state, mut window) = search_page(&repository, Mode::Text);
    let _ = act(&mut window, &mut state, &[typed("socket")]);
    let _ = type_in(&mut window, &mut state, (words::INCLUDE, "*.rs"));
    assert_eq!(state.file_search.search.include, "*.rs");
    assert_eq!(lines(&state), ["src/sockets/listen.rs:1", "src/sockets/listen.rs:3"]);
    ask(&mut state, FileSearchIntent::Include(String::new()));
    let _ = type_in(&mut window, &mut state, (words::EXCLUDE, "docs/"));
    assert_eq!(state.file_search.search.exclude, "docs/");
    let found = lines(&state);
    assert_eq!(found.len(), 7, "{found:?}");
    assert!(!found.iter().any(|line| line.starts_with("docs/")), "{found:?}");
}

#[test]
fn file_search_page__gitignore_off_finds_what_git_ignores_and_hidden_on_what_starts_with_a_dot() {
    let repository = sockets();
    let (mut state, mut window) = search_page(&repository, Mode::Text);
    let tree = act(&mut window, &mut state, &[typed("socket")]);
    assert_eq!(
        find(&tree, Role::CheckBox, words::GITIGNORE).toggled(),
        Some(Toggled::True)
    );
    assert_eq!(
        find(&tree, Role::CheckBox, words::HIDDEN).toggled(),
        Some(Toggled::False)
    );
    let ignored = String::from("node_modules/pkg/index.js:1");
    let hidden = String::from(".github/workflows/ci.yml:1");
    assert!(!lines(&state).contains(&ignored) && !lines(&state).contains(&hidden));
    let _ = click(&mut window, &mut state, (Role::CheckBox, words::GITIGNORE));
    assert!(lines(&state).contains(&ignored), "{:?}", lines(&state));
    assert!(!lines(&state).contains(&hidden), "{:?}", lines(&state));
    let _ = click(&mut window, &mut state, (Role::CheckBox, words::GITIGNORE));
    let _ = click(&mut window, &mut state, (Role::CheckBox, words::HIDDEN));
    assert!(lines(&state).contains(&hidden), "{:?}", lines(&state));
    assert!(!lines(&state).contains(&ignored), "{:?}", lines(&state));
}

#[test]
fn file_search_page__down_from_the_box_passes_over_a_heading_and_enter_on_a_line_opens_the_file_page_there() {
    let repository = sockets();
    let (mut state, mut window) = search_page(&repository, Mode::All);
    let _ = act(&mut window, &mut state, &[typed("socket")]);
    assert_eq!(state.file_search.lit, Some(1), "the first result, under its heading");
    let tree = act(&mut window, &mut state, &[key(NamedKey::ArrowDown)]);
    assert_eq!(
        state.file_search.lit,
        Some(3),
        "the heading of the files is passed over"
    );
    assert_eq!(
        find(&tree, Role::ListBoxOption, "src/hooks/useSocket.ts").is_selected(),
        Some(true)
    );
    let _ = act(&mut window, &mut state, &[key(NamedKey::ArrowUp)]);
    assert_eq!(state.file_search.lit, Some(1));
    let hook = "src/hooks/useSocket.ts";
    let line = words::line_said(hook, 4, "const socket = open();");
    let row = listed(&state).iter().position(|row| *row == line).unwrap();
    ask(&mut state, FileSearchIntent::Light(row));
    let tree = act(&mut window, &mut state, &[key(NamedKey::Enter)]);
    let page = shown(&state);
    assert_eq!(page.path, hook);
    let app::store::FileContent::Text(document) = &page.content else {
        panic!("the file is read as text");
    };
    assert_eq!(page.cursor.head, document.buffer().line_range(3).start);
    assert_eq!(
        super::page_trail::names(&tree),
        [views::text::pages::TIMELINE, words::PAGE, "useSocket.ts"]
    );
}

#[test]
fn file_search_page__a_regular_expression_git_refuses_is_said_under_the_box_and_lists_no_line() {
    let repository = sockets();
    let (mut state, mut window) = search_page(&repository, Mode::Text);
    let _ = act(&mut window, &mut state, &[shortcut('r', KEYS_HELD)]);
    assert!(state.file_search.search.regex);
    let tree = act(&mut window, &mut state, &[typed("(")]);
    let refused = state.file_search.refused.clone().expect("git refuses it");
    assert!(says(&tree, refused.trim()), "{refused}");
    assert_eq!(lines(&state), Vec::<String>::new());
    let tree = act(&mut window, &mut state, &[typed("s.cket)")]);
    assert_eq!(state.file_search.search.query, "(s.cket)");
    assert_eq!(state.file_search.refused, None);
    assert_eq!(lines(&state).len(), 9);
    assert!(!says_in(&tree, refused.trim()));
}

#[test]
fn file_search_page__a_search_that_ends_after_another_was_started_is_thrown_away() {
    let repository = sockets();
    let (mut state, _window) = search_page(&repository, Mode::Text);
    let typing = |text: &str| Message::Intent(Intent::FileSearch(FileSearchIntent::Type(text.to_owned())));
    let started = |effects: Vec<Effect>| -> Vec<app::store::Job> {
        let jobs = effects.into_iter().filter_map(|effect| match effect {
            Effect::Start(job) => Some(job),
            _ => None,
        });
        jobs.collect()
    };
    let first = started(reduce(&mut state, typing("socket")));
    let effects = reduce(&mut state, typing("hook"));
    assert!(
        effects
            .iter()
            .any(|effect| matches!(effect, Effect::Cancel(job) if *job == first[0].id)),
        "the search before is stopped"
    );
    let second = started(effects);
    let said = |job| jobs::messages(job, &CancelToken::new());
    for message in said(&second[0]).into_iter().chain(said(&first[0])) {
        let _effects = support::run(&mut state, message);
    }
    assert_eq!(lines(&state), ["docs/guide.md:3"]);
    assert_eq!(state.file_search.finding, None);
}

#[test]
fn file_search_page__each_repository_tab_has_its_own_search() {
    let first = sockets();
    let mut second = Repository::new();
    let _ = second.commit("Write the notes", &[("notes.txt", "notes\n")]);
    let mut state = super::window();
    let folders = vec![first.path().to_path_buf(), second.path().to_path_buf()];
    let _effects = support::run(&mut state, Message::Started { folders });
    ask(&mut state, FileSearchIntent::Show);
    ask(&mut state, FileSearchIntent::Type(String::from("usock")));
    assert_eq!(listed(&state)[..2], ["# Files", "src/hooks/useSocket.ts"]);
    let _effects = support::run(&mut state, Message::Intent(Intent::SwitchTab(1)));
    assert_eq!(state.file_search.search.query, "");
    let _effects = support::run(&mut state, Message::Intent(Intent::SwitchTab(0)));
    assert_eq!(state.file_search.search.query, "usock");
    assert_eq!(state.page_shown(), Some(Page::Search));
    assert_eq!(listed(&state)[..2], ["# Files", "src/hooks/useSocket.ts"]);
}

#[test]
fn file_search_page__is_drawn_in_each_mode_in_the_dark_and_the_light_theme() {
    let repository = sockets();
    let pictures = [
        (ThemeName::Dark, Mode::All, "socket", "file-search-all-dark"),
        (ThemeName::Light, Mode::All, "socket", "file-search-all-light"),
        (ThemeName::Dark, Mode::Text, "socket", "file-search-text-dark"),
        (ThemeName::Dark, Mode::Files, "usock", "file-search-files-dark"),
        (ThemeName::Light, Mode::Folders, "src", "file-search-folders-light"),
        (ThemeName::Dark, Mode::Text, "", "file-search-empty-dark"),
    ];
    for (theme, mode, text, name) in pictures {
        let mut state = showing(repository.path().to_path_buf());
        ask(&mut state, FileSearchIntent::Show);
        ask(&mut state, FileSearchIntent::Mode(mode));
        ask(&mut state, FileSearchIntent::Type(text.to_owned()));
        let mut window = Window::new(theme, SIZE);
        let _ = window.draw(&state, &[]);
        let _ = window.draw(&state, &[]);
        window.assert_snapshot(name);
    }
}
