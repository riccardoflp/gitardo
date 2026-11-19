use std::path::{Path, PathBuf};

use super::{UpdateRefsInput, update_refs};
use crate::CommitId;
use crate::rebase::{GitVersion, StackedBranch, StackedBranches};

fn version(minor: u32) -> GitVersion {
    GitVersion {
        major: 2,
        minor,
        patch: 0,
        said: format!("2.{minor}.0"),
    }
}

fn stack(git_setting: Option<bool>, p2_elsewhere: bool) -> StackedBranches {
    let branch = |name: &str, digit: char, worktree: Option<&str>| StackedBranch {
        name: name.to_owned(),
        commit: CommitId::parse(&digit.to_string().repeat(40)).unwrap(),
        worktree: worktree.map(PathBuf::from),
    };
    StackedBranches {
        branches: vec![
            branch("p1", 'b', None),
            branch("p2", 'c', p2_elsewhere.then_some("C:/work/p2")),
        ],
        update_refs: git_setting,
    }
}

fn input<'a>(version: &'a GitVersion, setting: Option<bool>, stacked: &'a StackedBranches) -> UpdateRefsInput<'a> {
    UpdateRefsInput {
        version: Some(version),
        setting,
        stacked,
        worktree: Path::new("C:/work/app"),
    }
}

#[test]
fn rebase_update_refs__the_branches_under_the_tip_move_with_the_rebase() {
    let (version, stacked) = (version(53), stack(None, false));
    let decided = update_refs(&input(&version, None, &stacked));
    assert_eq!(decided.option, Some("--update-refs"));
    assert_eq!(decided.moving, ["p1", "p2"]);
    assert_eq!(
        decided.note.as_deref(),
        Some("These branches move with it: `p1`, `p2`.")
    );
}

#[test]
fn rebase_update_refs__gitardo_s_setting_off_over_git_s_on_gives_no_update_refs_and_says_they_stay() {
    let (version, stacked) = (version(53), stack(Some(true), false));
    let decided = update_refs(&input(&version, Some(false), &stacked));
    assert_eq!(decided.option, Some("--no-update-refs"));
    assert_eq!(decided.staying, ["p1", "p2"]);
    assert_eq!(decided.note.as_deref(), Some("`p1` and `p2` stay on the old commits."));
}

#[test]
fn rebase_update_refs__with_no_other_branch_nothing_changes() {
    let (version, stacked) = (version(53), StackedBranches::default());
    assert_eq!(
        update_refs(&input(&version, None, &stacked)),
        super::UpdateRefs::default()
    );
}

#[test]
fn rebase_update_refs__git_2_37_has_no_option_and_says_which_stay_and_why() {
    let (version, stacked) = (version(37), stack(None, false));
    let decided = update_refs(&input(&version, None, &stacked));
    assert_eq!(decided.option, None);
    assert_eq!(decided.staying, ["p1", "p2"]);
    assert_eq!(
        decided.note.as_deref(),
        Some(
            "Git 2.37.0 cannot move `p1` and `p2` with the rewrite (2.38 or later can): they stay on the old commits."
        )
    );
    let unknown = UpdateRefsInput {
        version: None,
        ..input(&version, None, &stacked)
    };
    assert_eq!(
        update_refs(&unknown).option,
        None,
        "a version not read is taken as too old"
    );
}

#[test]
fn rebase_update_refs__a_branch_of_another_worktree_stays_and_the_note_says_where() {
    let (version, stacked) = (version(53), stack(None, true));
    let decided = update_refs(&input(&version, None, &stacked));
    assert_eq!(decided.moving, ["p1"]);
    assert_eq!(decided.staying, ["p2"]);
    assert_eq!(
        decided.note.as_deref(),
        Some(
            "These branches move with it: `p1`. `p2` is checked out in C:/work/p2 and stays on the old commits: \
             rebase it there."
        )
    );
}
