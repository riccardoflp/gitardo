//! The merges folded and unfolded, the first parent only, and the history brought out of the selected commit and of a
//! label, from the store and the window drawn from it.

use std::fmt::Write as _;

use app::persisted::Persisted;
use app::settings::Settings;
use app::store::{Message, State};
use model::CommitId;
use model::graph::{Row, foldable_merges};
use platform::accesskit::Role;
use render::ThemeName;
use testkit::Repository;
use views::Intent;
use views::graph::{GraphIntent, HoverTarget, HoverWay, Selected, TimelineSwitch};

use super::histories::{Commit, imported, line, merges, write_commit};
use super::pointer::{apply, bounds, click_at, middle, rows_drawn, said_at};
use super::showing;
use super::support::{Window, run};
use super::timeline::{pages_of_100, showing_with, subjects, written};

/// The size of the window of these tests.
const WIDE: (u32, u32) = (1000, 520);

/// `main` with a commit, `feature` with three from it, a commit on `main`, then `feature` merged into `main`.
pub fn merged_feature() -> Repository {
    let mut repository = Repository::new();
    let _ = repository.commit("Start", &[("README.md", "start\n")]);
    repository.branch("feature");
    repository.checkout("feature");
    for number in 1..=3 {
        let _ = repository.commit(&format!("Feature {number}"), &[("feature.txt", &format!("{number}\n"))]);
    }
    repository.checkout("main");
    let _ = repository.commit("Move on", &[("main.txt", "on\n")]);
    repository.merge("feature");
    repository
}

/// The commit of the row whose subject is this.
fn commit_of(state: &State, subject: &str) -> CommitId {
    state
        .timeline
        .rows
        .iter()
        .filter_map(Row::commit)
        .find(|commit| commit.subject == subject)
        .map_or_else(|| panic!("no commit {subject}"), |commit| commit.id.clone())
}

/// Hands one intent of the timeline to the store.
fn ask(state: &mut State, intent: GraphIntent) -> Vec<app::store::Effect> {
    run(state, Message::Intent(Intent::Graph(intent)))
}

/// Whether the history brought out has the row of a commit in it; none when nothing is brought out.
fn is_brought_out(state: &State, commit: &CommitId) -> Option<bool> {
    let (cells, _) = state.timeline.trace.as_ref()?;
    let shown = state.timeline.layout.shown_of.get(commit)?;
    cells.get(*shown).map(|cell| cell.is_in)
}

#[test]
fn graph_fold_merge__folding_a_merge_of_three_commits_gives_a_row_3_commits_of_feature() {
    let repository = merged_feature();
    let mut state = showing(repository.path().to_path_buf());
    let merge = commit_of(&state, "Merge branch 'feature'");
    let _effects = ask(&mut state, GraphIntent::Fold(merge));
    assert_eq!(state.timeline.layout.len(), state.timeline.rows.len() - 2);
    let mut window = Window::new(ThemeName::Dark, WIDE);
    let (_, tree) = window.draw(&state, &[]);
    let list = bounds(&tree, Role::ListBox, "History");
    let _row = said_at(&tree, "3 commits of feature", list);
}

#[test]
fn graph_fold_merge__a_click_on_the_folded_row_shows_the_commits_again() {
    let repository = merged_feature();
    let mut state = showing(repository.path().to_path_buf());
    let merge = commit_of(&state, "Merge branch 'feature'");
    let _effects = ask(&mut state, GraphIntent::Fold(merge.clone()));
    let mut window = Window::new(ThemeName::Dark, WIDE);
    let (_, tree) = window.draw(&state, &[]);
    let list = bounds(&tree, Role::ListBox, "History");
    let folded = said_at(&tree, "3 commits of feature", list);
    let intents = click_at(&mut window, &state, middle(folded), 1);
    assert!(
        intents.contains(&Intent::Graph(GraphIntent::Unfold(merge))),
        "{intents:?}"
    );
    apply(&mut state, intents);
    assert!(state.timeline.folded.is_empty());
    assert_eq!(state.timeline.layout.len(), state.timeline.rows.len());
}

#[test]
fn graph_fold_merge__the_folds_are_there_after_the_window_is_closed_and_opened_again() {
    let repository = merged_feature();
    let mut state = showing(repository.path().to_path_buf());
    let merge = commit_of(&state, "Merge branch 'feature'");
    let effects = ask(&mut state, GraphIntent::Fold(merge.clone()));
    let Some(app::store::Effect::WritePersisted(kept)) = effects
        .into_iter()
        .find(|effect| matches!(effect, app::store::Effect::WritePersisted(_)))
    else {
        panic!("the folds are not kept");
    };
    let mut again = State::new(
        Persisted {
            has_started: true,
            ..kept
        },
        Settings::default(),
        None,
    );
    let _effects = run(
        &mut again,
        Message::Started {
            folders: vec![repository.path().to_path_buf()],
        },
    );
    assert!(again.timeline.folded.contains(&merge));
    assert_eq!(again.timeline.layout.len(), again.timeline.rows.len() - 2);
}

#[test]
fn graph_fold_all__fold_all_merges_folds_every_merge_loaded_and_those_of_the_next_page_when_it_comes() {
    let repository = imported(&merges(60));
    let mut state = showing_with(repository.path().to_path_buf(), pages_of_100());
    let _effects = ask(&mut state, GraphIntent::FoldAll);
    let loaded = foldable_merges(&state.timeline.rows);
    assert!(loaded.len() >= 30);
    assert!(loaded.iter().all(|merge| state.timeline.folded.contains(merge)));
    let _effects = ask(&mut state, GraphIntent::LoadMore { loaded: 100 });
    assert_eq!(state.timeline.loaded(), 181);
    let all = foldable_merges(&state.timeline.rows);
    assert_eq!(all.len(), 60);
    assert!(all.iter().all(|merge| state.timeline.folded.contains(merge)));
}

#[test]
fn graph_fold_all__a_merge_unfolded_by_hand_while_all_are_folded_stays_unfolded_after_the_next_page() {
    let repository = imported(&merges(60));
    let mut state = showing_with(repository.path().to_path_buf(), pages_of_100());
    let _effects = ask(&mut state, GraphIntent::FoldAll);
    let merge = commit_of(&state, "Merge topic 50");
    let _effects = ask(&mut state, GraphIntent::Unfold(merge.clone()));
    let _effects = ask(&mut state, GraphIntent::LoadMore { loaded: 100 });
    assert_eq!(state.timeline.loaded(), 181);
    assert!(!state.timeline.folded.contains(&merge));
    assert!(state.timeline.folded.contains(&commit_of(&state, "Merge topic 2")));
}

#[test]
fn graph_fold_all__unfold_all_merges_shows_every_commit_again() {
    let repository = imported(&merges(20));
    let mut state = showing(repository.path().to_path_buf());
    let _effects = ask(&mut state, GraphIntent::FoldAll);
    assert_eq!(state.timeline.layout.len(), state.timeline.rows.len());
    assert!(!state.timeline.folded.is_empty());
    let _effects = ask(&mut state, GraphIntent::UnfoldAll);
    assert!(state.timeline.folded.is_empty() && !state.timeline.is_all_folded);
    assert_eq!(state.timeline.layout.len(), state.timeline.rows.len());
}

#[test]
fn graph_first_parent__a_merge_of_feature_is_one_row_on_the_line_of_main_and_its_commits_are_listed_only_if_feature_is()
{
    let repository = merged_feature();
    let _ = repository.git(&["branch", "--quiet", "--delete", "feature"]);
    let mut state = showing_with(repository.path().to_path_buf(), Settings::default());
    let effects = ask(&mut state, GraphIntent::Switch(TimelineSwitch::FirstParent, true));
    assert_eq!(written(&effects), Some(("graph.firstParent", json::Value::Bool(true))));
    assert_eq!(subjects(&state), ["Merge branch 'feature'", "Move on", "Start"]);
    assert!(state.timeline.layout.cells.iter().all(|cell| cell.lane == 0));
    let kept = merged_feature();
    let mut state = showing_with(kept.path().to_path_buf(), Settings::default());
    let _effects = ask(&mut state, GraphIntent::Switch(TimelineSwitch::FirstParent, true));
    assert_eq!(state.timeline.rows.len(), 6, "feature itself reaches its commits");
    assert!(state.timeline.layout.foldable.is_empty());
}

/// `main` with `Start`, `Main 1` and `Main 2`; `feature` left it at `Start` and has `Feature 1`, made last.
pub fn feature_left_main() -> Repository {
    let mut repository = Repository::new();
    let _ = repository.commit("Start", &[("README.md", "start\n")]);
    repository.branch("feature");
    let _ = repository.commit("Main 1", &[("main.txt", "1\n")]);
    let _ = repository.commit("Main 2", &[("main.txt", "2\n")]);
    repository.checkout("feature");
    let _ = repository.commit("Feature 1", &[("feature.txt", "1\n")]);
    repository.checkout("main");
    repository
}

#[test]
fn graph_trace_selected__selecting_a_commit_on_feature_dims_the_commits_of_main_made_after_feature_left_it() {
    let repository = feature_left_main();
    let mut state = showing(repository.path().to_path_buf());
    let feature = commit_of(&state, "Feature 1");
    let _effects = ask(&mut state, GraphIntent::Select(Selected::Commit(feature.clone())));
    assert_eq!(is_brought_out(&state, &feature), Some(true));
    assert_eq!(is_brought_out(&state, &commit_of(&state, "Start")), Some(true));
    assert_eq!(is_brought_out(&state, &commit_of(&state, "Main 1")), Some(false));
    assert_eq!(is_brought_out(&state, &commit_of(&state, "Main 2")), Some(false));
}

#[test]
fn graph_trace_selected__selecting_the_draft_dims_nothing() {
    let repository = feature_left_main();
    std::fs::write(repository.path().join("main.txt"), "changed\n").unwrap();
    let mut state = showing(repository.path().to_path_buf());
    let feature = commit_of(&state, "Feature 1");
    let _effects = ask(&mut state, GraphIntent::Select(Selected::Commit(feature)));
    assert!(state.timeline.trace.is_some());
    let _effects = ask(&mut state, GraphIntent::Select(Selected::Draft));
    assert!(state.timeline.trace.is_none());
}

#[test]
fn graph_trace_selected__with_two_commits_selected_nothing_is_dimmed() {
    let repository = feature_left_main();
    let mut state = showing(repository.path().to_path_buf());
    let feature = commit_of(&state, "Feature 1");
    let _effects = ask(&mut state, GraphIntent::Select(Selected::Commit(feature.clone())));
    assert!(state.timeline.trace.is_some());
    let main = commit_of(&state, "Main 2");
    let _effects = ask(
        &mut state,
        GraphIntent::SelectSeveral {
            commits: vec![main.clone(), feature],
            shown: main,
        },
    );
    assert!(state.timeline.trace.is_none());
    let _effects = ask(&mut state, GraphIntent::ClearSelection);
    assert!(
        state.timeline.trace.is_some(),
        "one commit selected brings its history out again"
    );
}

#[test]
fn graph_date_separators__with_the_setting_off_there_is_no_separator() {
    let mut stream = String::new();
    for mark in 1..=3_u64 {
        let date = testkit::FIRST_COMMIT_TIME + mark * 40 * 86_400;
        let content = format!("content {mark}\n");
        write!(
            stream,
            "commit refs/heads/main\nmark :{mark}\ncommitter Test Author <author@example.com> {date} +0000\ndata 9\nChange {mark}\nM 644 inline file.txt\ndata {}\n{content}\n",
            content.len()
        )
        .unwrap();
    }
    let repository = imported(&stream);
    let mut state = showing_with(repository.path().to_path_buf(), Settings::default());
    assert_eq!(
        state.timeline.layout.separators.len(),
        2,
        "the commits are months apart"
    );
    let effects = ask(&mut state, GraphIntent::Switch(TimelineSwitch::DateSeparators, false));
    assert_eq!(
        written(&effects),
        Some(("graph.dateSeparators", json::Value::Bool(false)))
    );
    assert_eq!(state.timeline.layout.separators, std::collections::BTreeMap::new());
}

#[test]
fn graph_trace_selected__unchecking_the_entry_sets_graph_trace_selected_to_false_and_nothing_is_dimmed() {
    let repository = feature_left_main();
    let mut state = showing_with(repository.path().to_path_buf(), Settings::default());
    let feature = commit_of(&state, "Feature 1");
    let _effects = ask(&mut state, GraphIntent::Select(Selected::Commit(feature)));
    let effects = ask(&mut state, GraphIntent::Switch(TimelineSwitch::TraceSelected, false));
    assert_eq!(
        written(&effects),
        Some(("graph.traceSelected", json::Value::Bool(false)))
    );
    assert!(state.timeline.trace.is_none());
}

#[test]
fn graph_hover_history__with_shift_held_on_feature_only_the_commits_of_feature_not_in_the_one_checked_out_stay() {
    let repository = feature_left_main();
    let mut state = showing(repository.path().to_path_buf());
    let feature = commit_of(&state, "Feature 1");
    let target = HoverTarget {
        commit: feature.clone(),
        way: HoverWay::NotCheckedOut,
    };
    let _effects = ask(&mut state, GraphIntent::Hover(Some(target)));
    assert_eq!(is_brought_out(&state, &feature), Some(true));
    for other in ["Start", "Main 1", "Main 2"] {
        assert_eq!(
            is_brought_out(&state, &commit_of(&state, other)),
            Some(false),
            "{other}"
        );
    }
}

#[test]
fn graph_sticky_branch_names__clicking_it_scrolls_to_the_tip_and_selects_it() {
    let mut stream = line(40);
    write_commit(
        &mut stream,
        &Commit {
            branch: "feature",
            mark: 41,
            from: Some(5),
            merge: None,
            message: "Feature",
            file: "feature.txt",
        },
    );
    let repository = imported(&stream);
    let mut state = showing(repository.path().to_path_buf());
    let tip = commit_of(&state, "Feature");
    let mut window = Window::new(ThemeName::Dark, WIDE);
    let (_, tree) = window.draw(&state, &[]);
    let list = bounds(&tree, Role::ListBox, "History");
    let _ = super::pointer::wheel(&mut window, &state, middle(list), 600.0);
    let (_, tree) = window.draw(&state, &[]);
    let sticky = said_at(&tree, "feature", list);
    assert!(sticky.y < rows_drawn(&tree)[1].y, "the name is at the top of the list");
    let intents = click_at(&mut window, &state, middle(sticky), 1);
    assert!(
        intents.contains(&Intent::Graph(GraphIntent::GoTo(tip.clone()))),
        "{intents:?}"
    );
    apply(&mut state, intents);
    assert_eq!(state.timeline.selected, Some(Selected::Commit(tip)));
    let (_, tree) = window.draw(&state, &[]);
    let first = rows_drawn(&tree)[0];
    let _label = said_at(&tree, "feature", first);
}
