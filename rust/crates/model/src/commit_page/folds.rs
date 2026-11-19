//! Which diffs of a page start folded, and the folds the user changes afterwards.

use std::collections::{BTreeMap, BTreeSet};

use super::FilePatch;

/// The most lines a diff may have, hunk headers included, to start unfolded.
pub const FILE_FOLD_LIMIT: usize = 800;

/// The most lines the diffs that start unfolded may have together, hunk headers included.
pub const PAGE_FOLD_LIMIT: usize = 5_000;

/// Whether a diff is folded.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Fold {
    /// Only its header shows.
    Folded,
    /// Its lines show.
    Unfolded,
}

/// Which diffs of a page are folded, by their place in the page: as the rules of [`initial_folds`] say, except for the
/// diffs the user folded or unfolded, which stay as the user left them.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct FoldSet {
    folded_by_rules: BTreeSet<usize>,
    chosen: BTreeMap<usize, Fold>,
}

impl FoldSet {
    /// Whether the diff at a place in the page is folded.
    #[must_use]
    pub fn is_folded(&self, index: usize) -> bool {
        match self.chosen.get(&index) {
            Some(fold) => *fold == Fold::Folded,
            None => self.folded_by_rules.contains(&index),
        }
    }

    /// Folds or unfolds a diff as the user asked; the rules no longer decide for it.
    pub fn choose(&mut self, index: usize, fold: Fold) {
        let _ = self.chosen.insert(index, fold);
    }

    /// Folds or unfolds every diff of a page of so many, as **Fold all** and **Unfold all** do.
    pub fn choose_all(&mut self, count: usize, fold: Fold) {
        for index in 0..count {
            self.choose(index, fold);
        }
    }
}

/// Which diffs start folded: in order, a diff is unfolded when it has at most [`FILE_FOLD_LIMIT`] lines and the diffs
/// unfolded before it, with it, have at most [`PAGE_FOLD_LIMIT`]; otherwise it is folded, and its lines do not count. A
/// file with no lines, binary or past the cap of lines, is unfolded, showing its note.
///
/// Feature: commit-page.initial-folds
#[must_use]
pub fn initial_folds(input: &[FilePatch]) -> FoldSet {
    let mut unfolded_lines = 0;
    let mut folded_by_rules = BTreeSet::new();
    for (index, file) in input.iter().enumerate() {
        let size = file.size();
        if size <= FILE_FOLD_LIMIT && unfolded_lines + size <= PAGE_FOLD_LIMIT {
            unfolded_lines += size;
        } else {
            let _ = folded_by_rules.insert(index);
        }
    }
    FoldSet {
        folded_by_rules,
        chosen: BTreeMap::new(),
    }
}

#[cfg(test)]
mod tests {
    #![allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]

    use super::{Fold, initial_folds};
    use crate::commit_page::{DiffLine, FilePatch, Hunk, LineKind};
    use crate::state::ChangeKind;

    /// A file whose diff has one hunk, and so many lines with its header.
    fn file_of(size: usize) -> FilePatch {
        let line = DiffLine {
            kind: LineKind::Added,
            text: String::new(),
            before: None,
            after: Some(1),
        };
        FilePatch {
            path: format!("{size}.txt"),
            original_path: None,
            kind: ChangeKind::Modified,
            hunks: vec![Hunk {
                header: String::from("@@ -0,0 +1 @@"),
                lines: vec![line; size - 1],
            }],
            added: size - 1,
            removed: 0,
            is_binary: false,
            is_past_limit: false,
            stored_with_lfs: None,
        }
    }

    /// Which of the files start folded.
    fn folded(sizes: &[usize]) -> Vec<bool> {
        let files: Vec<FilePatch> = sizes.iter().map(|size| file_of(*size)).collect();
        let folds = initial_folds(&files);
        (0..files.len()).map(|index| folds.is_folded(index)).collect()
    }

    #[test]
    fn commit_page_initial_folds__a_diff_of_801_lines_starts_folded_one_of_800_unfolded() {
        assert_eq!(folded(&[801]), [true]);
        assert_eq!(folded(&[800]), [false]);
        assert_eq!(folded(&[900, 100, 200]), [true, false, false]);
    }

    #[test]
    fn commit_page_initial_folds__past_5000_lines_a_diff_starts_folded_and_a_smaller_one_after_may_not() {
        let mut sizes = vec![450; 12];
        assert_eq!(folded(&sizes), [vec![false; 11], vec![true]].concat());
        sizes.push(50);
        assert_eq!(folded(&sizes).last(), Some(&false));
    }

    #[test]
    fn commit_page_initial_folds__folding_a_short_diff_by_hand_keeps_it_folded() {
        let files = vec![file_of(10), file_of(900)];
        let mut folds = initial_folds(&files);
        assert!(!folds.is_folded(0));
        folds.choose(0, Fold::Folded);
        folds.choose(1, Fold::Unfolded);
        assert!(folds.is_folded(0));
        assert!(!folds.is_folded(1));
        folds.choose_all(files.len(), Fold::Unfolded);
        assert!(!folds.is_folded(0));
    }

    #[test]
    fn commit_page_initial_folds__a_file_with_no_lines_is_unfolded() {
        let mut binary = file_of(2);
        binary.hunks.clear();
        binary.is_binary = true;
        let mut files = vec![file_of(800); 7];
        files.push(binary);
        let folds = initial_folds(&files);
        assert!(folds.is_folded(6));
        assert!(!folds.is_folded(7));
    }
}
