//! The commits a merge brought in: those loaded that its other parents reach and its first parent does not.

use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashMap};

use crate::commit::Commit;
use crate::graph::reach::Ancestry;

/// Reached from the first parent of the merge.
const FROM_FIRST: u8 = 1;
/// Reached from another parent of the merge.
const FROM_OTHER: u8 = 2;

/// The rows of the commits a merge brought in, from the top. The rows are walked down from the merge in their order,
/// each once all the rows above it that reach it were, and the walk stops as soon as no row is left that only the
/// other parents reach: where the merged branch meets the history of the first parent.
pub(super) fn brought_in(ancestry: &Ancestry<'_>, merge: &Commit, row: usize) -> Vec<usize> {
    let mut walk = Walk::default();
    let parents = merge.parents.iter().map(|parent| ancestry.row_of(parent));
    for (number, parent) in parents.enumerate() {
        let flag = if number == 0 { FROM_FIRST } else { FROM_OTHER };
        if let Some(parent) = parent.filter(|parent| *parent > row) {
            walk.reach(parent, flag);
        }
    }
    let mut brought = Vec::new();
    while walk.only_from_other > 0 {
        let Some(Reverse(next)) = walk.waiting.pop() else {
            break;
        };
        let flags = walk.flags.get(&next).copied().unwrap_or(0);
        if flags == FROM_OTHER {
            walk.only_from_other -= 1;
            brought.push(next);
        }
        for parent in ancestry.parents_of(next) {
            walk.reach(*parent, flags);
        }
    }
    brought
}

/// The rows reached so far, from which parents of the merge.
#[derive(Debug, Default)]
struct Walk {
    /// What reaches each row reached.
    flags: HashMap<usize, u8>,
    /// The rows reached and not walked yet, the top one first.
    waiting: BinaryHeap<Reverse<usize>>,
    /// How many of the rows not walked yet only the other parents reach.
    only_from_other: usize,
}

impl Walk {
    /// Marks a row as reached from some parents of the merge.
    fn reach(&mut self, row: usize, flag: u8) {
        let before = self.flags.get(&row).copied().unwrap_or(0);
        let after = before | flag;
        if after == before {
            return;
        }
        let _ = self.flags.insert(row, after);
        if before == 0 {
            self.waiting.push(Reverse(row));
        }
        if before == FROM_OTHER {
            self.only_from_other -= 1;
        }
        if after == FROM_OTHER {
            self.only_from_other += 1;
        }
    }
}
