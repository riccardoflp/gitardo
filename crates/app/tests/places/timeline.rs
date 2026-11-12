//! The timeline of a repository drawn from the store: the commits and their labels, the columns, the pages loaded while
//! scrolling, the draft and the stashes.

use std::collections::HashSet;
use std::path::PathBuf;

use app::persisted::Persisted;
use app::settings::Settings;
use app::store::{Effect, Message, State, draft_of};
use model::commit::Commit;
use model::graph::{PageSize, Row};
use model::state::RepositoryState;
use model::{CommitId, Timestamp};
use platform::accesskit::Role;
use render::ThemeName;
use views::Intent;
use views::graph::{
    Folds, GraphIntent, GraphState, HistoryState, LayoutInput, Reading, Selected, Switches, TimelineLayout,
    TimelineSwitch,
};

use super::histories::{imported, line};
use super::pointer::{apply, bounds, rows_drawn, says, wheel};
use super::support::{Window, run};
use super::{repository_with_history, showing};

/// The size of the window of these tests.
const WIDE: (u32, u32) = (1000, 520);

/// A window showing a folder with some settings, its jobs run, that writes its settings in a file.
pub fn showing_with(folder: PathBuf, settings: Settings) -> State {
    let file = folder.join("settings.json");
    let mut state = State::new(
        Persisted {
            has_started: true,
            ..Persisted::default()
        },
        settings,
        Some(file),
    );
    let _effects = run(&mut state, Message::Started { folders: vec![folder] });
    state
}

/// The settings with pages of 100 commits.
pub fn pages_of_100() -> Settings {
    Settings {
        page_size: PageSize::from_setting(Some(50.0)),
        ..Settings::default()
    }
}

/// The subjects of the rows read, from the top.
pub fn subjects(state: &State) -> Vec<String> {
    state
        .timeline
        .rows
        .iter()
        .map(|row| match row {
            Row::Commit(commit) | Row::Stash { commit, .. } => commit.subject.clone(),
            Row::Uncommitted { .. } => String::from("(draft)"),
        })
        .collect()
}

/// The setting a switch writes, among the effects of turning it.
pub fn written(effects: &[Effect]) -> Option<(&'static str, json::Value)> {
    effects.iter().find_map(|effect| match effect {
        Effect::WriteSetting { key, value, .. } => Some((*key, value.clone())),
        _ => None,
    })
}

#[test]
fn graph_view__a_repository_with_main_feature_and_a_tag_lists_the_commits_of_all_three_newest_first_with_labels() {
    let (repository, _remote) = repository_with_history();
    let state = showing(repository.path().to_path_buf());
    assert_eq!(
        subjects(&state),
        [
            "On main: half the tags",
            "Show the branch rail",
            "Read the settings at startup",
            "Add the readme"
        ]
    );
    let mut window = Window::new(ThemeName::Dark, WIDE);
    let (_, tree) = window.draw(&state, &[]);
    let rows = rows_drawn(&tree);
    for (label, row) in [("main", 1), ("feature", 2), ("v1.0", 2), ("origin/main", 2)] {
        let at = super::pointer::said_at(&tree, label, rows[row]);
        assert!(
            at.y >= rows[row].y && at.bottom() <= rows[row].bottom(),
            "{label} is on row {row}"
        );
    }
}

#[test]
fn graph_view__in_a_new_repository_with_no_commits_and_no_changes_the_list_says_no_commits_yet() {
    let repository = testkit::Repository::new();
    let state = showing(repository.path().to_path_buf());
    let mut window = Window::new(ThemeName::Dark, WIDE);
    let (_, tree) = window.draw(&state, &[]);
    assert_eq!(rows_drawn(&tree).len(), 0);
    assert!(says(&tree, "No commits yet"));
}

/// A line of commits made by hand, the newest first.
fn line_of_rows(count: usize) -> Vec<Row> {
    let id = |number: usize| CommitId::parse(&format!("{number:040x}")).unwrap();
    (1..=count)
        .map(|number| {
            Row::Commit(Commit {
                id: id(number),
                parents: if number < count {
                    vec![id(number + 1)]
                } else {
                    Vec::new()
                },
                author_name: String::from("Test Author"),
                author_email: String::new(),
                author_date: Timestamp(1_735_689_600),
                subject: format!("Change {number}"),
            })
        })
        .collect()
}

#[test]
fn graph_view__scrolling_a_list_of_50000_commits_draws_only_the_rows_in_sight() {
    let rows = line_of_rows(50_000);
    let repository = RepositoryState::default();
    let folded = HashSet::new();
    let layout = TimelineLayout::new(&LayoutInput {
        rows: &rows,
        repository: &repository,
        folded: &folded,
        combine_upstream: true,
        separators_at: None,
    });
    let history = HistoryState {
        rows: &rows,
        layout: &layout,
        folds: Folds {
            folded: &folded,
            is_all_folded: false,
        },
        has_no_commits: false,
        has_more: false,
        reading: Reading::Nothing,
        asked_at: None,
        selected: None,
        reveal: None,
        trace: None,
        hover: None,
        hover_left_at: None,
        switches: Switches::DEFAULT,
        graph_width: None,
        now: 1_735_689_600,
        found: &[],
        draft: None,
    };
    let graph = GraphState::Listed(Box::new(history));
    let mut window = Window::new(ThemeName::Dark, WIDE);
    let (_, tree) = window.frame(&[], |ui| views::graph::view(&graph, ui));
    let drawn = rows_drawn(&tree);
    assert!(drawn.len() <= 12, "{} rows drawn", drawn.len());
    let list = bounds(&tree, Role::ListBox, "History");
    let turn = ui::InputEvent::Wheel(ui::WheelEvent {
        delta: ui::WheelDelta::Pixels { x: 0.0, y: 1_000_000.0 },
        position: super::pointer::middle(list),
        modifiers: ui::Modifiers::NONE,
    });
    let _ = window.frame(&[turn], |ui| views::graph::view(&graph, ui));
    let (_, tree) = window.frame(&[], |ui| views::graph::view(&graph, ui));
    let drawn = rows_drawn(&tree);
    assert!(!drawn.is_empty() && drawn.len() <= 12, "{} rows drawn", drawn.len());
}

#[test]
fn graph_columns__by_default_each_row_shows_the_author_the_date_and_the_short_hash() {
    let defaults = Settings::default().switches;
    for column in [TimelineSwitch::Author, TimelineSwitch::Date, TimelineSwitch::Hash] {
        assert!(defaults.is_on(column), "{column:?}");
    }
    let (repository, _remote) = repository_with_history();
    let state = showing(repository.path().to_path_buf());
    let mut window = Window::new(ThemeName::Dark, WIDE);
    let _ = window.draw(&state, &[]);
    let all = window.frame.to_rgba();
    for column in [TimelineSwitch::Author, TimelineSwitch::Date, TimelineSwitch::Hash] {
        let mut without = showing(repository.path().to_path_buf());
        let _effects = run(
            &mut without,
            Message::Intent(Intent::Graph(GraphIntent::Switch(column, false))),
        );
        let mut window = Window::new(ThemeName::Dark, WIDE);
        let _ = window.draw(&without, &[]);
        assert_ne!(window.frame.to_rgba(), all, "{column:?} is drawn");
    }
}

#[test]
fn graph_columns__unchecking_show_the_date_removes_the_dates_and_writes_the_columns_left() {
    let (repository, _remote) = repository_with_history();
    let mut state = showing_with(repository.path().to_path_buf(), Settings::default());
    let effects = run(
        &mut state,
        Message::Intent(Intent::Graph(GraphIntent::Switch(TimelineSwitch::Date, false))),
    );
    assert!(!state.settings.switches.is_on(TimelineSwitch::Date));
    let columns = json::Value::Array(vec![
        json::Value::String("author".into()),
        json::Value::String("hash".into()),
    ]);
    assert_eq!(written(&effects), Some(("graph.columns", columns)));
}

#[test]
fn graph_load_more__after_a_page_is_added_the_selected_commit_and_the_scroll_position_are_as_they_were() {
    let repository = imported(&line(150));
    let mut state = showing_with(repository.path().to_path_buf(), pages_of_100());
    assert_eq!(state.timeline.loaded(), 100);
    let chosen = state.timeline.rows[3].id().unwrap().clone();
    apply(
        &mut state,
        vec![Intent::Graph(GraphIntent::Select(Selected::Commit(chosen.clone())))],
    );
    let reveal = state.timeline.reveal;
    let first: Vec<Row> = state.timeline.rows.clone();
    apply(&mut state, vec![Intent::Graph(GraphIntent::LoadMore { loaded: 100 })]);
    assert_eq!(state.timeline.loaded(), 150);
    assert_eq!(state.timeline.selected, Some(Selected::Commit(chosen)));
    assert_eq!(state.timeline.reveal, reveal, "nothing scrolls the list");
    assert_eq!(state.timeline.rows[..100], first[..]);
}

#[test]
fn graph_load_more__the_last_row_says_loading_more_commits_while_there_are_more_and_is_gone_at_the_end() {
    let repository = imported(&line(150));
    let mut state = showing_with(repository.path().to_path_buf(), pages_of_100());
    let mut window = Window::new(ThemeName::Dark, WIDE);
    let (_, tree) = window.draw(&state, &[]);
    let list = bounds(&tree, Role::ListBox, "History");
    let intents = wheel(&mut window, &state, super::pointer::middle(list), 10_000.0);
    let (_, tree) = window.draw(&state, &[]);
    assert!(says(&tree, "Loading more commits…"));
    assert!(state.timeline.has_more);
    apply(&mut state, intents);
    let _ = wheel(&mut window, &state, super::pointer::middle(list), 10_000.0);
    let (_, tree) = window.draw(&state, &[]);
    assert_eq!(state.timeline.loaded(), 150);
    assert!(!state.timeline.has_more);
    assert!(!says(&tree, "Loading more commits…"));
}

#[test]
fn graph_uncommitted_row__changing_a_file_adds_the_draft_above_the_commit_checked_out_with_a_dashed_line_to_it() {
    let (repository, _remote) = repository_with_history();
    std::fs::write(repository.path().join("README.md"), "Gitardo, changed\n").unwrap();
    let state = showing(repository.path().to_path_buf());
    let head = state.shown_read().unwrap().state.head.commit.clone();
    assert!(matches!(state.timeline.rows[0], Row::Uncommitted { ref parent, files: 1 } if *parent == head));
    assert_eq!(state.timeline.draft.map(|draft| draft.files), Some(1));
    let below = &state.timeline.layout.cells[0];
    assert!(
        below.lines.iter().any(|line| line.is_dashed),
        "a dashed line goes down from the draft"
    );
    assert_eq!(views::text::timeline::files_changed(1, 0), "1 file changed");
}

#[test]
fn graph_uncommitted_row__in_a_new_repository_with_one_untracked_file_the_draft_is_the_only_card_with_no_line_down() {
    let repository = testkit::Repository::new();
    std::fs::write(repository.path().join("notes.txt"), "first\n").unwrap();
    let state = showing(repository.path().to_path_buf());
    assert!(matches!(
        state.timeline.rows[..],
        [Row::Uncommitted { parent: None, files: 1 }]
    ));
    let cell = &state.timeline.layout.cells[0];
    assert!(cell.lines.is_empty(), "{cell:?}");
    let mut window = Window::new(ThemeName::Dark, WIDE);
    let (_, tree) = window.draw(&state, &[]);
    assert_eq!(rows_drawn(&tree).len(), 1);
}

#[test]
fn graph_uncommitted_row__narrowed_to_a_branch_that_is_not_checked_out_the_draft_is_not_there() {
    let (repository, _remote) = repository_with_history();
    std::fs::write(repository.path().join("README.md"), "Gitardo, changed\n").unwrap();
    let state = showing(repository.path().to_path_buf());
    let read = &state.shown_read().unwrap().state;
    let head = read.head.commit.clone().unwrap();
    let feature: Vec<Row> = state
        .timeline
        .read_rows()
        .iter()
        .filter(|row| row.id() != Some(&head))
        .cloned()
        .collect();
    assert!(draft_of(read, &feature).is_none());
    assert!(draft_of(read, state.timeline.read_rows()).is_some());
}

#[test]
fn graph_stashes__git_stash_adds_a_square_above_the_commit_checked_out_labeled_stash_0() {
    let (repository, _remote) = repository_with_history();
    let state = showing(repository.path().to_path_buf());
    let head = state.shown_read().unwrap().state.head.commit.clone().unwrap();
    assert!(matches!(state.timeline.rows[0], Row::Stash { index: 0, .. }));
    assert_eq!(state.timeline.rows[1].id(), Some(&head));
    let mut window = Window::new(ThemeName::Dark, WIDE);
    let (_, tree) = window.draw(&state, &[]);
    let first = rows_drawn(&tree)[0];
    let label = super::pointer::said_at(&tree, "stash@{0}", first);
    assert!(label.y >= first.y && label.bottom() <= first.bottom());
}

#[test]
fn graph_stashes__unchecking_show_stashes_takes_them_out_and_sets_graph_show_stashes_to_false() {
    let (repository, _remote) = repository_with_history();
    let mut state = showing_with(repository.path().to_path_buf(), Settings::default());
    let switch = GraphIntent::Switch(TimelineSwitch::ShowStashes, false);
    let effects = run(&mut state, Message::Intent(Intent::Graph(switch)));
    assert_eq!(written(&effects), Some(("graph.showStashes", json::Value::Bool(false))));
    assert!(state.timeline.rows.iter().all(|row| !matches!(row, Row::Stash { .. })));
    assert_eq!(state.timeline.rows.len(), 3);
}
