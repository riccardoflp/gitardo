#![allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]

use std::collections::{HashMap, HashSet};

use super::{FoldedRows, ShownRow, fold_rows, foldable_merges};
use crate::CommitId;
use crate::graph::labels::{Label, LabelKind};
use crate::graph::lanes::{Half, lanes};
use crate::graph::row::Row;
use crate::graph::testing::{id, rows};

/// Rows written `X: P Q`, the merge `M` with the message given.
fn with_merge_message(lines: &[&str], message: &str) -> Vec<Row> {
    let mut rows = rows(lines);
    for row in &mut rows {
        if let Row::Commit(commit) = row
            && commit.id == id("M")
        {
            message.clone_into(&mut commit.subject);
        }
    }
    rows
}

/// The merges folded, by name.
fn folding(names: &[&str]) -> HashSet<CommitId> {
    names.iter().map(|name| id(name)).collect()
}

/// What each row shown is, as the tests name them: the name of a commit, or `[n]` for a fold of n commits.
fn shown(rows: &[Row], folded: &FoldedRows) -> Vec<String> {
    folded
        .rows
        .iter()
        .map(|shown| match shown {
            ShownRow::Row { index, .. } => rows[*index]
                .commit()
                .unwrap()
                .subject
                .trim_start_matches("Commit ")
                .to_owned(),
            ShownRow::Fold(fold) => format!("[{}]", fold.commits.len()),
        })
        .collect()
}

#[test]
fn graph_fold_rows__a_merge_of_three_commits_folds_into_one_row_where_the_first_was() {
    let lines = ["M: B F3", "F3: F2", "B: A", "F2: F1", "F1: A", "A:"];
    let rows = with_merge_message(&lines, "Merge branch 'feature'");
    let folded = fold_rows(&rows, &folding(&["M"]));
    assert_eq!(shown(&rows, &folded), ["Merge branch 'feature'", "[3]", "B", "A"]);
    let ShownRow::Fold(fold) = &folded.rows[1] else {
        panic!("not a fold: {:?}", folded.rows[1]);
    };
    assert_eq!(fold.title(&HashMap::new()), "3 commits of feature");
    assert_eq!(fold.parents, [id("A")]);
    assert_eq!(folded.folded_by[&id("F2")], id("M"));
    let inputs = folded.graph_inputs(&rows);
    let cells = lanes(&inputs);
    let merge_lines = &cells[0].lines;
    let to_fold = merge_lines
        .iter()
        .find(|line| line.half == Half::Lower && line.to != cells[0].lane)
        .unwrap();
    assert_eq!(to_fold.to, cells[1].lane, "the merge's line goes to the folded row");
}

#[test]
fn graph_fold_rows__a_commit_made_on_top_of_a_merged_one_further_down_keeps_them_under_it() {
    let lines = ["M: B F3", "F3: F2", "F2: F1", "X: F1", "F1: A", "B: A", "A:"];
    let rows = with_merge_message(&lines, "Merge branch 'feature'");
    let folded = fold_rows(&rows, &folding(&["M"]));
    assert_eq!(shown(&rows, &folded), ["Merge branch 'feature'", "X", "[3]", "B", "A"]);
    let ShownRow::Row { parents, .. } = &folded.rows[1] else {
        panic!("not a row: {:?}", folded.rows[1]);
    };
    assert_eq!(parents.as_deref(), Some(&[id("F3")][..]));
}

#[test]
fn graph_fold_rows__a_merge_whose_second_parent_is_in_the_first_parents_history_brought_nothing_in() {
    let rows = with_merge_message(&["M: B A", "B: A", "A:"], "Merge branch 'old'");
    let folded = fold_rows(&rows, &folding(&["M"]));
    assert_eq!(shown(&rows, &folded), ["Merge branch 'old'", "B", "A"]);
    assert!(folded.folded_by.is_empty());
}

#[test]
fn graph_fold_rows__a_pull_request_with_no_label_is_named_from_its_message() {
    let lines = ["M: B F1", "F1: A", "B: A", "A:"];
    let rows = with_merge_message(&lines, "Merge pull request #7 from octo/fix");
    let folded = fold_rows(&rows, &folding(&["M"]));
    let ShownRow::Fold(fold) = &folded.rows[1] else {
        panic!("not a fold: {:?}", folded.rows[1]);
    };
    assert_eq!(fold.title(&HashMap::new()), "1 commit of octo/fix");
}

#[test]
fn graph_fold_rows__the_branch_at_the_second_parent_names_the_fold_before_the_message() {
    let lines = ["M: B F1", "F1: A", "B: A", "A:"];
    let rows = with_merge_message(&lines, "Merge pull request #7 from octo/fix");
    let folded = fold_rows(&rows, &folding(&["M"]));
    let ShownRow::Fold(fold) = &folded.rows[1] else {
        panic!("not a fold: {:?}", folded.rows[1]);
    };
    let label = |name: &str, kind| Label {
        name: name.to_owned(),
        kind,
        is_checked_out: false,
    };
    let labels = HashMap::from([(
        id("F1"),
        vec![
            label("origin/fix", LabelKind::RemoteBranch),
            label(
                "fix",
                LabelKind::Branch {
                    worktree: None,
                    upstream: None,
                },
            ),
        ],
    )]);
    assert_eq!(fold.title(&labels), "1 commit of fix");
    let merged = with_merge_message(&lines, "Squash the work");
    let folded = fold_rows(&merged, &folding(&["M"]));
    let ShownRow::Fold(fold) = &folded.rows[1] else {
        panic!("not a fold: {:?}", folded.rows[1]);
    };
    assert_eq!(fold.title(&HashMap::new()), "1 commit merged");
}

#[test]
fn graph_fold_rows__a_commit_put_away_by_a_merge_folded_first_is_not_put_away_again() {
    // M2 merges G, which had merged F (M1); folding both, M2 takes all, M1 among them.
    let lines = ["M2: B G2", "G2: M1", "M1: G1 F1", "F1: A", "G1: A", "B: A", "A:"];
    let rows = rows(&lines);
    let folded = fold_rows(&rows, &folding(&["M2", "M1"]));
    assert_eq!(shown(&rows, &folded), ["M2", "[4]", "B", "A"]);
    assert_eq!(folded.folded_by[&id("F1")], id("M2"));
    assert_eq!(folded.folded_by[&id("M1")], id("M2"));
}

#[test]
fn graph_fold_rows__the_merges_that_can_fold_are_those_that_brought_commits_in() {
    let lines = ["N: M A", "M: B F1", "F1: A", "B: A", "A:"];
    let rows = with_merge_message(&lines, "Merge branch 'feature'");
    let foldable = foldable_merges(&rows);
    assert_eq!(
        foldable,
        folding(&["M"]),
        "N's second parent is in its first parent's history"
    );
}
