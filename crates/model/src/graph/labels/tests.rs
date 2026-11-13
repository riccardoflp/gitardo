#![allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]

use std::path::PathBuf;

use super::{GraphSettings, Label, LabelKind, labels};
use crate::Timestamp;
use crate::graph::testing::{branch, id, remote_branch};
use crate::state::{Head, RepositoryState, Stash, Tag, Worktree};

/// A repository with `main` checked out at `A`.
fn on_main() -> RepositoryState {
    let mut main = branch("main", "A", None);
    main.is_checked_out = true;
    RepositoryState {
        head: Head {
            commit: Some(id("A")),
            branch: Some(String::from("refs/heads/main")),
        },
        branches: vec![main],
        ..RepositoryState::default()
    }
}

fn tag(name: &str, commit: &str) -> Tag {
    Tag {
        name: name.to_owned(),
        commit: id(commit),
        date: Timestamp(0),
    }
}

/// The names of the labels of a commit, in order.
fn names(state: &RepositoryState, settings: GraphSettings, commit: &str) -> Vec<String> {
    labels(state, &settings)
        .remove(&id(commit))
        .unwrap_or_default()
        .into_iter()
        .map(|label| label.name)
        .collect()
}

#[test]
fn graph_labels__detached_at_a_commit_it_has_a_filled_head_label() {
    let mut state = on_main();
    state.head.branch = None;
    state.head.commit = Some(id("B"));
    state.branches[0].is_checked_out = false;
    let found = labels(&state, &GraphSettings::default());
    assert_eq!(
        found[&id("B")],
        [Label {
            name: String::from("HEAD"),
            kind: LabelKind::Head,
            is_checked_out: true,
        }]
    );
    assert!(!found[&id("A")][0].is_checked_out);
}

#[test]
fn graph_labels__the_branch_checked_out_comes_filled_before_a_tag_at_its_commit() {
    let mut state = on_main();
    state.branches.insert(0, branch("alpha", "A", None));
    state.tags.push(tag("v1.0", "A"));
    let found = labels(&state, &GraphSettings::default());
    let at_a = &found[&id("A")];
    assert_eq!(names(&state, GraphSettings::default(), "A"), ["main", "alpha", "v1.0"]);
    assert!(at_a[0].is_checked_out);
    assert!(!at_a[1].is_checked_out && !at_a[2].is_checked_out);
}

#[test]
fn graph_labels__a_branch_checked_out_in_another_worktree_names_its_folder() {
    let mut state = on_main();
    state.branches.push(branch("feature", "B", None));
    state.worktrees.push(Worktree {
        folder: PathBuf::from("/work/feature"),
        commit: Some(id("B")),
        branch: Some(String::from("refs/heads/feature")),
        is_main: false,
        is_current: false,
        is_missing: false,
        uncommitted: Some(0),
    });
    let found = labels(&state, &GraphSettings::default());
    let LabelKind::Branch { worktree, .. } = &found[&id("B")][0].kind else {
        panic!("not a branch: {:?}", found[&id("B")]);
    };
    assert_eq!(worktree.as_deref(), Some(PathBuf::from("/work/feature").as_path()));
}

#[test]
fn graph_labels__a_remote_head_has_no_label() {
    let mut state = on_main();
    state.remote_branches = vec![remote_branch("origin/HEAD", "A"), remote_branch("origin/main", "A")];
    assert_eq!(names(&state, GraphSettings::default(), "A"), ["main", "origin/main"]);
}

#[test]
fn graph_labels__a_tag_labels_the_commit_it_points_at_and_a_stash_its_own_commit() {
    let mut state = on_main();
    state.tags.push(tag("v1.0.0", "A"));
    state.stashes.push(Stash {
        commit: id("S"),
        index: 0,
        message: String::from("On main: draft"),
        date: Timestamp(0),
    });
    assert_eq!(names(&state, GraphSettings::default(), "A"), ["main", "v1.0.0"]);
    assert_eq!(names(&state, GraphSettings::default(), "S"), ["stash@{0}"]);
}

#[test]
fn graph_combine_upstream__a_shared_label_is_the_branch_with_its_upstream_and_no_remote_label() {
    let mut state = on_main();
    state.branches.push(branch("feature", "B", Some(("origin", "feature"))));
    state.remote_branches.push(remote_branch("origin/feature", "B"));
    let found = labels(&state, &GraphSettings::default());
    assert_eq!(
        found[&id("B")],
        [Label {
            name: String::from("feature"),
            kind: LabelKind::Branch {
                worktree: None,
                upstream: Some(String::from("origin/feature")),
            },
            is_checked_out: false,
        }]
    );
}

#[test]
fn graph_combine_upstream__with_the_setting_off_each_has_its_label() {
    let mut state = on_main();
    state.branches.push(branch("feature", "B", Some(("origin", "feature"))));
    state.remote_branches.push(remote_branch("origin/feature", "B"));
    let settings = GraphSettings {
        combine_upstream: false,
    };
    assert_eq!(names(&state, settings, "B"), ["feature", "origin/feature"]);
}

#[test]
fn bisect_marks__after_marking_e_bad_and_a_good_e_says_bad_and_a_good() {
    let mut state = on_main();
    state.tags.push(tag("v1.0", "E"));
    state.bisect = Some(crate::bisect::Bisect {
        bad: Some(id("E")),
        good: vec![id("A")],
        skipped: vec![id("C")],
        remaining: Some(4),
    });
    let settings = GraphSettings::default();
    assert_eq!(names(&state, settings, "E"), ["v1.0", "bad"]);
    assert_eq!(names(&state, settings, "A"), ["main", "good"]);
    assert_eq!(names(&state, settings, "C"), ["skipped"]);
    let marked = labels(&state, &settings).remove(&id("E")).unwrap_or_default();
    assert!(matches!(
        marked.last().map(|label| &label.kind),
        Some(LabelKind::Bisect(super::BisectMark::Bad))
    ));
}

#[test]
fn bisect_marks__once_the_bisect_ended_the_marks_are_gone() {
    let state = on_main();
    assert_eq!(names(&state, GraphSettings::default(), "A"), ["main"]);
}
