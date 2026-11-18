//! The preview of the Search page drawn from the store (`file-search.preview`): the file of the result under the
//! keyboard with the line found lit and its matches marked, a folder, a line of it opened, and the files it does not
//! show.

use app::store::State;
use app::store::file_search::page_state;
use model::file_search::Mode;
use platform::accesskit::{Role, TreeUpdate};
use render::ThemeName;
use testkit::Repository;
use ui::NamedKey;
use views::file_search::{FileSearchIntent, PreviewShown, PreviewState};
use views::text::file_search as words;

use super::file_search::{act, ask, key, listed, search_page, sockets};
use super::file_view::shown;
use super::pointer::{bounds, rect};
use super::support::{self, Window, says, typed};

/// The path of the hook whose lines say `socket`.
const HOOK: &str = "src/hooks/useSocket.ts";

/// The preview the page is drawn with.
fn preview(state: &State) -> PreviewState<'_> {
    page_state(state, None)
        .preview
        .unwrap_or_else(|| panic!("no preview is shown"))
}

/// The row of the results that says some words, brought under the keyboard.
fn light(state: &mut State, said: &str) {
    let row = listed(state).iter().position(|row| row == said).unwrap();
    ask(state, FileSearchIntent::Light(row));
}

/// The names the preview of a folder lists, from its top.
fn folder_names(tree: &TreeUpdate, names: &[&str]) -> Vec<String> {
    let region = bounds(tree, Role::Region, words::PREVIEW);
    let mut found: Vec<(f32, String)> = tree
        .nodes
        .iter()
        .filter_map(|(_, node)| {
            let label = node.label().or(node.value())?;
            let place = rect(node.bounds()?);
            let is_in = names.contains(&label) && place.x >= region.x;
            is_in.then(|| (place.y, label.to_owned()))
        })
        .collect();
    found.sort_by(|first, second| first.0.total_cmp(&second.0));
    found.into_iter().map(|(_, label)| label).collect()
}

#[test]
fn file_search_preview__a_line_under_the_keyboard_shows_its_file_with_the_line_lit_and_every_match_marked() {
    let repository = sockets();
    let (mut state, mut window) = search_page(&repository, Mode::Text);
    let _ = act(&mut window, &mut state, &[typed("socket")]);
    light(&mut state, &words::line_said(HOOK, 4, "const socket = open();"));
    let tree = act(&mut window, &mut state, &[]);
    let shown = preview(&state);
    assert_eq!((shown.path, shown.lit_line), (HOOK, Some(3)));
    let PreviewShown::Text { buffer, marks, .. } = shown.shown else {
        panic!("the file is shown as text, not {:?}", shown.shown);
    };
    let marked: Vec<&str> = marks.iter().map(|mark| &buffer.text()[mark.clone()]).collect();
    assert_eq!(marked, ["Socket", "Socket", "Socket", "socket", "socket"]);
    assert!(support::has(&tree, Role::TextInput, HOOK));
}

#[test]
fn file_search_preview__down_to_a_line_of_another_file_shows_that_file() {
    let repository = sockets();
    let (mut state, mut window) = search_page(&repository, Mode::Text);
    let _ = act(&mut window, &mut state, &[typed("socket")]);
    let first = preview(&state).path.to_owned();
    for _ in 0..10 {
        let _ = act(&mut window, &mut state, &[key(NamedKey::ArrowDown)]);
        if preview(&state).path != first {
            break;
        }
    }
    let shown = preview(&state);
    assert_ne!(shown.path, first);
    assert!(matches!(shown.shown, PreviewShown::Text { .. }));
    assert!(shown.lit_line.is_some(), "the line found is lit");
}

#[test]
fn file_search_preview__a_file_found_by_its_name_is_shown_from_its_first_line() {
    let repository = sockets();
    let (mut state, mut window) = search_page(&repository, Mode::Files);
    let _ = act(&mut window, &mut state, &[typed("usock")]);
    let shown = preview(&state);
    assert_eq!((shown.path, shown.lit_line, shown.reveal.1), (HOOK, None, 0));
    let PreviewShown::Text { buffer, marks, .. } = shown.shown else {
        panic!("the file is shown as text");
    };
    assert!(buffer.text().starts_with("import { Socket }"));
    assert!(marks.is_empty(), "the text was not searched");
}

#[test]
fn file_search_preview__a_folder_shows_what_it_holds_its_folders_first() {
    let repository = sockets();
    std::fs::write(repository.path().join("src").join("lib.rs"), "pub mod hooks;\n").unwrap();
    let (mut state, mut window) = search_page(&repository, Mode::Folders);
    let tree = act(&mut window, &mut state, &[typed("src")]);
    assert_eq!(listed(&state).first().map(String::as_str), Some("src/"));
    let shown = preview(&state);
    assert_eq!(shown.path, "src");
    let PreviewShown::Folder { folders, files } = shown.shown else {
        panic!("the folder is shown, not {:?}", shown.shown);
    };
    assert_eq!(folders, ["hooks", "sockets"]);
    assert_eq!(files, ["lib.rs"]);
    assert_eq!(
        folder_names(&tree, &["hooks", "sockets", "lib.rs"]),
        ["hooks", "sockets", "lib.rs"]
    );
}

#[test]
fn file_search_preview__a_double_click_on_a_line_of_the_preview_opens_the_file_page_at_that_line() {
    let repository = sockets();
    let (mut state, mut window) = search_page(&repository, Mode::Text);
    let _ = act(&mut window, &mut state, &[typed("socket")]);
    light(&mut state, &words::line_said(HOOK, 4, "const socket = open();"));
    let (_, tree) = window.draw(&state, &[]);
    let editor = bounds(&tree, Role::TextInput, HOOK);
    let first_line = render::Point::new(editor.x + editor.width / 2.0, editor.y + 6.0);
    let intents = window.click_at(&state, first_line, 2);
    assert_eq!(
        intents,
        [views::Intent::FileSearch(FileSearchIntent::OpenAt(HOOK.to_owned(), 1))]
    );
    super::pointer::apply(&mut state, intents);
    let page = shown(&state);
    assert_eq!((page.path.as_str(), page.cursor.head), (HOOK, 0));
}

/// A window whose Search page shows a file it found by its name, written with some bytes.
fn previewing(name: &str, bytes: &[u8]) -> (Repository, State, TreeUpdate) {
    let repository = sockets();
    std::fs::write(repository.path().join(name), bytes).unwrap();
    let (mut state, mut window) = search_page(&repository, Mode::Files);
    let tree = act(&mut window, &mut state, &[typed(name)]);
    (repository, state, tree)
}

#[test]
fn file_search_preview__a_file_with_a_nul_byte_says_it_is_not_text() {
    let (_repository, state, tree) = previewing("data.bin", b"socket\0\x01\x02");
    assert_eq!(preview(&state).shown, PreviewShown::NotText);
    assert!(says(&tree, words::NOT_TEXT));
}

#[test]
fn file_search_preview__a_file_larger_than_1_mb_is_not_read_and_the_preview_says_so() {
    let large = "socket\n".repeat(160_000);
    let (_repository, state, tree) = previewing("large.txt", large.as_bytes());
    assert_eq!(preview(&state).shown, PreviewShown::TooLarge);
    assert!(says(&tree, words::TOO_LARGE));
}

#[test]
fn file_search_preview__is_drawn_with_a_line_lit_in_the_dark_and_the_light_theme() {
    let repository = sockets();
    for (theme, name) in [
        (ThemeName::Dark, "file-search-preview-dark"),
        (ThemeName::Light, "file-search-preview-light"),
    ] {
        let (mut state, _) = search_page(&repository, Mode::Text);
        ask(&mut state, FileSearchIntent::Type(String::from("socket")));
        light(&mut state, &words::line_said(HOOK, 4, "const socket = open();"));
        let mut window = Window::new(theme, (1200, 700));
        let _ = window.draw(&state, &[]);
        let _ = window.draw(&state, &[]);
        window.assert_snapshot(name);
    }
}
