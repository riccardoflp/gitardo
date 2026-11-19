//! The history of a shallow clone in the window (`graph.shallow-end`, `sync.deepen`, `sync.unshallow`): where it ends,
//! in the timeline and in the inspector, and the fetches that bring more of it. Every clone is of a throwaway
//! repository on disk, made with `--depth` through `file://` so that git honors it.

use std::path::PathBuf;

use app::store::{Message, State};
use model::graph::Row;
use platform::accesskit::Role;
use render::ThemeName;
use testkit::Repository;
use views::Intent;
use views::feature::FeatureIntent;
use views::graph::{GraphIntent, Selected, fetch_more};
use views::search::SearchIntent;
use views::text::details::NOT_FETCHED;
use views::text::timeline::{LOAD_MORE_HISTORY, LOAD_WHOLE_HISTORY, SHALLOW_END};

use super::showing;
use super::support::{OperationsDriver, Window, run, says};
use super::tabs::WIDE;

/// A repository of some commits, each changing one file.
fn history(commits: usize) -> Repository {
    let mut origin = Repository::new();
    for number in 1..=commits {
        let _commit = origin.commit(&format!("Change {number}"), &[("file.txt", &number.to_string())]);
    }
    origin
}

/// A clone of a repository beside it, with git's options, shown in a window.
fn clone_shown(origin: &Repository, options: &[&str]) -> (PathBuf, State) {
    let folder = origin.path().parent().unwrap().join("clone");
    let path = origin.path().to_string_lossy().replace('\\', "/");
    let url = format!("file:///{}", path.trim_start_matches('/'));
    let mut arguments = vec!["clone", "--quiet"];
    arguments.extend(options);
    arguments.extend([url.as_str(), folder.to_str().unwrap()]);
    let _cloned = origin.git(&arguments);
    for (key, value) in [("user.name", "Test Author"), ("user.email", "author@example.com")] {
        let _set = origin.git(&["-C", folder.to_str().unwrap(), "config", key, value]);
    }
    let state = showing(folder.clone());
    (folder, state)
}

/// The commits the timeline lists.
fn commits(state: &State) -> usize {
    state
        .timeline
        .rows
        .iter()
        .filter(|row| matches!(row, Row::Commit(_)))
        .count()
}

#[test]
fn graph_shallow_end__a_clone_with_depth_2_of_five_commits_lists_two_and_its_last_row_says_where_the_history_ends() {
    let origin = history(5);
    let (_folder, state) = clone_shown(&origin, &["--depth", "2"]);
    assert_eq!(commits(&state), 2);
    let tree = Window::new(ThemeName::Dark, WIDE).draw(&state, &[]).1;
    for words in [SHALLOW_END, LOAD_MORE_HISTORY, LOAD_WHOLE_HISTORY] {
        assert!(says(&tree, words), "{words}");
    }
}

#[test]
fn graph_shallow_end__the_first_commit_of_a_repository_cloned_whole_has_no_row_under_it() {
    let origin = history(3);
    let (_folder, state) = clone_shown(&origin, &[]);
    assert_eq!(commits(&state), 3);
    let tree = Window::new(ThemeName::Dark, WIDE).draw(&state, &[]).1;
    assert!(!says(&tree, SHALLOW_END));
}

#[test]
fn graph_shallow_end__load_more_history_runs_sync_deepen_and_load_the_whole_history_runs_sync_unshallow() {
    let origin = history(5);
    let (_folder, state) = clone_shown(&origin, &["--depth", "2"]);
    let mut window = Window::new(ThemeName::Dark, WIDE);
    assert_eq!(
        window.click(&state, Role::Button, LOAD_MORE_HISTORY),
        [fetch_more("sync.deepen")]
    );
    assert_eq!(
        window.click(&state, Role::Button, LOAD_WHOLE_HISTORY),
        [fetch_more("sync.unshallow")]
    );
}

#[test]
fn graph_shallow_end__selecting_the_older_commit_shows_not_fetched_for_its_parents_in_the_inspector() {
    let origin = history(5);
    let (_folder, mut state) = clone_shown(&origin, &["--depth", "2"]);
    let older = state.shown_read().unwrap().state.shallow.ends.first().unwrap().clone();
    let select = Intent::Graph(GraphIntent::Select(Selected::Commit(older)));
    let _effects = run(&mut state, Message::Intent(select));
    let tree = Window::new(ThemeName::Dark, (1200, 720)).draw(&state, &[]).1;
    assert!(says(&tree, NOT_FETCHED));
}

#[test]
fn graph_shallow_end__the_snapshots_of_the_end_of_a_shallow_history_match_in_the_dark_and_in_the_light_theme() {
    let origin = history(5);
    let (_folder, state) = clone_shown(&origin, &["--depth", "2"]);
    for (theme, name) in [
        (ThemeName::Dark, "shallow-end-dark"),
        (ThemeName::Light, "shallow-end-light"),
    ] {
        let mut window = Window::new(theme, WIDE);
        let _ = window.draw(&state, &[]);
        window.assert_snapshot(name);
    }
}

/// Whether Quick search, opened with a text, lists an entry.
fn quick_lists(state: &mut State, typed: &str, entry: &str) -> bool {
    let _effects = run(
        state,
        Message::Intent(Intent::Search(SearchIntent::OpenQuick(typed.to_owned()))),
    );
    let tree = Window::new(ThemeName::Dark, WIDE).draw(state, &[]).1;
    says(&tree, entry)
}

#[test]
fn sync_deepen__a_repository_that_is_not_shallow_has_no_load_more_history_in_quick_search() {
    let origin = history(3);
    let (_whole, mut state) = clone_shown(&origin, &[]);
    assert!(!quick_lists(&mut state, "history", "Load More History"));
    let (_shallow, mut shallow) = clone_shown(&history(3), &["--depth", "1"]);
    assert!(quick_lists(&mut shallow, "history", "Load More History"));
}

#[test]
fn sync_unshallow__a_repository_that_is_not_shallow_has_no_load_the_whole_history_in_quick_search() {
    let origin = history(3);
    let (_whole, mut state) = clone_shown(&origin, &[]);
    assert!(!quick_lists(&mut state, "whole", "Load the Whole History"));
    let (_shallow, mut shallow) = clone_shown(&history(3), &["--depth", "1"]);
    assert!(quick_lists(&mut shallow, "whole", "Load the Whole History"));
}

/// The variables the git of the tests runs with, for the thread of the operations.
fn driver(repository: &Repository) -> OperationsDriver {
    let command = repository.command("git");
    let git_variables = command
        .get_envs()
        .filter_map(|(name, value)| Some((name.to_os_string(), value?.to_os_string())))
        .collect();
    OperationsDriver::with_access(app::work::github::GitHubAccess {
        git_variables,
        ..app::work::github::GitHubAccess::system()
    })
}

/// Runs an operation at `HEAD` until it ends, confirming its plan and answering its questions as given.
fn run_operation(
    driver: &OperationsDriver,
    state: &mut State,
    feature: &'static str,
    answers: model::contract::Answers,
) {
    let started = FeatureIntent::RunWith(
        model::contract::FeatureId(feature),
        model::contract::Target::Head,
        answers,
    );
    driver.intent(state, Intent::Feature(started));
    loop {
        match driver.next(state) {
            Message::FeatureConfirms(_) => driver.intent(state, Intent::Feature(FeatureIntent::Confirm(true))),
            Message::FeatureEnded { .. } => return,
            _ => {}
        }
    }
}

#[test]
fn sync_unshallow__in_a_clone_with_depth_1_of_ten_commits_it_fetches_the_whole_history_and_the_row_is_gone() {
    let origin = history(10);
    let (_folder, mut state) = clone_shown(&origin, &["--depth", "1"]);
    assert_eq!(commits(&state), 1);
    let driver = driver(&origin);
    run_operation(&driver, &mut state, "sync.unshallow", model::contract::Answers::new());
    let lines: Vec<&str> = state.log.entries().map(|entry| entry.line.as_str()).collect();
    assert!(lines.contains(&"git fetch --unshallow origin"), "{lines:?}");
    let _effects = run(&mut state, Message::Intent(Intent::Refresh));
    assert_eq!(commits(&state), 10);
    assert!(!state.shown_read().unwrap().state.shallow.is_shallow());
    let tree = Window::new(ThemeName::Dark, WIDE).draw(&state, &[]).1;
    assert!(!says(&tree, SHALLOW_END));
}

#[test]
fn sync_deepen__three_more_commits_in_a_clone_with_depth_2_of_ten_lists_five_and_the_row_stays() {
    let origin = history(10);
    let (_folder, mut state) = clone_shown(&origin, &["--depth", "2"]);
    let driver = driver(&origin);
    let answers =
        model::contract::Answers::new().with(features::sync::COUNT, model::contract::Answer::Text("3".into()));
    run_operation(&driver, &mut state, "sync.deepen", answers);
    let _effects = run(&mut state, Message::Intent(Intent::Refresh));
    assert_eq!(commits(&state), 5);
    let tree = Window::new(ThemeName::Dark, WIDE).draw(&state, &[]).1;
    assert!(says(&tree, SHALLOW_END));
}
