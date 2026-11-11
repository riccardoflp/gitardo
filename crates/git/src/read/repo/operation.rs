//! The operation git stopped half way, from the files it leaves in the folder of the worktree.

use std::fs;
use std::path::Path;

use model::state::Operation;

/// The files that say an operation is in progress, in the order they are looked for: the first one there names it.
/// `git am` and `git rebase` both use `rebase-apply`, so the file only `git am` writes comes first.
const MARKERS: [(&str, Operation); 6] = [
    ("rebase-apply/applying", Operation::Patch),
    ("rebase-merge", Operation::Rebase),
    ("rebase-apply", Operation::Rebase),
    ("MERGE_HEAD", Operation::Merge),
    ("CHERRY_PICK_HEAD", Operation::CherryPick),
    ("REVERT_HEAD", Operation::Revert),
];

/// The list of the commits a cherry-pick or a revert of a series has still to do.
const SEQUENCER_TODO: &str = "sequencer/todo";

/// The operation in progress, given which of the files of [`MARKERS`] are there and what the list of a series holds,
/// if there is one. Between two commits of a series only the list is there: it is a revert when its first line is
/// one, and a cherry-pick otherwise.
pub(super) fn operation_from(present: &[&str], sequencer_todo: Option<&str>) -> Option<Operation> {
    MARKERS
        .iter()
        .find(|(marker, _)| present.contains(marker))
        .map(|(_, operation)| *operation)
        .or_else(|| {
            sequencer_todo.map(|todo| {
                if todo.starts_with("revert") {
                    Operation::Revert
                } else {
                    Operation::CherryPick
                }
            })
        })
}

/// The operation in progress in the worktree whose git folder this is.
pub(super) fn read_operation(git_folder: &Path) -> Option<Operation> {
    let present: Vec<&str> = MARKERS
        .iter()
        .map(|(marker, _)| *marker)
        .filter(|marker| git_folder.join(marker).exists())
        .collect();
    // A list that cannot be read is not there: git removes it at the end of a series, maybe while this reads.
    let sequencer_todo = fs::read(git_folder.join(SEQUENCER_TODO)).ok();
    let sequencer_todo = sequencer_todo.as_deref().map(String::from_utf8_lossy);
    operation_from(&present, sequencer_todo.as_deref())
}

#[cfg(test)]
mod tests {
    use super::operation_from;
    use model::state::Operation;

    #[test]
    fn names_the_operation_from_the_first_file_there() {
        let cases: [(&[&str], Option<&str>, Option<Operation>); 9] = [
            (&[], None, None),
            (&["rebase-apply/applying", "rebase-apply"], None, Some(Operation::Patch)),
            (&["rebase-apply"], None, Some(Operation::Rebase)),
            (&["rebase-merge", "MERGE_HEAD"], None, Some(Operation::Rebase)),
            (&["MERGE_HEAD"], None, Some(Operation::Merge)),
            (
                &["CHERRY_PICK_HEAD"],
                Some("pick 1234 x\n"),
                Some(Operation::CherryPick),
            ),
            (&["REVERT_HEAD"], Some("revert 1234 x\n"), Some(Operation::Revert)),
            (&[], Some("revert 1234 x\nrevert 5678 y\n"), Some(Operation::Revert)),
            (&[], Some("pick 1234 x\n"), Some(Operation::CherryPick)),
        ];
        for (present, todo, expected) in cases {
            assert_eq!(operation_from(present, todo), expected, "{present:?} {todo:?}");
        }
    }
}
