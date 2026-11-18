//! The Files page drawn from the store, on repositories made for each test: the tree of the working tree, a file
//! opened from it, the history of a file, of some lines and of a folder asked from the context menus, and a file or a
//! folder compared with a branch.

use app::store::{Message, State};
use model::contract::Answer;
use platform::accesskit::{Role, TreeUpdate};
use render::{Point, ThemeName};
use testkit::Repository;
use ui::editor::Command;
use ui::{InputEvent, Modifiers, PointerButton, PointerButtonEvent};
use views::Intent;
use views::feature::FeatureIntent;
use views::file_view::FilePageIntent;
use views::files::FilesPageIntent;
use views::page::Page;
use views::search::QuickCommand;
use views::text::{blame as blame_words, files as words};

use super::blame::{ask_page, at_line, give, page, quick, three_commits};
use super::compare::{files, until_ended};
use super::file_view::shown;
use super::pointer::{middle, moved, rect as rect_of};
use super::showing;
use super::support::{self, OperationsDriver, Window, says, says_in};
use super::timeline::subjects;

/// The size of the window of the Files page.
const SIZE: (u32, u32) = (1000, 700);

/// A window showing the Files page of a repository, with the folders named opened.
pub(super) fn files_page(repository: &Repository, opened: &[&str]) -> (State, Window, OperationsDriver) {
    let mut state = showing(repository.path().to_path_buf());
    let driver = OperationsDriver::new();
    quick(&driver, &mut state, QuickCommand::ShowFiles);
    for folder in opened {
        let _effects = support::run(
            &mut state,
            Message::Intent(Intent::Files(FilesPageIntent::Fold((*folder).to_owned()))),
        );
    }
    (state, Window::new(ThemeName::Dark, SIZE), driver)
}

/// The middle of the lowest thing of the window that says some words: the row of the tree that says them.
pub(super) fn row(tree: &TreeUpdate, name: &str) -> Point {
    let found = tree
        .nodes
        .iter()
        .filter(|(_, node)| node.value() == Some(name) || node.label() == Some(name))
        .filter_map(|(_, node)| node.bounds().map(rect_of))
        .max_by(|first, second| first.y.total_cmp(&second.y));
    middle(found.unwrap_or_else(|| panic!("nothing says {name}")))
}

/// Opens the context menu of the row that says some words, then chooses an entry of it, and gives what it asks.
pub(super) fn choose(window: &mut Window, state: &State, (name, entry): (&str, &str)) -> Vec<Intent> {
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
    window.click(state, Role::MenuItem, entry)
}

/// Whether the context menu of the row that says some words has an entry.
fn menu_has(window: &mut Window, state: &State, (name, entry): (&str, &str)) -> bool {
    let _ = choose(window, state, (name, words::COMPARE_WITH));
    let (_, tree) = window.draw(state, &[]);
    let point = row(&tree, name);
    let press = PointerButtonEvent {
        button: PointerButton::Secondary,
        position: point,
        modifiers: Modifiers::NONE,
        click_count: 1,
    };
    let (_, tree) = window.draw(
        state,
        &[InputEvent::PointerPressed(press), InputEvent::PointerReleased(press)],
    );
    support::has(&tree, Role::MenuItem, entry)
}

/// What the bar above the commits of the timeline says.
fn drawn(state: &State) -> TreeUpdate {
    Window::new(ThemeName::Dark, SIZE).draw(state, &[]).1
}

/// Answers the question of the side to compare with by a branch, and waits for the comparison to open.
fn compare_with(driver: &OperationsDriver, state: &mut State, intents: Vec<Intent>, branch: &str) {
    for intent in intents {
        driver.intent(state, intent);
    }
    loop {
        if let Message::FeatureAsks(_) = driver.next(state) {
            break;
        }
    }
    let answer = Answer::Choice(format!("ref:{branch}"));
    driver.intent(state, Intent::Feature(FeatureIntent::Answer(Some(answer))));
    until_ended(driver, state);
}

#[test]
fn file_view_page__a_click_on_a_file_of_the_files_page_opens_its_file_page() {
    let (repository, _commits) = three_commits();
    let (mut state, mut window, _driver) = files_page(&repository, &["src"]);
    let tree = window.draw(&state, &[]).1;
    assert!(says(&tree, words::PAGE));
    let point = row(&tree, "app.rs");
    let intents = super::pointer::click_at(&mut window, &state, point, 1);
    for intent in intents {
        let _effects = support::run(&mut state, Message::Intent(intent));
    }
    assert_eq!(shown(&state).path, "src/app.rs");
}

#[test]
fn file_history_file__the_history_of_src_b_rs_renamed_from_src_a_rs_lists_both_names_and_the_bar_says_so() {
    let mut repository = Repository::new();
    let _ = repository.commit("Write a", &[("src/a.rs", "one\ntwo\nthree\nfour\n")]);
    let _ = repository.commit("Write the readme", &[("README.md", "Gitardo\n")]);
    let _ = repository.git(&["mv", "src/a.rs", "src/b.rs"]);
    let _ = repository.commit("Rename a to b", &[]);
    let _ = repository.commit("Change b", &[("src/b.rs", "one\ntwo\nthree\nfour\nfive\n")]);
    let (mut state, mut window, driver) = files_page(&repository, &["src"]);
    let intents = choose(&mut window, &state, ("b.rs", words::SHOW_FILE_HISTORY));
    give(&driver, &mut state, intents);
    assert_eq!(state.page_shown(), None, "the timeline is shown");
    assert_eq!(subjects(&state), ["Change b", "Rename a to b", "Write a"]);
    let tree = drawn(&state);
    assert!(says(&tree, "History of src/b.rs"));
    assert!(says_in(&tree, "earlier src/a.rs"));
}

#[test]
fn file_history_file__a_commit_that_changed_only_other_files_is_not_listed() {
    let (mut repository, _commits) = three_commits();
    let _ = repository.commit("Write the readme", &[("README.md", "Gitardo\n")]);
    let (mut state, mut window, driver) = files_page(&repository, &["src"]);
    let intents = choose(&mut window, &state, ("app.rs", words::SHOW_FILE_HISTORY));
    give(&driver, &mut state, intents);
    assert_eq!(
        subjects(&state),
        ["Add the end", "Change the second line", "Write the first lines"]
    );
    assert!(says(&drawn(&state), "History of src/app.rs"));
}

#[test]
fn file_history_file__the_commits_of_a_later_file_with_an_earlier_name_are_listed_too() {
    let mut repository = Repository::new();
    let _ = repository.commit("Write a", &[("src/a.rs", "one\ntwo\nthree\nfour\n")]);
    let _ = repository.git(&["mv", "src/a.rs", "src/b.rs"]);
    let _ = repository.commit("Rename a to b", &[]);
    let _ = repository.commit("Write another a", &[("src/a.rs", "another\n")]);
    let (mut state, mut window, driver) = files_page(&repository, &["src"]);
    let intents = choose(&mut window, &state, ("b.rs", words::SHOW_FILE_HISTORY));
    give(&driver, &mut state, intents);
    assert_eq!(subjects(&state), ["Write another a", "Rename a to b", "Write a"]);
}

#[test]
fn file_history_file__narrowed_to_main_the_history_lists_only_the_commits_of_main() {
    let (repository, _commits) = three_commits();
    repository.branch("feature");
    repository.checkout("feature");
    let mut repository = repository;
    let _ = repository.commit(
        "Change the app on feature",
        &[("src/app.rs", "one\nTWO\nthree\nfour\nFIVE\n")],
    );
    let (mut state, mut window, driver) = files_page(&repository, &["src"]);
    let only_main = model::filter::BranchesShown::References(vec![String::from("refs/heads/main")]);
    let _effects = support::run(
        &mut state,
        Message::Intent(Intent::Filter(views::FilterIntent::Show(Some(only_main)))),
    );
    quick(&driver, &mut state, QuickCommand::ShowFiles);
    let intents = choose(&mut window, &state, ("app.rs", words::SHOW_FILE_HISTORY));
    give(&driver, &mut state, intents);
    assert_eq!(
        subjects(&state),
        ["Add the end", "Change the second line", "Write the first lines"]
    );
}

#[test]
fn file_history_lines__lines_selected_in_the_file_page_show_their_history_from_its_context_menu() {
    let (repository, _commits) = three_commits();
    let (mut state, mut window, driver) = page(&repository);
    at_line(&mut state, 2);
    ask_page(
        &mut state,
        FilePageIntent::Edit(vec![Command::Move(ui::editor::Movement::Down(2), true)]),
    );
    let (_, tree) = window.draw(&state, &[]);
    let editor = super::pointer::bounds(&tree, Role::TextInput, "app.rs");
    let point = Point::new(editor.x + editor.width / 2.0, editor.y + editor.height / 2.0);
    let press = PointerButtonEvent {
        button: PointerButton::Secondary,
        position: point,
        modifiers: Modifiers::NONE,
        click_count: 1,
    };
    let _ = window.draw(&state, &[moved(point)]);
    let _ = window.draw(
        &state,
        &[InputEvent::PointerPressed(press), InputEvent::PointerReleased(press)],
    );
    let intents = window.click(&state, Role::MenuItem, blame_words::SHOW_LINES_HISTORY);
    give(&driver, &mut state, intents);
    assert_eq!(state.page_shown(), None, "the timeline is shown");
    assert!(says(&drawn(&state), "History of lines 2\u{2013}3 of src/app.rs"));
    assert_eq!(subjects(&state), ["Change the second line", "Write the first lines"]);
}

#[test]
fn file_history_folder__the_history_of_src_ui_lists_the_commits_that_changed_a_file_under_it_and_only_those() {
    let mut repository = Repository::new();
    let _ = repository.commit("Add the button", &[("src/ui/button.rs", "button\n")]);
    let _ = repository.commit("Add the parser", &[("src/parse.rs", "parse\n")]);
    let _ = repository.commit("Add the list", &[("src/ui/list/rows.rs", "rows\n")]);
    let (mut state, mut window, driver) = files_page(&repository, &["src"]);
    let intents = choose(&mut window, &state, ("ui", words::SHOW_FOLDER_HISTORY));
    give(&driver, &mut state, intents);
    assert_eq!(subjects(&state), ["Add the list", "Add the button"]);
    assert!(says(&drawn(&state), "History of the folder src/ui"));
}

#[test]
fn file_history_folder__the_context_menu_of_the_top_folder_has_no_show_folder_history() {
    let (repository, _commits) = three_commits();
    let (state, mut window, _driver) = files_page(&repository, &[]);
    assert!(menu_has(&mut window, &state, ("src", words::SHOW_FOLDER_HISTORY)));
    assert!(!menu_has(
        &mut window,
        &state,
        ("repository", words::SHOW_FOLDER_HISTORY)
    ));
    assert!(menu_has(&mut window, &state, ("repository", words::COMPARE_WITH)));
}

/// A repository with `main` and, checked out, `feature`, which changed `src/app.rs` and `src/ui/list.rs` and added
/// `README.md`.
fn two_branches() -> Repository {
    let mut repository = Repository::new();
    let _ = repository.commit(
        "Write the app",
        &[("src/app.rs", "app\n"), ("src/ui/list.rs", "list\n")],
    );
    repository.branch("feature");
    repository.checkout("feature");
    let _ = repository.commit(
        "Change the app",
        &[
            ("src/app.rs", "app 2\n"),
            ("src/ui/list.rs", "list 2\n"),
            ("README.md", "readme\n"),
        ],
    );
    repository
}

#[test]
fn compare_path_with__on_src_app_rs_choosing_main_opens_main_and_the_working_tree_listing_only_that_file() {
    let repository = two_branches();
    let (mut state, mut window, driver) = files_page(&repository, &["src"]);
    let intents = choose(&mut window, &state, ("app.rs", words::COMPARE_WITH));
    compare_with(&driver, &mut state, intents, "main");
    assert!(matches!(state.page_shown(), Some(Page::Compare(_))));
    assert_eq!(files(&state), ["src/app.rs"]);
    assert!(says_in(&drawn(&state), "main \u{2194} Working tree"));
}

#[test]
fn compare_path_with__on_a_folder_the_page_lists_only_the_files_under_it() {
    let repository = two_branches();
    let (mut state, mut window, driver) = files_page(&repository, &["src"]);
    let intents = choose(&mut window, &state, ("ui", words::COMPARE_WITH));
    compare_with(&driver, &mut state, intents, "main");
    assert_eq!(files(&state), ["src/ui/list.rs"]);
}

#[test]
fn compare_path_with__on_a_file_of_another_repository_of_the_workspace_the_window_switches_to_it_first() {
    let mut first = Repository::new();
    let _ = first.commit("Write the notes", &[("notes.txt", "notes\n")]);
    let second = two_branches();
    let mut state = super::window();
    let folders = vec![first.path().to_path_buf(), second.path().to_path_buf()];
    let _effects = support::run(&mut state, Message::Started { folders });
    assert!(state.shown_folder().is_some_and(|shown| shown == first.path()));
    let driver = OperationsDriver::new();
    let on_disk = second.path().join("src").join("app.rs");
    let run = FeatureIntent::Run(
        model::contract::FeatureId("compare.path-with"),
        model::contract::Target::Path(on_disk),
    );
    compare_with(&driver, &mut state, vec![Intent::Feature(run)], "main");
    assert!(state.shown_folder().is_some_and(|shown| shown == second.path()));
    assert!(matches!(state.page_shown(), Some(Page::Compare(_))));
    assert_eq!(files(&state), ["src/app.rs"]);
}
