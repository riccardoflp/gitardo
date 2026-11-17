//! The pages of a read of the timeline taken as git prints them (`graph.load-more`): the jobs run as a worker would,
//! but the pages after the first of each read are held, to be given to the window one at a time.

use std::collections::VecDeque;

use app::settings::Settings;
use app::store::{Effect, Message, State, reduce};
use model::filter::show_only;
use platform::accesskit::Role;
use render::ThemeName;
use testkit::Repository;
use views::graph::{GraphIntent, Selected};
use views::{FilterIntent, Intent};

use super::histories::{imported, line};
use super::pointer::{apply, bounds, middle, says, wheel};
use super::support::{Window, run_holding, showing_first_page};
use super::timeline::subjects;

/// The size of the window of these tests.
const WIDE: (u32, u32) = (1000, 520);

/// A window that opened a repository with the default settings, and the pages of its read still to come.
fn opened(repository: &Repository) -> (State, VecDeque<Message>) {
    showing_first_page(repository.path().to_path_buf(), Settings::default())
}

/// Gives the window the next page held.
fn give_next(state: &mut State, held: &mut VecDeque<Message>) {
    let page = held.pop_front().unwrap();
    let _effects = run_holding(state, page, held);
}

/// Whether the last row of the list, scrolled to its end, says more commits are loaded.
fn says_loading_more(state: &State) -> bool {
    let mut window = Window::new(ThemeName::Dark, WIDE);
    let (_, tree) = window.draw(state, &[]);
    let list = bounds(&tree, Role::ListBox, "History");
    let _ = wheel(&mut window, state, middle(list), 1_000_000.0);
    let (_, tree) = window.draw(state, &[]);
    says(&tree, "Loading more commits…")
}

#[test]
fn graph_load_more__in_a_repository_of_5000_commits_the_five_pages_come_one_after_the_other_and_loading_ends_after_the_fifth()
 {
    let repository = imported(&line(5_000));
    let (mut state, mut held) = opened(&repository);
    assert_eq!(state.timeline.loaded(), 1_000);
    assert_eq!(held.len(), 4, "four pages follow the first");
    assert!(says_loading_more(&state));
    for pages in 2..=5 {
        give_next(&mut state, &mut held);
        assert_eq!(state.timeline.loaded(), pages * 1_000, "the list was never scrolled");
    }
    assert!(!state.timeline.has_more);
    assert!(!says_loading_more(&state));
    assert_eq!(subjects(&state).last().map(String::as_str), Some("Change 1"));
}

#[test]
fn graph_load_more__after_a_page_is_added_the_selected_commit_and_the_scroll_position_are_as_they_were() {
    let repository = imported(&line(2_500));
    let (mut state, mut held) = opened(&repository);
    let chosen = state.timeline.rows.get(3).and_then(|row| row.id()).unwrap().clone();
    apply(
        &mut state,
        vec![Intent::Graph(GraphIntent::Select(Selected::Commit(chosen.clone())))],
    );
    let reveal = state.timeline.reveal;
    let first = state.timeline.rows.clone();
    give_next(&mut state, &mut held);
    assert_eq!(state.timeline.loaded(), 2_000);
    assert_eq!(state.timeline.selected, Some(Selected::Commit(chosen)));
    assert_eq!(state.timeline.reveal, reveal, "nothing scrolls the list");
    assert_eq!(state.timeline.rows.get(..1_000), Some(first.as_slice()));
}

#[test]
fn graph_load_more__choosing_another_branch_while_a_read_runs_stops_it_and_no_page_of_it_is_added_after() {
    let repository = imported(&line(3_000));
    let _ = repository.git(&["branch", "feature", "main~2500"]);
    let (mut state, held) = opened(&repository);
    assert_eq!(held.len(), 2);
    let old = held.clone();
    let mut new = VecDeque::new();
    let effects = run_holding(
        &mut state,
        Message::Intent(Intent::Filter(FilterIntent::Show(Some(show_only(
            "refs/heads/feature",
        ))))),
        &mut new,
    );
    assert!(
        effects.iter().any(|effect| matches!(effect, Effect::Cancel(_))),
        "the old read is stopped: {effects:?}"
    );
    assert!(new.is_empty(), "the history of feature is one page");
    let narrowed = subjects(&state);
    assert_eq!(narrowed.len(), 500);
    for page in old {
        let _effects = reduce(&mut state, page);
    }
    assert_eq!(subjects(&state), narrowed, "no page of the old read is added");
}
