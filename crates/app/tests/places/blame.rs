//! The blame of the line under the cursor of the File page, and the operations run on that line, drawn from the store
//! on repositories made for each test: who last changed the line said at its end, the hover and its link, switching it
//! off and on in the user's settings, and opening or revealing the commit of the line.

use std::path::PathBuf;
use std::time::Duration;

use app::settings::{SettingsFiles, read_files};
use app::store::{Message, State};
use model::CommitId;
use platform::accesskit::{Role, TreeUpdate};
use render::ThemeName;
use testkit::Repository;
use ui::editor::{Command, Movement};
use ui::{InputEvent, Modifiers};
use views::Intent;
use views::file_view::{FilePageIntent, FileTarget, FileViewIntent};
use views::graph::Selected;
use views::page::Page;
use views::search::{QuickChoice, QuickCommand, SearchIntent};
use views::text::blame as words;

use super::compare::until_ended;
use super::file_view::{FILE, open, shown, step, type_text};
use super::pointer::{middle, moved, said_at};
use super::showing;
use super::support::{self, OperationsDriver, Window, has, says_in, shortcut};

/// What the end of the line under the cursor says before the subject of its commit.
const BULLET: &str = " \u{2022} ";

/// `src/app.rs` written by three commits: lines 1 and 3 by the first, line 2 by the second, lines 4 and 5 by the third.
pub fn three_commits() -> (Repository, [CommitId; 3]) {
    let mut repository = Repository::new();
    let first = repository.commit("Write the first lines", &[("src/app.rs", "one\ntwo\nthree\n")]);
    let second = repository.commit("Change the second line", &[("src/app.rs", "one\nTWO\nthree\n")]);
    let third = repository.commit("Add the end", &[("src/app.rs", "one\nTWO\nthree\nfour\nfive\n")]);
    let parse = |id: String| CommitId::parse(&id).unwrap();
    (repository, [parse(first), parse(second), parse(third)])
}

/// The user's settings file of the tests, beside the repository.
pub fn settings_file(repository: &Repository) -> PathBuf {
    repository.path().parent().unwrap().join("settings.json")
}

/// A window showing the File page of a file of a repository, the user's settings in a file of their own.
pub fn page_of_file(repository: &Repository, path: &str) -> (State, Window, OperationsDriver) {
    let mut state = showing(repository.path().to_path_buf());
    state.user_settings_file = Some(settings_file(repository));
    open(&mut state, FileTarget::working_tree(path));
    let mut window = Window::new(ThemeName::Dark, FILE);
    let _ = step(&mut window, &mut state, &[]);
    (state, window, OperationsDriver::new())
}

/// A window showing the File page of `src/app.rs`.
pub fn page(repository: &Repository) -> (State, Window, OperationsDriver) {
    page_of_file(repository, "src/app.rs")
}

/// Asks something of the File page shown, running the reads that follow.
pub fn ask_page(state: &mut State, intent: FilePageIntent) {
    let id = shown(state).id;
    let _effects = support::run(
        state,
        Message::Intent(Intent::FileView(FileViewIntent::Page(id, intent))),
    );
}

/// Puts the cursor at the start of a line, from 1, and lets the blame of the line be read.
pub fn at_line(state: &mut State, line: usize) {
    ask_page(state, FilePageIntent::GoToLine(line));
}

/// Gives the operations thread what a frame asked, and waits for the operation it started to end.
pub fn give(driver: &OperationsDriver, state: &mut State, intents: Vec<Intent>) {
    for intent in intents {
        driver.intent(state, intent);
    }
    if state.is_operation_running {
        until_ended(driver, state);
    }
}

/// Runs a command of Quick search, and waits for the operation it started to end.
pub fn quick(driver: &OperationsDriver, state: &mut State, command: QuickCommand) {
    let chosen = Intent::Search(SearchIntent::ChooseQuick(QuickChoice::Command(command)));
    give(driver, state, vec![chosen]);
}

/// Reads the user's settings file again, as the window does when it changes.
pub fn settings_read(state: &mut State, repository: &Repository) {
    let settings = read_files(&SettingsFiles {
        user: Some(settings_file(repository)),
        repository: None,
    });
    let _effects = support::run(state, Message::SettingsRead(settings));
}

/// The texts the window says that end of a line say of who last changed it.
fn notes(tree: &TreeUpdate) -> Vec<String> {
    tree.nodes
        .iter()
        .filter_map(|(_, node)| node.value().map(str::to_owned))
        .filter(|said| said.contains(BULLET) || said == model::blame::NOT_COMMITTED)
        .collect()
}

/// The key that runs an operation of the blame on the line of the cursor: `Alt+B`, with `Shift` for the timeline.
fn blame_key(with_shift: bool) -> InputEvent {
    let base = if cfg!(target_os = "macos") {
        Modifiers::ALT | Modifiers::CONTROL
    } else {
        Modifiers::ALT
    };
    shortcut('b', if with_shift { base | Modifiers::SHIFT } else { base })
}

/// The commit selected in the timeline.
fn selected(state: &State) -> Option<CommitId> {
    match &state.timeline.selected {
        Some(Selected::Commit(commit)) => Some(commit.clone()),
        _ => None,
    }
}

#[test]
fn blame_current_line__the_line_of_the_cursor_says_its_author_age_and_subject_at_its_end_and_only_there() {
    let (repository, _commits) = three_commits();
    let (mut state, mut window, _driver) = page(&repository);
    at_line(&mut state, 2);
    let said = notes(&step(&mut window, &mut state, &[]));
    assert_eq!(said.len(), 1, "{said:?}");
    let note = said.first().unwrap();
    assert!(note.starts_with("Test Author, "), "{note}");
    assert!(note.ends_with(&format!("{BULLET}Change the second line")), "{note}");
    at_line(&mut state, 4);
    let said = notes(&step(&mut window, &mut state, &[]));
    assert_eq!(said.len(), 1, "{said:?}");
    assert!(
        said.first().unwrap().ends_with(&format!("{BULLET}Add the end")),
        "{said:?}"
    );
}

#[test]
fn blame_current_line__typing_on_a_line_makes_it_say_not_committed_yet_once_the_cursor_rests() {
    let (repository, _commits) = three_commits();
    let (mut state, mut window, _driver) = page(&repository);
    at_line(&mut state, 2);
    type_text(&mut window, &mut state, "x");
    let said = notes(&step(&mut window, &mut state, &[]));
    assert_eq!(said, [model::blame::NOT_COMMITTED]);
}

#[test]
fn blame_current_line__with_blame_enabled_off_nothing_is_shown() {
    let (repository, _commits) = three_commits();
    let (mut state, mut window, _driver) = page(&repository);
    state.settings.features.blame.is_enabled = false;
    at_line(&mut state, 2);
    assert_eq!(notes(&step(&mut window, &mut state, &[])), Vec::<String>::new());
}

#[test]
fn blame_current_line__selecting_across_two_lines_shows_nothing() {
    let (repository, _commits) = three_commits();
    let (mut state, mut window, _driver) = page(&repository);
    at_line(&mut state, 2);
    assert_eq!(notes(&step(&mut window, &mut state, &[])).len(), 1);
    ask_page(
        &mut state,
        FilePageIntent::Edit(vec![Command::Move(Movement::Down(1), true)]),
    );
    assert_eq!(notes(&step(&mut window, &mut state, &[])), Vec::<String>::new());
}

#[test]
fn blame_current_line__the_link_of_the_hover_selects_the_commit_in_the_timeline() {
    let (repository, [_, second, _]) = three_commits();
    let (mut state, mut window, driver) = page(&repository);
    at_line(&mut state, 2);
    let tree = step(&mut window, &mut state, &[]);
    let note = notes(&tree).first().unwrap().clone();
    let point = middle(said_at(&tree, &note, ui::Rect::new(0.0, 0.0, 2000.0, 2000.0)));
    let _ = step(&mut window, &mut state, &[moved(point)]);
    window.now += Duration::from_millis(500);
    let _ = step(&mut window, &mut state, &[moved(point)]);
    let link = words::show_in_timeline(second.short());
    let tree = step(&mut window, &mut state, &[]);
    assert!(has(&tree, Role::Button, &link), "the hover is open");
    assert!(
        !has(&tree, Role::Button, words::OPEN_AS_PAGE),
        "the hover of the line has the one link"
    );
    let intents = window.click(&state, Role::Button, &link);
    give(&driver, &mut state, intents);
    assert_eq!(state.page_shown(), None, "the timeline is shown");
    assert_eq!(selected(&state), Some(second));
}

#[test]
fn blame_toggle_current_line__running_it_writes_blame_enabled_false_and_the_end_of_the_line_says_nothing() {
    let (repository, _commits) = three_commits();
    let (mut state, mut window, driver) = page(&repository);
    at_line(&mut state, 2);
    assert_eq!(notes(&step(&mut window, &mut state, &[])).len(), 1);
    quick(&driver, &mut state, QuickCommand::ToggleInlineBlame);
    let written = std::fs::read_to_string(settings_file(&repository)).unwrap();
    assert!(written.contains("\"blame.enabled\": false"), "{written}");
    settings_read(&mut state, &repository);
    assert_eq!(notes(&step(&mut window, &mut state, &[])), Vec::<String>::new());
}

#[test]
fn blame_toggle_current_line__running_it_again_sets_it_to_true() {
    let (repository, _commits) = three_commits();
    let (mut state, mut window, driver) = page(&repository);
    for _ in 0..2 {
        quick(&driver, &mut state, QuickCommand::ToggleInlineBlame);
        settings_read(&mut state, &repository);
    }
    let written = std::fs::read_to_string(settings_file(&repository)).unwrap();
    assert!(written.contains("\"blame.enabled\": true"), "{written}");
    at_line(&mut state, 2);
    assert_eq!(notes(&step(&mut window, &mut state, &[])).len(), 1);
}

#[test]
fn blame_open_line_commit__alt_b_on_a_line_opens_the_page_of_its_commit() {
    let (repository, [_, second, _]) = three_commits();
    let (mut state, mut window, driver) = page(&repository);
    at_line(&mut state, 2);
    let (intents, _) = window.draw(&state, &[blame_key(false)]);
    give(&driver, &mut state, intents);
    let Some(Page::Commit(id)) = state.page_shown() else {
        panic!("the page of the commit is shown, not {:?}", state.page_shown());
    };
    let opened = state.commit_pages.pages.iter().find(|page| page.id == id).unwrap();
    assert_eq!(opened.commit, second);
}

#[test]
fn blame_open_line_commit__alt_b_on_a_line_typed_since_the_last_commit_says_it_is_not_committed_yet() {
    let (repository, _commits) = three_commits();
    let (mut state, mut window, driver) = page(&repository);
    at_line(&mut state, 2);
    type_text(&mut window, &mut state, "x");
    let (intents, _) = window.draw(&state, &[blame_key(false)]);
    give(&driver, &mut state, intents);
    assert!(matches!(state.page_shown(), Some(Page::File(_))), "no page opens");
    assert!(says_in(
        &step(&mut window, &mut state, &[]),
        "This line is not committed yet."
    ));
}

#[test]
fn blame_reveal_line_commit__shift_alt_b_on_a_line_selects_its_commit_in_the_timeline() {
    let (repository, [first, ..]) = three_commits();
    let (mut state, mut window, driver) = page(&repository);
    at_line(&mut state, 3);
    let (intents, _) = window.draw(&state, &[blame_key(true)]);
    give(&driver, &mut state, intents);
    assert_eq!(state.page_shown(), None, "the timeline is shown");
    assert_eq!(selected(&state), Some(first));
}

#[test]
fn blame_reveal_line_commit__narrowed_to_a_branch_without_the_commit_the_narrowing_is_removed_and_it_is_selected() {
    let (mut repository, [first, ..]) = three_commits();
    let _ = repository.git(&["checkout", "--quiet", "--orphan", "other"]);
    let _ = repository.git(&["rm", "--quiet", "-r", "--cached", "."]);
    let _ = repository.commit("Start another history", &[("other.txt", "other\n")]);
    let _ = repository.git(&["checkout", "--quiet", "--force", "main"]);
    let (mut state, mut window, driver) = page(&repository);
    let only_other = model::filter::BranchesShown::References(vec![String::from("refs/heads/other")]);
    let _effects = support::run(
        &mut state,
        Message::Intent(Intent::Filter(views::FilterIntent::Show(Some(only_other)))),
    );
    let _ = step(&mut window, &mut state, &[]);
    at_line(&mut state, 1);
    let (intents, _) = window.draw(&state, &[blame_key(true)]);
    give(&driver, &mut state, intents);
    assert_eq!(selected(&state), Some(first));
    let rows: Vec<String> = super::timeline::subjects(&state);
    assert!(
        rows.iter().any(|subject| subject == "Write the first lines"),
        "{rows:?}"
    );
}
