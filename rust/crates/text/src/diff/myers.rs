//! A minimal diff of two lists of numbers, each number standing for a line or a word.
//!
//! Myers, "An O(ND) Difference Algorithm and Its Variations", Algorithmica 1, 1986: the greedy search for the furthest
//! point reached on each diagonal, run from both ends at once to find the middle snake of a shortest path, and the
//! linear-space refinement of section 4b, which splits the lists there and goes on with each half. Before the search,
//! the elements found on one side only are taken out, since no shortest path keeps them, and the elements both lists
//! start and end with are kept without searching.

use std::ops::Range;

use super::TooComplex;

/// Which elements of each list a shortest path takes out and puts in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Marks {
    /// The elements of the old list taken out.
    pub(crate) removed: Vec<bool>,
    /// The elements of the new list put in.
    pub(crate) added: Vec<bool>,
}

/// The two lists being compared, and what is decided about them so far.
struct Search<'a> {
    /// The old list.
    old: &'a [u32],
    /// The new list.
    new: &'a [u32],
    /// What is decided.
    marks: Marks,
    /// Asked between steps of the search: false stops it.
    keep_going: &'a dyn Fn() -> bool,
}

/// The elements of each list a shortest path takes out and puts in, the elements standing for their lines or words by
/// a number each, the same number for the same text.
pub(crate) fn marks(old: &[u32], new: &[u32], keep_going: &dyn Fn() -> bool) -> Result<Marks, TooComplex> {
    let in_old = counts(old);
    let in_new = counts(new);
    let mut marks = Marks {
        removed: old.iter().map(|value| !in_new.contains(*value)).collect(),
        added: new.iter().map(|value| !in_old.contains(*value)).collect(),
    };
    let old_kept: Vec<usize> = (0..old.len()).filter(|index| !flag(&marks.removed, *index)).collect();
    let new_kept: Vec<usize> = (0..new.len()).filter(|index| !flag(&marks.added, *index)).collect();
    let old_values: Vec<u32> = old_kept.iter().filter_map(|index| old.get(*index).copied()).collect();
    let new_values: Vec<u32> = new_kept.iter().filter_map(|index| new.get(*index).copied()).collect();
    let mut search = Search {
        old: &old_values,
        new: &new_values,
        marks: Marks {
            removed: vec![false; old_values.len()],
            added: vec![false; new_values.len()],
        },
        keep_going,
    };
    search.compare(0..old_values.len(), 0..new_values.len())?;
    for (index, removed) in old_kept.iter().zip(&search.marks.removed) {
        if let Some(mark) = marks.removed.get_mut(*index) {
            *mark = *removed;
        }
    }
    for (index, added) in new_kept.iter().zip(&search.marks.added) {
        if let Some(mark) = marks.added.get_mut(*index) {
            *mark = *added;
        }
    }
    Ok(marks)
}

/// The values of a list, sorted with no repeats, to ask whether a value is in it.
struct Present(Vec<u32>);

impl Present {
    /// Whether a value is in the list.
    fn contains(&self, value: u32) -> bool {
        self.0.binary_search(&value).is_ok()
    }
}

/// The values of a list.
fn counts(values: &[u32]) -> Present {
    let mut sorted = values.to_vec();
    sorted.sort_unstable();
    sorted.dedup();
    Present(sorted)
}

/// A flag of a list, false past its end.
fn flag(flags: &[bool], index: usize) -> bool {
    flags.get(index).copied().unwrap_or(false)
}

/// The middle snake of a shortest path: a run of equal elements, from a point of the grid to another.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Snake {
    /// Where it starts, in the old list and in the new one.
    start: (usize, usize),
    /// Where it ends.
    end: (usize, usize),
}

/// A diagonal no path reaches yet, in the forward search.
const FORWARD_UNREACHED: isize = isize::MIN;
/// A diagonal no path reaches yet, in the backward search.
const BACKWARD_UNREACHED: isize = isize::MAX;

impl Search<'_> {
    /// Marks the elements of two parts of the lists that a shortest path between them takes out and puts in.
    fn compare(&mut self, mut old: Range<usize>, mut new: Range<usize>) -> Result<(), TooComplex> {
        while old.start < old.end && new.start < new.end && self.old.get(old.start) == self.new.get(new.start) {
            old.start += 1;
            new.start += 1;
        }
        while old.start < old.end && new.start < new.end && self.old.get(old.end - 1) == self.new.get(new.end - 1) {
            old.end -= 1;
            new.end -= 1;
        }
        if old.is_empty() || new.is_empty() {
            self.mark(old, new);
            return Ok(());
        }
        let snake = self.middle_snake(&old, &new)?;
        let before = (old.start..snake.start.0, new.start..snake.start.1);
        let after = (snake.end.0..old.end, snake.end.1..new.end);
        if (before.0 == old && before.1 == new) || (after.0 == old && after.1 == new) {
            // A shortest path always moves at least once on each side of its middle snake; this keeps the search from
            // going round should that not hold.
            self.mark(old, new);
            return Ok(());
        }
        self.compare(before.0, before.1)?;
        self.compare(after.0, after.1)
    }

    /// Marks every element of two parts as taken out and put in.
    fn mark(&mut self, old: Range<usize>, new: Range<usize>) {
        for index in old {
            if let Some(mark) = self.marks.removed.get_mut(index) {
                *mark = true;
            }
        }
        for index in new {
            if let Some(mark) = self.marks.added.get_mut(index) {
                *mark = true;
            }
        }
    }

    /// The middle snake of a shortest path between two parts, neither empty and not starting or ending with the same
    /// element: the forward and the backward search go one more step each in turn until they overlap.
    fn middle_snake(&self, old: &Range<usize>, new: &Range<usize>) -> Result<Snake, TooComplex> {
        let grid = Grid::new(old, new);
        let size = grid.diagonals();
        let mut forward = vec![FORWARD_UNREACHED; size];
        let mut backward = vec![BACKWARD_UNREACHED; size];
        let delta = grid.width - grid.height;
        let is_odd = delta.rem_euclid(2) == 1;
        for steps in 0..=grid.width + grid.height {
            if !(self.keep_going)() {
                return Err(TooComplex);
            }
            if let Some(snake) = self.forward_step(&grid, (&mut forward, &backward), (steps, is_odd)) {
                return Ok(snake);
            }
            if let Some(snake) = self.backward_step(&grid, (&forward, &mut backward), (steps, is_odd)) {
                return Ok(snake);
            }
        }
        Ok(Snake {
            start: (old.start, new.start),
            end: (old.start, new.start),
        })
    }

    /// One more step of the forward search: on each diagonal it can reach, the furthest point a path of so many moves
    /// reaches, followed down its run of equal elements; the snake when it meets the backward search.
    fn forward_step(
        &self,
        grid: &Grid,
        (forward, backward): (&mut [isize], &[isize]),
        (steps, is_odd): (isize, bool),
    ) -> Option<Snake> {
        let delta = grid.width - grid.height;
        let mut diagonal = first_at_least(-steps, -grid.height);
        while diagonal <= steps.min(grid.width) {
            let start = if steps == 0 {
                Some(0)
            } else {
                let down = value(forward, grid.slot(diagonal + 1)).filter(|x| *x != FORWARD_UNREACHED);
                let right = value(forward, grid.slot(diagonal - 1))
                    .filter(|x| *x != FORWARD_UNREACHED)
                    .map(|x| x + 1);
                [down, right]
                    .into_iter()
                    .flatten()
                    .filter(|x| grid.holds(*x, *x - diagonal))
                    .max()
            };
            if let Some(slot) = grid.slot(diagonal) {
                let end = start.map(|x| (x, self.slide_forward(grid, x, diagonal)));
                if let Some(cell) = forward.get_mut(slot) {
                    *cell = end.map_or(FORWARD_UNREACHED, |(_, end)| end);
                }
                let reached = value(backward, Some(slot)).filter(|x| *x != BACKWARD_UNREACHED);
                if let (Some((x, end)), Some(back)) = (end, reached)
                    && is_odd
                    && (diagonal - delta).abs() < steps
                    && end >= back
                {
                    return Some(grid.snake((x, x - diagonal), (end, end - diagonal)));
                }
            }
            diagonal += 2;
        }
        None
    }

    /// One more step of the backward search, from the end of both parts towards their start: on each diagonal it can
    /// reach, the point nearest the start, followed up its run of equal elements; the snake when it meets the forward
    /// search.
    fn backward_step(
        &self,
        grid: &Grid,
        (forward, backward): (&[isize], &mut [isize]),
        (steps, is_odd): (isize, bool),
    ) -> Option<Snake> {
        let delta = grid.width - grid.height;
        let mut diagonal = first_at_least(delta - steps, -grid.height);
        while diagonal <= (delta + steps).min(grid.width) {
            let start = if steps == 0 {
                Some(grid.width)
            } else {
                let up = value(backward, grid.slot(diagonal - 1)).filter(|x| *x != BACKWARD_UNREACHED);
                let left = value(backward, grid.slot(diagonal + 1))
                    .filter(|x| *x != BACKWARD_UNREACHED)
                    .map(|x| x - 1);
                [up, left]
                    .into_iter()
                    .flatten()
                    .filter(|x| grid.holds(*x, *x - diagonal))
                    .min()
            };
            if let Some(slot) = grid.slot(diagonal) {
                let end = start.map(|x| (x, self.slide_backward(grid, x, diagonal)));
                if let Some(cell) = backward.get_mut(slot) {
                    *cell = end.map_or(BACKWARD_UNREACHED, |(_, end)| end);
                }
                let reached = value(forward, Some(slot)).filter(|x| *x != FORWARD_UNREACHED);
                if let (Some((x, end)), Some(ahead)) = (end, reached)
                    && !is_odd
                    && diagonal.abs() <= steps
                    && end <= ahead
                {
                    return Some(grid.snake((end, end - diagonal), (x, x - diagonal)));
                }
            }
            diagonal += 2;
        }
        None
    }

    /// How far a path on a diagonal goes forward over equal elements from a point.
    fn slide_forward(&self, grid: &Grid, mut x: isize, diagonal: isize) -> isize {
        while x < grid.width && x - diagonal < grid.height && self.same(grid, x, x - diagonal) {
            x += 1;
        }
        x
    }

    /// How far a path on a diagonal goes backward over equal elements from a point.
    fn slide_backward(&self, grid: &Grid, mut x: isize, diagonal: isize) -> isize {
        while x > 0 && x - diagonal > 0 && self.same(grid, x - 1, x - 1 - diagonal) {
            x -= 1;
        }
        x
    }

    /// Whether the elements at a point of the grid are equal.
    fn same(&self, grid: &Grid, x: isize, y: isize) -> bool {
        let (Ok(x), Ok(y)) = (usize::try_from(x), usize::try_from(y)) else {
            return false;
        };
        let old = self.old.get(grid.old_start + x);
        old.is_some() && old == self.new.get(grid.new_start + y)
    }
}

/// The first diagonal of a step, from `start` in steps of two, that is not below `least`.
fn first_at_least(start: isize, least: isize) -> isize {
    if start >= least {
        start
    } else {
        start + (least - start + 1) / 2 * 2
    }
}

/// The value of a slot of a search, if the slot is there.
fn value(values: &[isize], slot: Option<usize>) -> Option<isize> {
    slot.and_then(|slot| values.get(slot)).copied()
}

/// The grid of the edit graph of two parts: x along the old part, y along the new one, a diagonal being x − y.
struct Grid {
    /// Where the old part starts in its list.
    old_start: usize,
    /// Where the new part starts in its list.
    new_start: usize,
    /// The length of the old part.
    width: isize,
    /// The length of the new part.
    height: isize,
}

impl Grid {
    /// The grid of two parts.
    fn new(old: &Range<usize>, new: &Range<usize>) -> Grid {
        Grid {
            old_start: old.start,
            new_start: new.start,
            width: isize::try_from(old.len()).unwrap_or(isize::MAX / 4),
            height: isize::try_from(new.len()).unwrap_or(isize::MAX / 4),
        }
    }

    /// How many diagonals cross the grid.
    fn diagonals(&self) -> usize {
        usize::try_from(self.width + self.height + 1).unwrap_or(0)
    }

    /// The slot of a diagonal in the lists of a search; none for a diagonal that does not cross the grid.
    fn slot(&self, diagonal: isize) -> Option<usize> {
        if diagonal < -self.height || diagonal > self.width {
            return None;
        }
        usize::try_from(diagonal + self.height).ok()
    }

    /// Whether a point is on the grid.
    fn holds(&self, x: isize, y: isize) -> bool {
        (0..=self.width).contains(&x) && (0..=self.height).contains(&y)
    }

    /// A snake between two points of the grid, in the places of the lists.
    fn snake(&self, start: (isize, isize), end: (isize, isize)) -> Snake {
        let place = |offset: usize, at: isize| offset + usize::try_from(at).unwrap_or(0);
        Snake {
            start: (place(self.old_start, start.0), place(self.new_start, start.1)),
            end: (place(self.old_start, end.0), place(self.new_start, end.1)),
        }
    }
}
