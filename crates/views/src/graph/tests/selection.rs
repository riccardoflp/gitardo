//! Several commits selected at once: what a click with `Ctrl`, `Cmd` or `Shift` asks, the bar that counts them and the
//! menu of all of them.

use model::commit::Commit;
use model::contract::{FeatureId, Target};
use model::graph::Row;
use model::state::Operation;
use model::{CommitId, Timestamp};
use ui::Modifiers;

use super::super::intent::{GraphIntent, Selected};
use super::{Fixture, id, repository, rows};
use crate::feature::FeatureIntent;
use crate::intent::Intent;
use crate::selection::{Click, SelectionAction, Several, bar_entries, clicked, entry_intents, menu_entries};
use crate::text::history as words;

/// Six commits, `A` the newest, checked out on `main` at `A`, with `A` selected alone.
fn six() -> Fixture {
    Fixture::new(
        rows(&["A: B", "B: C", "C: D", "D: E", "E: F", "F:"]),
        &repository("A", &[("main", "A")]),
        Some(Selected::Commit(id("A"))),
    )
}

/// The row shown of a commit.
fn place(fixture: &Fixture, name: &str) -> usize {
    fixture.layout.shown_of.get(&id(name)).copied().unwrap_or(usize::MAX)
}

/// What a click on a commit asks.
fn click(fixture: &Fixture, name: &str, kind: Click) -> Option<GraphIntent> {
    clicked(&fixture.history(), place(fixture, name), kind)
}

/// The commits of a selection of several, by their names.
fn several(names: &[&str]) -> Vec<CommitId> {
    names.iter().map(|name| id(name)).collect()
}

/// The labels of the buttons of the bar of a fixture whose selection of several is set.
fn bar_labels(fixture: &Fixture) -> Vec<String> {
    let history = fixture.history();
    Several::of(&history).map_or_default(|chosen| {
        bar_entries(&history, &chosen)
            .into_iter()
            .map(|entry| entry.label)
            .collect()
    })
}

#[test]
fn selection_select__with_a_selected_ctrl_click_on_c_selects_a_and_c_and_the_bar_counts_two() {
    let mut fixture = six();
    let asked = click(&fixture, "C", Click::Toggle);
    assert_eq!(
        asked,
        Some(GraphIntent::SelectSeveral {
            commits: several(&["A", "C"]),
            shown: id("C")
        })
    );
    fixture.several = several(&["A", "C"]);
    let history = fixture.history();
    let chosen = Several::of(&history);
    assert_eq!(chosen.as_ref().map(|chosen| chosen.commits.len()), Some(2));
    assert_eq!(words::commits_selected(2), "2 commits selected");
}

#[test]
fn selection_select__ctrl_click_on_c_again_leaves_a_selected_alone_and_the_bar_goes_away() {
    let mut fixture = six();
    fixture.several = several(&["A", "C"]);
    assert_eq!(
        click(&fixture, "C", Click::Toggle),
        Some(GraphIntent::Select(Selected::Commit(id("A"))))
    );
    fixture.several = several(&["A"]);
    assert_eq!(Several::of(&fixture.history()), None);
}

#[test]
fn selection_select__shift_click_on_the_fourth_row_below_selects_five_or_fewer_with_a_stash_among_them() {
    let fixture = six();
    let Some(GraphIntent::SelectSeveral { commits, shown }) = click(&fixture, "E", Click::Extend) else {
        panic!("a Shift+click selects several")
    };
    assert_eq!(commits, several(&["A", "B", "C", "D", "E"]));
    assert_eq!(shown, id("E"));
    let stash = Row::Stash {
        commit: Commit {
            id: id("S"),
            parents: vec![id("C")],
            author_name: String::from("Author"),
            author_email: String::new(),
            author_date: Timestamp(0),
            subject: String::from("On main: work"),
        },
        index: 0,
    };
    let mut with_stash = rows(&["A: B", "B: C"]);
    with_stash.push(stash);
    with_stash.extend(rows(&["C: D", "D: E", "E: F", "F:"]));
    let fixture = Fixture::new(
        with_stash,
        &repository("A", &[("main", "A")]),
        Some(Selected::Commit(id("A"))),
    );
    assert_eq!(place(&fixture, "D"), 4, "the stash is the third row");
    let Some(GraphIntent::SelectSeveral { commits, .. }) = click(&fixture, "D", Click::Extend) else {
        panic!("a Shift+click selects several")
    };
    assert_eq!(commits, several(&["A", "B", "C", "D"]));
}

#[test]
fn selection_select__shift_click_across_the_draft_leaves_the_draft_out() {
    let mut all = vec![Row::Uncommitted {
        parent: Some(id("A")),
        files: 1,
    }];
    all.extend(rows(&["A: B", "B: C", "C:"]));
    let fixture = Fixture::new(all, &repository("A", &[("main", "A")]), Some(Selected::Commit(id("C"))));
    let Some(GraphIntent::SelectSeveral { commits, .. }) = click(&fixture, "A", Click::Extend) else {
        panic!("a Shift+click selects several")
    };
    assert_eq!(commits, several(&["C", "A", "B"]));
    assert_eq!(
        clicked(&fixture.history(), 0, Click::Extend),
        None,
        "the draft is selected alone"
    );
}

#[test]
fn selection_select__on_macos_cmd_click_does_what_ctrl_click_does_elsewhere() {
    assert_eq!(Click::of(Modifiers::META, true), Click::Toggle);
    assert_eq!(Click::of(Modifiers::CONTROL, false), Click::Toggle);
    assert_eq!(Click::of(Modifiers::META, false), Click::Plain);
    assert_eq!(Click::of(Modifiers::SHIFT, true), Click::Extend);
    assert_eq!(Click::of(Modifiers::NONE, true), Click::Plain);
}

#[test]
fn selection_bar__three_commits_that_follow_one_another_before_head_have_squash_into_one() {
    let mut fixture = six();
    fixture.several = several(&["D", "E", "F"]);
    assert_eq!(
        bar_labels(&fixture),
        [
            "Squash into one",
            "Cherry-pick onto main",
            "Copy as patch",
            "Clear selection"
        ]
    );
    fixture.several = several(&["B", "D", "F"]);
    assert_eq!(
        bar_labels(&fixture),
        ["Cherry-pick onto main", "Copy as patch", "Clear selection"]
    );
    fixture.several = several(&["E", "F"]);
    let history = fixture.history();
    let chosen = Several::of(&history).unwrap_or_else(|| panic!("two are selected"));
    let (entries, _) = menu_entries(&history, &chosen);
    assert_eq!(
        entries.first().map(|entry| entry_intents(entry, &chosen)),
        Some(vec![
            Intent::Feature(FeatureIntent::Run(
                FeatureId("squash.commits"),
                Target::Commits(several(&["E", "F"]))
            )),
            Intent::Graph(GraphIntent::ClearSelection),
        ])
    );
    assert_eq!(
        entries.first().map(|entry| entry.label.as_str()),
        Some("Squash 2 commits into one\u{2026}")
    );
}

#[test]
fn selection_bar__with_a_merge_commit_among_the_selection_neither_cherry_pick_nor_copy_as_patch_is_there() {
    let mut fixture = Fixture::new(
        rows(&["A: B", "B: C X", "X: C", "C:"]),
        &repository("A", &[("main", "A")]),
        Some(Selected::Commit(id("A"))),
    );
    fixture.several = several(&["X", "C"]);
    assert_eq!(
        bar_labels(&fixture),
        ["Cherry-pick onto main", "Copy as patch", "Clear selection"]
    );
    fixture.several = several(&["B", "C"]);
    assert_eq!(bar_labels(&fixture), ["Clear selection"]);
}

#[test]
fn selection_bar__while_a_rebase_is_stopped_cherry_pick_is_not_there() {
    let mut state = repository("A", &[("main", "A")]);
    state.operation = Some(Operation::Rebase);
    let mut fixture = Fixture::new(rows(&["A: B", "B: C", "C:"]), &state, Some(Selected::Commit(id("A"))));
    fixture.several = several(&["B", "C"]);
    assert_eq!(bar_labels(&fixture), ["Copy as patch", "Clear selection"]);
}

#[test]
fn selection_bar__clear_selection_empties_the_selection_of_several() {
    let mut fixture = six();
    fixture.several = several(&["A", "C"]);
    let history = fixture.history();
    let chosen = Several::of(&history).unwrap_or_else(|| panic!("two are selected"));
    let entries = bar_entries(&history, &chosen);
    let clear = entries.iter().find(|entry| entry.action == SelectionAction::Clear);
    assert_eq!(
        clear.map(|entry| entry_intents(entry, &chosen)),
        Some(vec![Intent::Graph(GraphIntent::ClearSelection)])
    );
    let pick = entries.iter().find(|entry| entry.runs_an_operation());
    assert_eq!(
        pick.map(|entry| entry_intents(entry, &chosen)),
        Some(vec![
            Intent::Feature(FeatureIntent::Run(
                FeatureId("selection.cherry-pick"),
                Target::Commits(several(&["A", "C"]))
            )),
            Intent::Graph(GraphIntent::ClearSelection),
        ]),
        "a cherry-pick empties the selection too"
    );
}

#[test]
fn selection_menu__copy_hashes_with_three_selected_copies_them_newest_first() {
    let mut fixture = six();
    fixture.several = several(&["D", "A", "B"]);
    let history = fixture.history();
    let chosen = Several::of(&history).unwrap_or_else(|| panic!("three are selected"));
    let (entries, separator) = menu_entries(&history, &chosen);
    let labels: Vec<&str> = entries.iter().map(|entry| entry.label.as_str()).collect();
    assert_eq!(
        labels,
        [
            "Cherry-pick 3 commits onto main",
            "Copy hashes",
            "Copy as patch",
            "Save as patch\u{2026}"
        ]
    );
    assert_eq!(separator, 1);
    let copy = entries.iter().find(|entry| entry.action == SelectionAction::CopyHashes);
    assert_eq!(
        copy.map(|entry| entry_intents(entry, &chosen)),
        Some(vec![Intent::Graph(GraphIntent::CopyHashes(several(&["A", "B", "D"])))])
    );
}

/// The words of the menu of a fixture whose selection of several is set.
fn menu_labels(fixture: &Fixture) -> Vec<String> {
    let history = fixture.history();
    Several::of(&history).map_or_default(|chosen| {
        menu_entries(&history, &chosen)
            .0
            .into_iter()
            .map(|entry| entry.label)
            .collect()
    })
}

#[test]
fn selection_menu__compare_these_two_commits_opens_a_comparison_whose_first_side_is_the_lower() {
    let mut fixture = six();
    fixture.several = several(&["A", "C"]);
    let history = fixture.history();
    let chosen = Several::of(&history).unwrap_or_else(|| panic!("two are selected"));
    let (entries, _) = menu_entries(&history, &chosen);
    let compare = entries.iter().find(|entry| entry.action == SelectionAction::CompareTwo);
    let answers = model::contract::Answers::new()
        .with(
            model::contract::AnswerKey("second"),
            model::contract::Answer::Choice(String::from("revision")),
        )
        .with(
            model::contract::AnswerKey("second-revision"),
            model::contract::Answer::Text(id("A").as_str().to_owned()),
        );
    assert_eq!(
        compare.map(|entry| (entry.label.as_str(), entry_intents(entry, &chosen))),
        Some((
            "Compare these two commits",
            vec![Intent::Feature(FeatureIntent::RunWith(
                FeatureId("compare.with"),
                Target::Commit(id("C")),
                answers
            ))]
        ))
    );
}

#[test]
fn selection_menu__with_three_selected_compare_these_two_commits_is_not_there() {
    let mut fixture = six();
    fixture.several = several(&["A", "C"]);
    assert!(menu_labels(&fixture).contains(&String::from("Compare these two commits")));
    fixture.several = several(&["A", "C", "E"]);
    assert!(!menu_labels(&fixture).contains(&String::from("Compare these two commits")));
}
