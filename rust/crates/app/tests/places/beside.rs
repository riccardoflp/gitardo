//! Pages beside what opened them, drawn from the store: the Diff page of a file of the inspector where the timeline was,
//! the inspector staying beside it (`diff.beside-inspector`), and a file of the Files page on the right of its tree
//! (`file-view.beside-files`).

use app::store::{Message, State, ViewData};
use platform::accesskit::{Role, TreeUpdate};
use render::{Point, ThemeName};
use testkit::Repository;
use ui::editor::Command;
use ui::{InputEvent, Key, KeyEvent, Modifiers, NamedKey};
use views::diff::DiffIntent;
use views::file_view::{FilePageIntent, FileViewIntent};
use views::files::FilesPageIntent;
use views::page::Page;
use views::search::QuickCommand;
use views::{Content, Intent, TimelineState};

use super::blame::quick;
use super::diff::shown;
use super::diff_links::words_at;
use super::inspector::{ask, select};
use super::pointer::{bounds, click_at, drag, middle, rect};
use super::showing;
use super::support::{self, OperationsDriver, Window, has};

/// A window wide enough for the inspector beside the timeline.
const WIDE: (u32, u32) = (1200, 700);
/// A window narrower than the narrow layout, where the inspector is over the timeline.
const NARROW: (u32, u32) = (680, 700);

/// A repository whose last commit changes `src/a.rs`, `src/b.rs` and `src/c.rs`.
fn three_files() -> Repository {
    let mut repository = Repository::new();
    let _ = repository.commit(
        "Start",
        &[("src/a.rs", "a\n"), ("src/b.rs", "b\n"), ("src/c.rs", "c\n")],
    );
    let _ = repository.commit(
        "Change three files",
        &[
            ("src/a.rs", "a, changed\n"),
            ("src/b.rs", "b, changed\n"),
            ("src/c.rs", "c, changed\n"),
        ],
    );
    repository
}

/// Clicks the words of a window drawn from the store, and hands what the click asks to the store.
fn click_words(window: &mut Window, state: &mut State, words: &str) {
    let (_, tree) = window.draw(state, &[]);
    let intents = click_at(window, state, words_at(&tree, words), 1);
    for intent in intents {
        let _effects = support::run(state, Message::Intent(intent));
    }
}

/// The path of the file the inspector marks, while the Diff page beside it is shown.
fn marked(state: &State) -> Option<String> {
    let data = ViewData::new(state, Vec::new(), support::NOW);
    match data.window_state(state).content {
        Content::Timeline(TimelineState::Repository { inspector, beside, .. }) => {
            assert!(beside.is_some(), "the Diff page is beside the inspector");
            inspector.shown_file.map(|shown| shown.path.to_owned())
        }
        _ => None,
    }
}

/// The right side of the file the Diff page shown compares.
fn diff_file(state: &State) -> String {
    shown(state)
        .target
        .files
        .first()
        .map(|file| file.right.clone())
        .unwrap_or_default()
}

/// The middle of **Timeline** in the trail, not the one of the bar of views.
fn trail_timeline(tree: &TreeUpdate) -> render::Point {
    let trail = bounds(tree, Role::Navigation, views::text::pages::TRAIL);
    let found = tree
        .nodes
        .iter()
        .filter(|(_, node)| node.role() == Role::Button && node.label() == Some(views::text::pages::TIMELINE))
        .filter_map(|(_, node)| node.bounds().map(rect))
        .find(|button| button.y >= trail.y && button.bottom() <= trail.bottom() + 1.0);
    middle(found.unwrap())
}

/// The tree of a window drawn from the store.
fn drawn(window: &mut Window, state: &State) -> TreeUpdate {
    window.draw(state, &[]).1
}

#[test]
fn diff_beside_inspector__a_click_on_a_file_opens_its_diff_and_the_inspector_stays_with_it_marked() {
    let repository = three_files();
    let mut state = showing(repository.path().to_path_buf());
    let _ = select(&mut state, "Change three files");
    let mut window = Window::new(ThemeName::Dark, WIDE);
    click_words(&mut window, &mut state, "a.rs");
    assert_eq!(diff_file(&state), "src/a.rs");
    assert_eq!(marked(&state).as_deref(), Some("src/a.rs"));
    let tree = drawn(&mut window, &state);
    assert!(has(&tree, Role::Complementary, views::text::timeline::INSPECTOR));
    assert!(has(&tree, Role::Navigation, views::text::pages::TRAIL));
}

#[test]
fn diff_beside_inspector__a_click_on_another_file_shows_it_in_the_same_page() {
    let repository = three_files();
    let mut state = showing(repository.path().to_path_buf());
    let _ = select(&mut state, "Change three files");
    let mut window = Window::new(ThemeName::Dark, WIDE);
    click_words(&mut window, &mut state, "a.rs");
    let page = shown(&state).id;
    click_words(&mut window, &mut state, "b.rs");
    assert_eq!(shown(&state).id, page);
    assert_eq!(diff_file(&state), "src/b.rs");
    assert_eq!(marked(&state).as_deref(), Some("src/b.rs"));
    let diffs = state
        .pages
        .open
        .iter()
        .filter(|page| matches!(page, Page::Diff(_)))
        .count();
    assert_eq!(diffs, 1, "Ctrl+Tab lists one Diff page");
}

#[test]
fn diff_beside_inspector__alt_down_shows_the_diff_of_the_next_file() {
    let repository = three_files();
    let mut state = showing(repository.path().to_path_buf());
    let _ = select(&mut state, "Change three files");
    let mut window = Window::new(ThemeName::Dark, WIDE);
    click_words(&mut window, &mut state, "a.rs");
    let key = |key| InputEvent::KeyPressed(KeyEvent::press(Key::Named(key), Modifiers::ALT));
    let (intents, _) = window.draw(&state, &[key(NamedKey::ArrowDown)]);
    assert!(intents.contains(&Intent::Diff(DiffIntent::StepBeside(views::diff::FileStep::Next))));
    for intent in intents {
        let _effects = support::run(&mut state, Message::Intent(intent));
    }
    assert_eq!(diff_file(&state), "src/b.rs");
    let (intents, _) = window.draw(&state, &[key(NamedKey::ArrowUp)]);
    for intent in intents {
        let _effects = support::run(&mut state, Message::Intent(intent));
    }
    assert_eq!(diff_file(&state), "src/a.rs");
}

#[test]
fn diff_beside_inspector__timeline_in_the_trail_shows_the_timeline_with_the_same_commit_selected() {
    let repository = three_files();
    let mut state = showing(repository.path().to_path_buf());
    let commit = select(&mut state, "Change three files");
    let mut window = Window::new(ThemeName::Dark, WIDE);
    click_words(&mut window, &mut state, "a.rs");
    let (_, tree) = window.draw(&state, &[]);
    let intents = click_at(&mut window, &state, trail_timeline(&tree), 1);
    assert_eq!(intents, [Intent::ShowTimeline]);
    for intent in intents {
        let _effects = support::run(&mut state, Message::Intent(intent));
    }
    assert_eq!(state.page_shown(), None);
    assert_eq!(state.timeline.selected, Some(views::graph::Selected::Commit(commit)));
    let (intents, _) = window.draw(&state, &[]);
    assert_eq!(intents, [] as [Intent; 0]);
    let escape = InputEvent::KeyPressed(KeyEvent::press(Key::Named(NamedKey::Escape), Modifiers::NONE));
    ask(&mut state, Intent::NextPage);
    let (intents, _) = window.draw(&state, &[escape]);
    assert_eq!(intents, [Intent::ShowTimeline], "Escape brings the timeline back too");
}

#[test]
fn diff_beside_inspector__in_a_narrow_window_a_file_opens_its_diff_in_place_of_the_timeline_and_the_inspector() {
    let repository = three_files();
    let mut state = showing(repository.path().to_path_buf());
    let _ = select(&mut state, "Change three files");
    let mut window = Window::new(ThemeName::Dark, NARROW);
    click_words(&mut window, &mut state, "a.rs");
    assert_eq!(diff_file(&state), "src/a.rs");
    let tree = drawn(&mut window, &state);
    assert!(!has(&tree, Role::Complementary, views::text::timeline::INSPECTOR));
    assert!(has(&tree, Role::Navigation, views::text::pages::TRAIL));
}

/// A repository with `README.md` and `src/app.rs`, with the Files page shown and `src` opened.
fn files_page() -> (Repository, State, Window) {
    let mut repository = Repository::new();
    let _ = repository.commit("Start", &[("README.md", "Gitardo\n"), ("src/app.rs", "fn main() {}\n")]);
    let mut state = showing(repository.path().to_path_buf());
    quick(&OperationsDriver::new(), &mut state, QuickCommand::ShowFiles);
    ask(&mut state, Intent::Files(FilesPageIntent::Fold(String::from("src"))));
    (repository, state, Window::new(ThemeName::Dark, WIDE))
}

/// The path of the file shown beside the tree of the Files page, and the path the tree marks.
fn beside_files(state: &State) -> (String, Option<String>) {
    let data = ViewData::new(state, Vec::new(), support::NOW);
    let Content::Page(host) = data.window_state(state).content else {
        panic!("a page is shown");
    };
    match host.page {
        views::page::PageState::FilesBeside(beside) => {
            (beside.file.path.to_owned(), beside.tree.marked.map(str::to_owned))
        }
        other => panic!("the Files page shows a file, not {:?}", other.page()),
    }
}

#[test]
fn file_view_beside_files__a_click_on_a_file_shows_it_on_the_right_and_the_tree_stays_with_it_marked() {
    let (_repository, mut state, mut window) = files_page();
    click_words(&mut window, &mut state, "app.rs");
    assert_eq!(state.page_shown(), Some(Page::Files));
    assert_eq!(
        beside_files(&state),
        (String::from("src/app.rs"), Some(String::from("src/app.rs")))
    );
    let tree = drawn(&mut window, &state);
    assert!(has(&tree, Role::ListBox, views::text::files::TREE));
    assert!(has(&tree, Role::Main, views::text::file_view::PAGE));
}

#[test]
fn file_view_beside_files__a_click_on_another_file_shows_it_in_the_same_place_and_the_folders_stay_open() {
    let (_repository, mut state, mut window) = files_page();
    click_words(&mut window, &mut state, "app.rs");
    click_words(&mut window, &mut state, "README.md");
    assert_eq!(beside_files(&state).0, "README.md");
    assert!(state.files_page.opened.contains("src"));
    assert_eq!(state.pages.open, [Page::Files]);
}

#[test]
fn file_view_beside_files__a_file_with_unsaved_edits_stays_in_ctrl_tab_when_another_is_opened() {
    let (_repository, mut state, mut window) = files_page();
    click_words(&mut window, &mut state, "app.rs");
    let id = state.files_page.shown_file.unwrap();
    let typed = FilePageIntent::Edit(vec![Command::Type(String::from("// "))]);
    ask(&mut state, Intent::FileView(FileViewIntent::Page(id, typed)));
    click_words(&mut window, &mut state, "README.md");
    assert_eq!(beside_files(&state).0, "README.md");
    assert!(state.pages.open.contains(&Page::File(id)));
}

#[test]
fn file_view_beside_files__the_line_dragged_resizes_the_tree_which_keeps_its_width_when_the_window_is_opened_again() {
    let (repository, mut state, mut window) = files_page();
    click_words(&mut window, &mut state, "app.rs");
    let tree = bounds(&drawn(&mut window, &state), Role::ListBox, views::text::files::TREE);
    let file = bounds(&drawn(&mut window, &state), Role::Main, views::text::file_view::PAGE);
    let line = Point::new(file.x - 0.5, file.y + file.height * 0.5);
    let intents = drag(&mut window, &state, line, Point::new(line.x + 60.0, line.y));
    super::pointer::apply(&mut state, intents);
    let wider = bounds(&drawn(&mut window, &state), Role::ListBox, views::text::files::TREE);
    assert!((wider.width - tree.width - 60.0).abs() < 1.0, "{tree:?} {wider:?}");
    let mut again = super::panes::opened_again(&state, repository.path());
    quick(&OperationsDriver::new(), &mut again, QuickCommand::ShowFiles);
    let mut window = Window::new(ThemeName::Dark, WIDE);
    click_words(&mut window, &mut again, "README.md");
    let kept = bounds(&drawn(&mut window, &again), Role::ListBox, views::text::files::TREE);
    assert!((kept.width - wider.width).abs() < 1.0, "{kept:?}");
}

#[test]
fn file_view_beside_files__files_in_the_trail_shows_the_tree_alone_again() {
    let (_repository, mut state, mut window) = files_page();
    click_words(&mut window, &mut state, "app.rs");
    let id = state.files_page.shown_file;
    let trail = bounds(&drawn(&mut window, &state), Role::Navigation, views::text::pages::TRAIL);
    let tree = drawn(&mut window, &state);
    let files = tree
        .nodes
        .iter()
        .filter(|(_, node)| node.role() == Role::Button && node.label() == Some(views::text::files::PAGE))
        .filter_map(|(_, node)| node.bounds().map(rect))
        .find(|button| button.y >= trail.y && button.bottom() <= trail.bottom() + 1.0);
    let intents = click_at(&mut window, &state, middle(files.unwrap()), 1);
    assert_eq!(intents, [Intent::Files(FilesPageIntent::ShowTree)]);
    super::pointer::apply(&mut state, intents);
    assert_eq!(state.page_shown(), Some(Page::Files));
    assert_eq!(state.files_page.shown_file, None);
    assert!(id.is_some_and(|id| !state.pages.open.contains(&Page::File(id))));
    assert!(!has(
        &drawn(&mut window, &state),
        Role::Main,
        views::text::file_view::PAGE
    ));
}

#[test]
fn the_diff_page_beside_the_inspector_is_drawn() {
    let repository = three_files();
    let mut state = showing(repository.path().to_path_buf());
    let _ = select(&mut state, "Change three files");
    let mut window = Window::new(ThemeName::Dark, WIDE);
    click_words(&mut window, &mut state, "b.rs");
    let _ = window.draw(&state, &[]);
    window.assert_snapshot("diff-beside-inspector-dark");
}

#[test]
fn the_file_beside_the_tree_of_the_files_page_is_drawn() {
    let (_repository, mut state, mut window) = files_page();
    click_words(&mut window, &mut state, "app.rs");
    let _ = window.draw(&state, &[]);
    window.assert_snapshot("files-beside-file-dark");
}
