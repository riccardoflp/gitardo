//! A minimal diff of two lists of lines, as runs of changes between unchanged parts.
//!
//! Myers, "An O(ND) Difference Algorithm and Its Variations", Algorithmica 1, 1986: the greedy forward search for the
//! furthest point reached on each diagonal, kept for each number of edits so that the path can be walked back.

use std::ops::Range;

/// A run of changes between two unchanged parts: the lines it takes out of the old list and puts in the new one,
/// counted from 0. One of the two may be empty.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ChangedRun {
    /// The lines of the old list taken out.
    pub(crate) old: Range<usize>,
    /// The lines of the new list put in.
    pub(crate) new: Range<usize>,
}

/// One step of the path from the start of both lists to their end.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Edit {
    /// A line of both lists.
    Same,
    /// A line of the old list only.
    TakenOut,
    /// A line of the new list only.
    PutIn,
}

/// The runs of changes that turn the old lines into the new ones with as few lines taken out and put in as can be,
/// in order.
pub(crate) fn changed_runs(old: &[&str], new: &[&str]) -> Vec<ChangedRun> {
    let mut runs = Vec::new();
    let (mut old_line, mut new_line) = (0, 0);
    let mut open: Option<(usize, usize)> = None;
    for edit in edits(old, new) {
        match edit {
            Edit::Same => {
                if let Some((old_start, new_start)) = open.take() {
                    runs.push(ChangedRun {
                        old: old_start..old_line,
                        new: new_start..new_line,
                    });
                }
                old_line += 1;
                new_line += 1;
            }
            Edit::TakenOut => {
                let _ = open.get_or_insert((old_line, new_line));
                old_line += 1;
            }
            Edit::PutIn => {
                let _ = open.get_or_insert((old_line, new_line));
                new_line += 1;
            }
        }
    }
    if let Some((old_start, new_start)) = open {
        runs.push(ChangedRun {
            old: old_start..old_line,
            new: new_start..new_line,
        });
    }
    runs
}

/// The path from the start to the end, the lines both lists start and end with left out of the search.
fn edits(old: &[&str], new: &[&str]) -> Vec<Edit> {
    let prefix = old.iter().zip(new).take_while(|(left, right)| left == right).count();
    let (old_rest, new_rest) = (
        old.get(prefix..).unwrap_or_default(),
        new.get(prefix..).unwrap_or_default(),
    );
    let suffix = old_rest
        .iter()
        .rev()
        .zip(new_rest.iter().rev())
        .take_while(|(left, right)| left == right)
        .count();
    let old_middle = old_rest.get(..old_rest.len() - suffix).unwrap_or_default();
    let new_middle = new_rest.get(..new_rest.len() - suffix).unwrap_or_default();
    let mut path = vec![Edit::Same; prefix];
    path.extend(middle_edits(old_middle, new_middle));
    path.extend(std::iter::repeat_n(Edit::Same, suffix));
    path
}

/// The furthest point of the old list reached on each diagonal `k` (old line minus new line) from `-reach` to
/// `reach`, after some number of edits.
#[derive(Debug, Clone)]
struct Frontier {
    reach: usize,
    furthest: Vec<usize>,
}

impl Frontier {
    fn new(reach: usize) -> Frontier {
        Frontier {
            reach,
            furthest: vec![0; 2 * reach + 1],
        }
    }

    /// The furthest old line reached on a diagonal; 0 for one not reached.
    fn get(&self, diagonal: isize) -> usize {
        self.index(diagonal)
            .and_then(|index| self.furthest.get(index))
            .copied()
            .unwrap_or(0)
    }

    fn set(&mut self, diagonal: isize, old_line: usize) {
        if let Some(slot) = self.index(diagonal).and_then(|index| self.furthest.get_mut(index)) {
            *slot = old_line;
        }
    }

    fn index(&self, diagonal: isize) -> Option<usize> {
        usize::try_from(diagonal.checked_add_unsigned(self.reach)?).ok()
    }
}

/// Whether the step to a diagonal after some edits comes down from the diagonal above (a line put in) rather than
/// across from the one below (a line taken out).
fn comes_from_above(before: &Frontier, diagonal: isize, edits: isize) -> bool {
    diagonal == -edits || (diagonal != edits && before.get(diagonal - 1) < before.get(diagonal + 1))
}

/// The frontiers after 0, 1, 2… edits, up to the first that reaches the end of both lists.
fn frontiers(old: &[&str], new: &[&str]) -> Vec<Frontier> {
    let (old_count, new_count) = (old.len(), new.len());
    let mut trace: Vec<Frontier> = Vec::new();
    for edits in 0..=(old_count + new_count) {
        let before = trace.last().cloned().unwrap_or_else(|| Frontier::new(0));
        let mut frontier = Frontier::new(edits);
        let signed = isize::try_from(edits).unwrap_or(isize::MAX);
        let mut diagonal = -signed;
        while diagonal <= signed {
            let mut old_line = if edits == 0 {
                0
            } else if comes_from_above(&before, diagonal, signed) {
                before.get(diagonal + 1)
            } else {
                before.get(diagonal - 1) + 1
            };
            let mut new_line = old_line.saturating_add_signed(-diagonal);
            while old_line < old_count && new_line < new_count && old.get(old_line) == new.get(new_line) {
                old_line += 1;
                new_line += 1;
            }
            frontier.set(diagonal, old_line);
            if old_line >= old_count && new_line >= new_count {
                trace.push(frontier);
                return trace;
            }
            diagonal += 2;
        }
        trace.push(frontier);
    }
    trace
}

/// The path through two lists that share no first and no last line, walked back from their end.
fn middle_edits(old: &[&str], new: &[&str]) -> Vec<Edit> {
    let trace = frontiers(old, new);
    let mut path = Vec::new();
    let (mut old_line, mut new_line) = (old.len(), new.len());
    for edits in (1..trace.len()).rev() {
        let Some(before) = trace.get(edits - 1) else { break };
        let signed = isize::try_from(edits).unwrap_or(isize::MAX);
        let diagonal = isize::try_from(old_line).unwrap_or(isize::MAX) - isize::try_from(new_line).unwrap_or(0);
        let from_above = comes_from_above(before, diagonal, signed);
        let previous_diagonal = if from_above { diagonal + 1 } else { diagonal - 1 };
        let previous_old = before.get(previous_diagonal);
        let previous_new = previous_old.saturating_add_signed(-previous_diagonal);
        // The one edit of this step, then the lines both lists have, down the diagonal to where the step ended.
        let (edit_old, edit) = if from_above {
            (previous_old, Edit::PutIn)
        } else {
            (previous_old + 1, Edit::TakenOut)
        };
        path.extend(std::iter::repeat_n(Edit::Same, old_line.saturating_sub(edit_old)));
        path.push(edit);
        (old_line, new_line) = (previous_old, previous_new);
    }
    path.extend(std::iter::repeat_n(Edit::Same, old_line));
    path.reverse();
    path
}

#[cfg(test)]
mod tests {
    use super::{ChangedRun, Edit, changed_runs, edits};

    fn lines(text: &str) -> Vec<&str> {
        text.split(' ').filter(|line| !line.is_empty()).collect()
    }

    #[test]
    fn finds_the_runs_of_an_edit_and_an_addition() {
        let runs = changed_runs(&lines("a b c d e"), &lines("a B c x d e"));
        assert_eq!(
            runs,
            vec![ChangedRun { old: 1..2, new: 1..2 }, ChangedRun { old: 3..3, new: 3..4 }]
        );
    }

    #[test]
    fn takes_out_and_puts_in_as_few_lines_as_can_be() {
        let cases = [
            ("a b c a b b a", "c b a b a c", 5),
            ("", "a b", 2),
            ("a b", "", 2),
            ("a b c", "a b c", 0),
            ("x a b c", "a b c y", 2),
            ("a b c d", "d c b a", 6),
        ];
        for (old, new, changes) in cases {
            let (old, new) = (lines(old), lines(new));
            let path = edits(&old, &new);
            let count = path.iter().filter(|edit| **edit != Edit::Same).count();
            assert_eq!(count, changes, "{old:?} {new:?}");
            let kept = |kind| {
                path.iter()
                    .filter(|edit| **edit == kind || **edit == Edit::Same)
                    .count()
            };
            assert_eq!((kept(Edit::TakenOut), kept(Edit::PutIn)), (old.len(), new.len()));
        }
    }

    /// The length of the longest common subsequence, by the table of every pair of prefixes.
    fn common_length(old: &[&str], new: &[&str]) -> usize {
        let mut table = vec![vec![0; new.len() + 1]; old.len() + 1];
        for (row, old_line) in old.iter().enumerate() {
            for (column, new_line) in new.iter().enumerate() {
                table[row + 1][column + 1] = if old_line == new_line {
                    table[row][column] + 1
                } else {
                    table[row][column + 1].max(table[row + 1][column])
                };
            }
        }
        table[old.len()][new.len()]
    }

    #[test]
    fn keeps_as_many_lines_as_the_longest_common_subsequence_for_every_short_list() {
        // Every list of up to five lines of two kinds, against every other.
        let all: Vec<Vec<&str>> = (0..=5)
            .flat_map(|length| {
                (0..1_usize << length).map(move |bits| {
                    (0..length)
                        .map(|place| if bits >> place & 1 == 1 { "a" } else { "b" })
                        .collect()
                })
            })
            .collect();
        for old in &all {
            for new in &all {
                let same = edits(old, new).iter().filter(|edit| **edit == Edit::Same).count();
                assert_eq!(same, common_length(old, new), "{old:?} {new:?}");
            }
        }
    }
}
