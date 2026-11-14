use super::{ChangesInput, FileStatus, StagedBox, list};
use crate::state::{ChangeKind, FileChange, WorkingTree};

fn change(path: &str, kind: ChangeKind) -> FileChange {
    FileChange {
        path: path.to_owned(),
        original_path: None,
        kind,
    }
}

fn listed(tree: &WorkingTree) -> super::ChangesList {
    list(&ChangesInput {
        working_tree: tree,
        unsaved: &[],
    })
}

#[test]
fn changes_list__a_file_staged_then_changed_again_is_one_row_half_ticked() {
    let tree = WorkingTree {
        staged: vec![change("a.txt", ChangeKind::Modified)],
        unstaged: vec![change("a.txt", ChangeKind::Modified)],
        ..WorkingTree::default()
    };
    let list = listed(&tree);
    assert_eq!(list.rows.len(), 1);
    assert_eq!(list.rows[0].staged, StagedBox::HalfTicked);
    assert_eq!(list.rows[0].status, FileStatus::Changed(ChangeKind::Modified));
}

#[test]
fn changes_list__a_file_staged_as_added_then_changed_stays_added_half_ticked() {
    let tree = WorkingTree {
        staged: vec![change("new.txt", ChangeKind::Added)],
        unstaged: vec![change("new.txt", ChangeKind::Modified)],
        ..WorkingTree::default()
    };
    let list = listed(&tree);
    assert_eq!(list.rows.len(), 1);
    assert_eq!(list.rows[0].status, FileStatus::Changed(ChangeKind::Added));
    assert_eq!(list.rows[0].staged, StagedBox::HalfTicked);
}

#[test]
fn changes_list__a_staged_rename_is_one_row_and_two_once_unstaged() {
    let renamed = WorkingTree {
        staged: vec![FileChange {
            path: String::from("b"),
            original_path: Some(String::from("a")),
            kind: ChangeKind::Renamed,
        }],
        ..WorkingTree::default()
    };
    let list = listed(&renamed);
    assert_eq!(list.rows.len(), 1);
    assert_eq!(list.rows[0].path, "b");
    assert_eq!(list.rows[0].original_path.as_deref(), Some("a"));
    assert_eq!(list.rows[0].staged, StagedBox::Ticked);
    let unstaged = WorkingTree {
        unstaged: vec![change("a", ChangeKind::Deleted)],
        untracked: vec![String::from("b")],
        ..WorkingTree::default()
    };
    let rows: Vec<(String, FileStatus, StagedBox)> = listed(&unstaged)
        .rows
        .into_iter()
        .map(|row| (row.path, row.status, row.staged))
        .collect();
    assert_eq!(
        rows,
        [
            (
                String::from("a"),
                FileStatus::Changed(ChangeKind::Deleted),
                StagedBox::Empty
            ),
            (String::from("b"), FileStatus::Untracked, StagedBox::Empty),
        ]
    );
}

#[test]
fn changes_list__the_header_box_is_ticked_with_all_staged_and_half_ticked_with_one_of_three() {
    let all = WorkingTree {
        staged: vec![change("a", ChangeKind::Modified), change("b", ChangeKind::Added)],
        ..WorkingTree::default()
    };
    assert_eq!(listed(&all).header, StagedBox::Ticked);
    let one_of_three = WorkingTree {
        staged: vec![change("a", ChangeKind::Modified)],
        unstaged: vec![change("b", ChangeKind::Modified)],
        untracked: vec![String::from("c")],
        ..WorkingTree::default()
    };
    assert_eq!(listed(&one_of_three).header, StagedBox::HalfTicked);
    let none = WorkingTree {
        unstaged: vec![change("b", ChangeKind::Modified)],
        ..WorkingTree::default()
    };
    assert_eq!(listed(&none).header, StagedBox::Empty);
}

#[test]
fn changes_list__with_only_a_file_not_saved_the_header_box_is_disabled() {
    let tree = WorkingTree::default();
    let unsaved = [String::from("src/a.rs")];
    let list = list(&ChangesInput {
        working_tree: &tree,
        unsaved: &unsaved,
    });
    assert_eq!(list.header, StagedBox::Disabled);
    assert_eq!(list.rows.len(), 1);
    assert_eq!(list.rows[0].status, FileStatus::Unsaved);
    assert_eq!(list.rows[0].staged, StagedBox::Disabled);
}

#[test]
fn keeps_conflicts_apart_and_orders_rows_by_bytes() {
    let tree = WorkingTree {
        unstaged: vec![
            change("b.txt", ChangeKind::Modified),
            change("B.txt", ChangeKind::Modified),
        ],
        untracked: vec![String::from("a.txt")],
        conflicted: vec![crate::state::ConflictedFile {
            path: String::from("c.txt"),
            kind: crate::state::ConflictKind::BothModified,
        }],
        ..WorkingTree::default()
    };
    let list = listed(&tree);
    let paths: Vec<&str> = list.rows.iter().map(|row| row.path.as_str()).collect();
    assert_eq!(paths, ["B.txt", "a.txt", "b.txt"]);
    assert_eq!(list.conflicts.len(), 1);
}

#[test]
fn a_staged_deletion_still_on_disk_is_modified() {
    let tree = WorkingTree {
        staged: vec![change("a.txt", ChangeKind::Deleted)],
        untracked: vec![String::from("a.txt")],
        ..WorkingTree::default()
    };
    let list = listed(&tree);
    assert_eq!(list.rows.len(), 1);
    assert_eq!(list.rows[0].status, FileStatus::Changed(ChangeKind::Modified));
    let gone = WorkingTree {
        staged: vec![change("a.txt", ChangeKind::Modified)],
        unstaged: vec![change("a.txt", ChangeKind::Deleted)],
        ..WorkingTree::default()
    };
    assert_eq!(listed(&gone).rows[0].status, FileStatus::Changed(ChangeKind::Deleted));
}
