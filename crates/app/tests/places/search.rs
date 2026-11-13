//! The search box at the top of the timeline, drawn from the store over repositories made for each test: what it finds
//! among the loaded commits and in the whole history, the steps through the matches, the commits found as a list, a
//! hash gone to, **Go to commit…** and Quick search.

use app::store::search::Found;
use app::store::search::view::{dims_others, search_box};
use app::store::{Effect, Message, State, reduce};
use model::CommitId;
use model::search::BoxMode;
use platform::accesskit::Role;
use render::{Point, ThemeName};
use testkit::Repository;
use ui::{InputEvent, Key, KeyEvent, Modifiers, NamedKey};
use views::Intent;
use views::graph::Selected;
use views::search::{QuickChoice, SearchIntent};

use super::histories::{imported, line};
use super::pointer::{apply, bounds, click_at, says};
use super::support::{Window, find, run};
use super::{repository_with_history, showing};

/// The size of the window of these tests.
const WIDE: (u32, u32) = (1000, 520);

/// Asks the search for something, its jobs run.
pub fn search(state: &mut State, intent: SearchIntent) {
    apply(state, vec![Intent::Search(intent)]);
}

/// Types a text with a mode chosen, then presses `Enter`.
fn find_with(state: &mut State, mode: BoxMode, text: &str) {
    search(state, SearchIntent::Mode(mode));
    search(state, SearchIntent::Type(text.to_owned()));
    search(state, SearchIntent::Submit);
}

/// The commit selected.
pub fn selected(state: &State) -> Option<CommitId> {
    match &state.timeline.selected {
        Some(Selected::Commit(commit)) => Some(commit.clone()),
        _ => None,
    }
}

/// The subject of a commit loaded.
fn subject(state: &State, commit: &CommitId) -> String {
    state
        .timeline
        .rows
        .iter()
        .filter_map(|row| row.commit())
        .find(|found| &found.id == commit)
        .map(|found| found.subject.clone())
        .unwrap()
}

/// The subjects of what the search found, in order.
fn found_subjects(state: &State) -> Vec<String> {
    state
        .search
        .found
        .commits()
        .iter()
        .map(|commit| subject(state, commit))
        .collect()
}

/// The words the search box says at its end: the count, or what stands in its place.
fn count(state: &State) -> Option<String> {
    search_box(state).found.count_words()
}

/// The commit a name stands for in a repository.
pub fn commit_of(repository: &Repository, name: &str) -> CommitId {
    CommitId::parse(repository.git(&["rev-parse", name]).trim()).unwrap()
}

/// A press of a key with modifiers.
fn press(key: Key, modifiers: Modifiers) -> InputEvent {
    InputEvent::KeyPressed(KeyEvent::press(key, modifiers))
}

/// Draws the window with the keyboard in the search box, then with some keys, and gives the intents of the keys.
fn keys_in_box(window: &mut Window, state: &State, events: &[InputEvent]) -> Vec<Intent> {
    let placeholder = views::text::search::placeholder(state.search.mode);
    let _ = window.click(state, Role::SearchInput, placeholder);
    window.draw(state, events).0
}

#[test]
fn search_box__ctrl_f_with_the_keyboard_on_the_list_of_commits_puts_the_cursor_in_the_box_with_its_text_selected() {
    let (repository, _remote) = repository_with_history();
    let mut state = showing(repository.path().to_path_buf());
    search(&mut state, SearchIntent::Type(String::from("branch")));
    let mut window = Window::new(ThemeName::Dark, WIDE);
    let (_, tree) = window.draw(&state, &[]);
    let list = bounds(&tree, Role::ListBox, "History");
    let _ = click_at(&mut window, &state, Point::new(list.x + 200.0, list.y + 40.0), 1);
    let _ = window.draw(&state, &[press(Key::Character('f'), Modifiers::CONTROL)]);
    let (intents, _) = window.draw(&state, &[InputEvent::TextTyped(String::from("x"))]);
    assert_eq!(intents, [Intent::Search(SearchIntent::Type(String::from("x")))]);
}

#[test]
fn search_box__escape_in_the_box_empties_it_and_the_timeline_is_no_longer_dimmed() {
    let (repository, _remote) = repository_with_history();
    let mut state = showing(repository.path().to_path_buf());
    search(&mut state, SearchIntent::Type(String::from("branch")));
    assert!(dims_others(&state));
    let mut window = Window::new(ThemeName::Dark, WIDE);
    let intents = keys_in_box(
        &mut window,
        &state,
        &[press(Key::Named(NamedKey::Escape), Modifiers::NONE)],
    );
    assert_eq!(intents, [Intent::Search(SearchIntent::Empty)]);
    apply(&mut state, intents);
    assert_eq!(state.search.text, "");
    assert!(!dims_others(&state));
}

#[test]
fn search_box__choosing_message_after_a_search_of_the_loaded_commits_drops_the_highlight_until_enter() {
    let (repository, _remote) = repository_with_history();
    let mut state = showing(repository.path().to_path_buf());
    search(&mut state, SearchIntent::Type(String::from("rail")));
    assert_eq!(state.search.found_rows.len(), 1);
    search(&mut state, SearchIntent::Mode(BoxMode::Message));
    assert_eq!(state.search.found, Found::Nothing);
    assert_eq!(state.search.found_rows.len(), 0);
    assert!(!dims_others(&state));
    search(&mut state, SearchIntent::Submit);
    assert_eq!(found_subjects(&state), ["Show the branch rail"]);
    assert!(dims_others(&state));
}

#[test]
fn search_box__a_search_typed_dims_the_commits_it_did_not_find_and_marks_the_others() {
    let (repository, _remote) = repository_with_history();
    let mut state = showing(repository.path().to_path_buf());
    search(&mut state, SearchIntent::Type(String::from("settings")));
    assert_eq!(found_subjects(&state), ["Read the settings at startup"]);
    assert_eq!(count(&state).as_deref(), Some("1"));
    for (theme, name) in [
        (ThemeName::Dark, "search-typed-dark"),
        (ThemeName::Light, "search-typed-light"),
    ] {
        let mut window = Window::new(theme, WIDE);
        let (_, tree) = window.draw(&state, &[]);
        assert!(says(&tree, "1"));
        window.assert_snapshot(name);
    }
}

#[test]
fn search_history__message_fix_selects_the_first_commit_found_and_dims_the_others() {
    let mut repository = Repository::new();
    let _ = repository.commit("Fix the parser", &[("a.txt", "1\n")]);
    let _ = repository.commit("Add the rail", &[("a.txt", "2\n")]);
    let _ = repository.commit("Fix the rail", &[("a.txt", "3\n")]);
    let mut state = showing(repository.path().to_path_buf());
    find_with(&mut state, BoxMode::Message, "fix");
    assert_eq!(found_subjects(&state), ["Fix the rail", "Fix the parser"]);
    assert_eq!(state.search.found_rows, [0, 2]);
    assert!(dims_others(&state));
    let first = state.search.found.commits()[0].clone();
    assert_eq!(selected(&state), Some(first));
    assert_eq!(count(&state).as_deref(), Some("1/2"));
}

#[test]
fn search_history__enter_again_selects_the_second() {
    let mut repository = Repository::new();
    let _ = repository.commit("Fix the parser", &[("a.txt", "1\n")]);
    let _ = repository.commit("Fix the rail", &[("a.txt", "2\n")]);
    let mut state = showing(repository.path().to_path_buf());
    find_with(&mut state, BoxMode::Message, "fix");
    search(&mut state, SearchIntent::Submit);
    let second = state.search.found.commits()[1].clone();
    assert_eq!(selected(&state), Some(second));
    assert_eq!(count(&state).as_deref(), Some("2/2"));
}

#[test]
fn search_history__a_commit_made_meanwhile_is_counted_once_the_timeline_is_read_again() {
    let mut repository = Repository::new();
    let _ = repository.commit("Fix the parser", &[("a.txt", "1\n")]);
    let mut state = showing(repository.path().to_path_buf());
    find_with(&mut state, BoxMode::Message, "fix");
    assert_eq!(count(&state).as_deref(), Some("1/1"));
    let _ = repository.commit("Fix the rail", &[("a.txt", "2\n")]);
    let _effects = run(&mut state, Message::Intent(Intent::Refresh));
    assert_eq!(found_subjects(&state), ["Fix the rail", "Fix the parser"]);
}

#[test]
fn search_history__a_match_3000_commits_back_is_selected_after_the_history_is_loaded_up_to_it() {
    let repository = imported(&line(3_500));
    let mut state = showing(repository.path().to_path_buf());
    assert_eq!(state.timeline.loaded(), 1_000);
    find_with(&mut state, BoxMode::Message, "Change 500");
    let wanted = commit_of(&repository, "main~3000");
    assert_eq!(state.search.found.commits(), std::slice::from_ref(&wanted));
    assert_eq!(state.timeline.loaded(), 3_500);
    assert_eq!(selected(&state), Some(wanted.clone()));
    assert_eq!(subject(&state, &wanted), "Change 500");
}

#[test]
fn search_history__changes_regex_with_an_open_parenthesis_says_the_search_failed_and_dims_nothing() {
    let (repository, _remote) = repository_with_history();
    let mut state = showing(repository.path().to_path_buf());
    find_with(&mut state, BoxMode::ChangesPattern, "(");
    let failure = state.search.failure.clone().unwrap();
    assert!(failure.starts_with("Gitardo: the search failed. "), "{failure}");
    assert_eq!(count(&state).as_deref(), Some("0"));
    assert!(!dims_others(&state));
    let mut window = Window::new(ThemeName::Dark, WIDE);
    let (_, tree) = window.draw(&state, &[]);
    assert!(says(&tree, &failure));
}

#[test]
fn search_list_mode__with_the_funnel_pressed_author_alice_lists_only_her_commits_with_no_lines() {
    let mut repository = Repository::new();
    let _ = repository.commit("Add the parser", &[("a.txt", "1\n")]);
    let _ = repository.git(&["commit", "--quiet", "--allow-empty", "-m", "Draw the rail"]);
    let by_alice = ["--author", "Alice Example <alice@example.com>"];
    let _ = repository.git(
        &[
            &["commit", "--quiet", "--allow-empty", "-m", "Fold the merges"],
            &by_alice[..],
        ]
        .concat(),
    );
    let _ = repository.git(&["commit", "--quiet", "--allow-empty", "-m", "Name the branches"]);
    let _ = repository.git(
        &[
            &["commit", "--quiet", "--allow-empty", "-m", "Mark the matches"],
            &by_alice[..],
        ]
        .concat(),
    );
    let mut state = showing(repository.path().to_path_buf());
    search(&mut state, SearchIntent::Mode(BoxMode::Author));
    search(&mut state, SearchIntent::ToggleList);
    find_with(&mut state, BoxMode::Author, "alice");
    let listed: Vec<&str> = state
        .timeline
        .rows
        .iter()
        .filter_map(|row| row.commit())
        .map(|commit| commit.subject.as_str())
        .collect();
    assert_eq!(listed, ["Mark the matches", "Fold the merges"]);
    assert!(state.timeline.layout.cells.iter().all(|cell| cell.lines.is_empty()));
    let mut window = Window::new(ThemeName::Dark, WIDE);
    let (_, tree) = window.draw(&state, &[]);
    assert!(says(&tree, "Commits by authors matching alice \u{b7} 2 commits"));
    window.assert_snapshot("search-list-dark");
}

#[test]
fn search_list_mode__show_all_commits_brings_back_the_whole_timeline() {
    let (repository, _remote) = repository_with_history();
    let mut state = showing(repository.path().to_path_buf());
    let all = state.timeline.rows.len();
    search(&mut state, SearchIntent::Mode(BoxMode::Message));
    search(&mut state, SearchIntent::ToggleList);
    find_with(&mut state, BoxMode::Message, "settings");
    assert_eq!(state.timeline.rows.len(), 1);
    let mut window = Window::new(ThemeName::Dark, WIDE);
    let intents = window.click(&state, Role::Button, views::text::search::SHOW_ALL_COMMITS);
    assert_eq!(intents, [Intent::Search(SearchIntent::ShowAllCommits)]);
    apply(&mut state, intents);
    assert_eq!(state.timeline.rows.len(), all);
    assert!(!dims_others(&state));
}

#[test]
fn search_list_mode__the_funnel_is_still_pressed_after_the_window_is_opened_again() {
    let (repository, _remote) = repository_with_history();
    let mut state = showing(repository.path().to_path_buf());
    search(&mut state, SearchIntent::Mode(BoxMode::Message));
    let effects = reduce(&mut state, Message::Intent(Intent::Search(SearchIntent::ToggleList)));
    let written = effects.iter().find_map(|effect| match effect {
        Effect::WritePersisted(persisted) => Some(persisted.clone()),
        _ => None,
    });
    let text = written.unwrap().to_text();
    assert!(text.contains("\"searchAsList\": true"), "{text}");
    let mut opened = State::new(
        app::persisted::Persisted::from_text(&text),
        app::settings::Settings::default(),
        None,
    );
    let _effects = run(
        &mut opened,
        Message::Started {
            folders: vec![repository.path().to_path_buf()],
        },
    );
    search(&mut opened, SearchIntent::Mode(BoxMode::Message));
    assert_eq!(search_box(&opened).funnel, Some(true));
}

#[test]
fn search_list_mode__the_funnel_is_not_there_with_loaded() {
    let (repository, _remote) = repository_with_history();
    let mut state = showing(repository.path().to_path_buf());
    let mut window = Window::new(ThemeName::Dark, WIDE);
    let funnel = views::text::search::FUNNEL;
    let (_, tree) = window.draw(&state, &[]);
    assert!(!super::support::has(&tree, Role::Button, funnel));
    search(&mut state, SearchIntent::Mode(BoxMode::Changes));
    let (_, tree) = window.draw(&state, &[]);
    let _funnel = find(&tree, Role::Button, funnel);
}

#[test]
fn quick_search_opens_with_ctrl_k_and_goes_to_a_branch_chosen() {
    let (repository, _remote) = repository_with_history();
    let mut state = showing(repository.path().to_path_buf());
    let mut window = Window::new(ThemeName::Dark, WIDE);
    let (intents, _) = window.draw(&state, &[press(Key::Character('k'), Modifiers::CONTROL)]);
    assert_eq!(intents, [Intent::Search(SearchIntent::OpenQuick(String::new()))]);
    apply(&mut state, intents);
    let _ = window.draw(&state, &[]);
    let _ = window.draw(&state, &[InputEvent::TextTyped(String::from("feat"))]);
    let (_, tree) = window.draw(&state, &[]);
    let _feature = find(&tree, Role::ListBoxOption, "feature");
    window.assert_snapshot("quick-search-dark");
    let feature = commit_of(&repository, "feature");
    search(
        &mut state,
        SearchIntent::ChooseQuick(QuickChoice::Commit(feature.clone())),
    );
    assert!(!state.search.quick.is_open);
    assert_eq!(selected(&state), Some(feature));
}

#[test]
fn quick_search_switch_worktree_runs_the_operation_that_lists_the_worktrees() {
    let (repository, _remote) = repository_with_history();
    let mut state = showing(repository.path().to_path_buf());
    search(&mut state, SearchIntent::OpenQuick(String::from("worktree")));
    let effects = reduce(
        &mut state,
        Message::Intent(Intent::Search(SearchIntent::ChooseQuick(QuickChoice::Command(
            views::search::QuickCommand::SwitchWorktree,
        )))),
    );
    let runs = effects.iter().any(
        |effect| matches!(effect, app::store::Effect::RunFeature { feature, .. } if feature.0 == "worktree.switch"),
    );
    assert!(runs, "{effects:?}");
    assert!(!state.search.quick.is_open);
}
