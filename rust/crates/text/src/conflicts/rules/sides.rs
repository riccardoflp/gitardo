//! The file as each side would have it, paired with the base, and the stretch around a conflict from where the two
//! sides are at the same place before it to where they are again after it.

use std::iter::once;
use std::ops::Range;

use super::combine::{Parts, Version};
use crate::conflicts::markers::{Line, MarkedConflict};
use crate::diff::{LineDiffInput, line_diff};

/// Which way lines that repeat are paired with those of the base.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Direction {
    /// From the top, as a diff usually pairs them.
    FromTheTop,
    /// From the bottom: the same diff on both lists of lines reversed, its result turned back.
    FromTheBottom,
}

/// How lines are paired: the direction, and what stops a diff that takes too long.
#[derive(Clone, Copy)]
pub(super) struct Pairing<'s> {
    /// Which way lines that repeat are paired.
    pub(super) direction: Direction,
    /// Asked between steps of a diff: false stops it.
    pub(super) keep_going: &'s dyn Fn() -> bool,
}

/// Lines of the base that a side replaced by other lines, possibly none, or in place of none.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct SideChange {
    /// The lines of the base replaced.
    pub(super) base: Range<usize>,
    /// The lines of the side put in their place.
    pub(super) side: Range<usize>,
}

impl Pairing<'_> {
    /// For each line of `new`, the line of `old` it is paired with by a minimal line diff; none when the diff was
    /// stopped.
    pub(super) fn paired(&self, old: &[&str], new: &[&str]) -> Option<Vec<Option<usize>>> {
        match self.direction {
            Direction::FromTheTop => self.paired_from_the_top(old, new),
            Direction::FromTheBottom => {
                let old_reversed: Vec<&str> = old.iter().rev().copied().collect();
                let new_reversed: Vec<&str> = new.iter().rev().copied().collect();
                let reversed = self.paired_from_the_top(&old_reversed, &new_reversed)?;
                let last = old.len().saturating_sub(1);
                Some(
                    reversed
                        .into_iter()
                        .rev()
                        .map(|paired| paired.map(|line| last - line))
                        .collect(),
                )
            }
        }
    }

    /// The pairing of a minimal line diff run from the top.
    fn paired_from_the_top(&self, old: &[&str], new: &[&str]) -> Option<Vec<Option<usize>>> {
        let changes = line_diff(&LineDiffInput {
            left: old,
            right: new,
            keep_going: self.keep_going,
        })
        .ok()?;
        let mut paired = vec![None; new.len()];
        let (mut old_line, mut new_line) = (0, 0);
        let ends = changes
            .iter()
            .map(|change| (change.left.lines(), change.right.lines()))
            .chain(once((old.len()..old.len(), new.len()..new.len())));
        for (removed, added) in ends {
            while old_line < removed.start && new_line < added.start {
                if let Some(slot) = paired.get_mut(new_line) {
                    *slot = Some(old_line);
                }
                old_line += 1;
                new_line += 1;
            }
            old_line = removed.end;
            new_line = added.end;
        }
        Some(paired)
    }

    /// A side's lines with its changes to some lines of the base.
    pub(super) fn version<'v, 'a>(&self, base: &[Line<'a>], side: &'v [Line<'a>]) -> Option<Version<'v, 'a>> {
        let paired = self.paired(&texts(base), &texts(side))?;
        Some(Version {
            lines: side,
            changes: side_changes(&paired, base.len()),
        })
    }
}

/// The texts of some lines, without what ends them.
pub(super) fn texts<'a>(lines: &[Line<'a>]) -> Vec<&'a str> {
    lines.iter().map(|line| line.text).collect()
}

/// The changes of a side to the base, in order, from the pairing of each of its lines with a line of the base.
pub(super) fn side_changes(paired: &[Option<usize>], base_length: usize) -> Vec<SideChange> {
    let mut changes = Vec::new();
    let (mut base_next, mut side_next) = (0, 0);
    let kept = paired
        .iter()
        .enumerate()
        .filter_map(|(side, base)| base.map(|base| (base, side)));
    for (base, side) in kept.chain(once((base_length, paired.len()))) {
        if base > base_next || side > side_next {
            changes.push(SideChange {
                base: base_next..base,
                side: side_next..side,
            });
        }
        base_next = base + 1;
        side_next = side + 1;
    }
    changes
}

/// The file as each side would have it: the lines outside every conflict, which both share, and its own part of each
/// conflict.
pub(super) struct Rebuilt<'a> {
    /// The current side's file.
    pub(super) current: Vec<Line<'a>>,
    /// The incoming side's file.
    pub(super) incoming: Vec<Line<'a>>,
    /// For each line of the file outside the conflicts, its place in the current side and in the incoming side.
    pub(super) places: Vec<Option<(usize, usize)>>,
}

/// The file as each side would have it.
pub(super) fn rebuild<'a>(file: &[Line<'a>], conflicts: &[MarkedConflict]) -> Rebuilt<'a> {
    let mut rebuilt = Rebuilt {
        current: Vec::new(),
        incoming: Vec::new(),
        places: vec![None; file.len()],
    };
    let mut line = 0;
    for conflict in conflicts.iter().map(Some).chain(once(None)) {
        let end = conflict.map_or(file.len(), |conflict| conflict.span.start);
        for (place, shared) in rebuilt.places.iter_mut().zip(file).take(end).skip(line) {
            *place = Some((rebuilt.current.len(), rebuilt.incoming.len()));
            rebuilt.current.push(*shared);
            rebuilt.incoming.push(*shared);
        }
        if let Some(conflict) = conflict {
            rebuilt
                .current
                .extend_from_slice(file.get(conflict.current.clone()).unwrap_or_default());
            rebuilt
                .incoming
                .extend_from_slice(file.get(conflict.incoming.clone()).unwrap_or_default());
            line = conflict.span.end;
        }
    }
    rebuilt
}

/// What the rules read of a file, with each side paired with the base in one direction.
pub(super) struct Paired<'s, 'a> {
    /// What the rules read of the file.
    pub(super) read: &'s Read<'s, 'a>,
    /// How lines are paired.
    pub(super) pairing: Pairing<'s>,
    /// For each line of the current side, the line of the base it is paired with.
    current: Vec<Option<usize>>,
    /// For each line of the incoming side, the line of the base it is paired with.
    incoming: Vec<Option<usize>>,
    /// For each line of the current side's own file, the line of the base it is paired with.
    own_current: Vec<Option<usize>>,
    /// For each line of the incoming side's own file, the line of the base it is paired with.
    own_incoming: Vec<Option<usize>>,
}

/// What a file is, before its sides are paired with the base.
pub(super) struct Read<'s, 'a> {
    /// The lines of the file.
    pub(super) file: &'s [Line<'a>],
    /// The lines of the base.
    pub(super) base: &'s [Line<'a>],
    /// The texts of the current side's own file, `:2:`.
    pub(super) own_current: &'s [&'a str],
    /// The texts of the incoming side's own file, `:3:`.
    pub(super) own_incoming: &'s [&'a str],
    /// The conflicts of the file.
    pub(super) conflicts: &'s [MarkedConflict],
    /// The file as each side would have it.
    pub(super) rebuilt: &'s Rebuilt<'a>,
}

/// A line of the file outside the conflicts where both sides are at the same line of the base.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Anchor {
    /// Its place in the file.
    file: usize,
    /// The line of the base both sides pair it with.
    base: usize,
    /// Its place in the current side.
    current: usize,
    /// Its place in the incoming side.
    incoming: usize,
}

/// Where a stretch is: from the line after its upper anchor to the one before its lower anchor, in the file, in the
/// base and in each side; and whether it starts at the start of the file and ends at its end.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Places {
    /// Its lines in the file.
    pub(super) file: Range<usize>,
    /// Its lines in the base.
    pub(super) base: Range<usize>,
    /// Its lines in the current side.
    pub(super) current: Range<usize>,
    /// Its lines in the incoming side.
    pub(super) incoming: Range<usize>,
    /// The upper anchor, in the file; none at the start of the file.
    upper: Option<usize>,
    /// The lower anchor, in the file; none at the end of the file.
    lower: Option<usize>,
}

impl<'s, 'a> Paired<'s, 'a> {
    /// The file with each side paired with the base; none when a diff was stopped.
    pub(super) fn new(read: &'s Read<'s, 'a>, pairing: Pairing<'s>) -> Option<Paired<'s, 'a>> {
        let base = texts(read.base);
        let current = pairing.paired(&base, &texts(&read.rebuilt.current))?;
        let incoming = pairing.paired(&base, &texts(&read.rebuilt.incoming))?;
        Some(Paired {
            read,
            pairing,
            current,
            incoming,
            own_current: pairing.paired(&base, read.own_current)?,
            own_incoming: pairing.paired(&base, read.own_incoming)?,
        })
    }

    /// The line of the file as an anchor, when both sides pair it with the same line of the base.
    fn anchor(&self, line: usize) -> Option<Anchor> {
        let (current, incoming) = self.read.rebuilt.places.get(line).copied().flatten()?;
        let base = self.current.get(current).copied().flatten()?;
        (self.incoming.get(incoming).copied().flatten() == Some(base)).then_some(Anchor {
            file: line,
            base,
            current,
            incoming,
        })
    }

    /// Where the stretch of a conflict is: none when another conflict is reached before an anchor, or when the anchors
    /// are not in order in the base.
    pub(super) fn places(&self, index: usize) -> Option<Places> {
        let conflict = self.read.conflicts.get(index)?;
        let floor = index
            .checked_sub(1)
            .and_then(|before| self.read.conflicts.get(before))
            .map(|before| before.span.end);
        let ceiling = self.read.conflicts.get(index + 1).map(|after| after.span.start);
        let upper = (floor.unwrap_or(0)..conflict.span.start)
            .rev()
            .find_map(|line| self.anchor(line));
        let lower = (conflict.span.end..ceiling.unwrap_or(self.read.file.len())).find_map(|line| self.anchor(line));
        if (upper.is_none() && floor.is_some()) || (lower.is_none() && ceiling.is_some()) {
            return None;
        }
        let after = |anchor: Option<Anchor>, place: fn(&Anchor) -> usize| anchor.map_or(0, |anchor| place(&anchor) + 1);
        let places = Places {
            file: after(upper, |anchor| anchor.file)..lower.map_or(self.read.file.len(), |anchor| anchor.file),
            base: after(upper, |anchor| anchor.base)..lower.map_or(self.read.base.len(), |anchor| anchor.base),
            current: after(upper, |anchor| anchor.current)
                ..lower.map_or(self.read.rebuilt.current.len(), |anchor| anchor.current),
            incoming: after(upper, |anchor| anchor.incoming)
                ..lower.map_or(self.read.rebuilt.incoming.len(), |anchor| anchor.incoming),
            upper: upper.map(|anchor| anchor.file),
            lower: lower.map(|anchor| anchor.file),
        };
        (places.base.start <= places.base.end).then_some(places)
    }

    /// Whether the stretch of each side, with its anchors, is found as consecutive lines in that side's own file, its
    /// anchors paired there with the lines of the base they are anchors for.
    pub(super) fn sides_have_it(&self, places: &Places) -> bool {
        let wanted = |stretch: &[Line<'a>]| -> Vec<&'a str> {
            let anchor = |line: Option<usize>| line.and_then(|line| self.read.file.get(line)).map(|line| line.text);
            anchor(places.upper)
                .into_iter()
                .chain(stretch.iter().map(|line| line.text))
                .chain(anchor(places.lower))
                .collect()
        };
        let current = self
            .read
            .rebuilt
            .current
            .get(places.current.clone())
            .unwrap_or_default();
        let incoming = self
            .read
            .rebuilt
            .incoming
            .get(places.incoming.clone())
            .unwrap_or_default();
        let own_current = Own {
            lines: self.read.own_current,
            paired: &self.own_current,
        };
        let own_incoming = Own {
            lines: self.read.own_incoming,
            paired: &self.own_incoming,
        };
        own_current.has(&wanted(current), places) && own_incoming.has(&wanted(incoming), places)
    }

    /// Whether every line of a stretch outside the markers of its conflict is kept by both sides from the same line of
    /// the base, or written by both in a change of their own. A line one side kept and the other wrote is one git took
    /// from one side and wrote outside the markers: neither is ground to read what the other side did.
    pub(super) fn is_shared_outside(&self, places: &Places, parts: &Parts<'_, '_>, index: usize) -> bool {
        let Some(conflict) = self.read.conflicts.get(index) else {
            return false;
        };
        let above = conflict.span.start.saturating_sub(places.file.start);
        let below = places.file.end.saturating_sub(conflict.span.end);
        let outside = |version: &Version<'_, '_>| -> Vec<Option<usize>> {
            let kept = version.kept_lines();
            let after = kept.len().saturating_sub(below);
            kept.iter()
                .take(above)
                .chain(kept.iter().skip(after))
                .copied()
                .collect()
        };
        let (current, incoming) = (outside(&parts.current), outside(&parts.incoming));
        current.len() == incoming.len()
            && current
                .iter()
                .zip(&incoming)
                .all(|(current, incoming)| current == incoming)
    }

    /// The base and the two sides of a stretch, with the changes of each side to the base of the stretch, found by a
    /// minimal line diff of that stretch alone; none when a diff was stopped.
    pub(super) fn parts(&self, places: &Places) -> Option<Parts<'s, 'a>> {
        let base = self.read.base.get(places.base.clone())?;
        Some(Parts {
            base,
            current: self
                .pairing
                .version(base, self.read.rebuilt.current.get(places.current.clone())?)?,
            incoming: self
                .pairing
                .version(base, self.read.rebuilt.incoming.get(places.incoming.clone())?)?,
        })
    }
}

/// A side's own file, `:2:` or `:3:`, with each of its lines paired with a line of the base.
struct Own<'o, 'a> {
    /// Its lines.
    lines: &'o [&'a str],
    /// For each of its lines, the line of the base it is paired with.
    paired: &'o [Option<usize>],
}

impl Own<'_, '_> {
    /// Whether some lines, a stretch with its anchors, are consecutive lines of the file with the anchors paired with
    /// the lines of the base they stand for: at its very start when the stretch starts at the start of the file, at
    /// its very end when it ends at its end.
    fn has(&self, wanted: &[&str], places: &Places) -> bool {
        let Some(last_start) = self.lines.len().checked_sub(wanted.len()) else {
            return false;
        };
        let is_paired = |line: usize, base: usize| self.paired.get(line).copied().flatten() == Some(base);
        (0..=last_start).any(|start| {
            let end = start + wanted.len();
            self.lines.get(start..end) == Some(wanted)
                && match places.upper {
                    Some(_) => is_paired(start, places.base.start.wrapping_sub(1)),
                    None => start == 0,
                }
                && match places.lower {
                    Some(_) => is_paired(end.wrapping_sub(1), places.base.end),
                    None => end == self.lines.len(),
                }
        })
    }
}
