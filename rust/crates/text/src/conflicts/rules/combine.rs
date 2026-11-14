//! The changes of the two sides to a stretch of the base, put together: those of one side written, the same change
//! written once, and where the two sides met, the lines they have alike written once with what is between them put
//! together on its own, or failing that the rules of spaces.

use std::ops::Range;

use super::meet::meet;
use super::sides::{Pairing, SideChange, texts};
use crate::conflicts::markers::Line;

/// One side of a stretch: its lines, and its changes to the base, both counted from the start of the stretch.
pub(super) struct Version<'v, 'a> {
    /// Its lines.
    pub(super) lines: &'v [Line<'a>],
    /// Its changes to the base, in order.
    pub(super) changes: Vec<SideChange>,
}

impl Version<'_, '_> {
    /// For each of its lines, the line of the base it keeps; none for a line it wrote in a change.
    pub(super) fn kept_lines(&self) -> Vec<Option<usize>> {
        let mut kept = vec![None; self.lines.len()];
        let (mut base, mut side) = (0, 0);
        let ends = self
            .changes
            .iter()
            .map(|change| (change.base.clone(), change.side.clone()))
            .chain(std::iter::once((
                usize::MAX..usize::MAX,
                self.lines.len()..self.lines.len(),
            )));
        for (base_lines, side_lines) in ends {
            while side < side_lines.start {
                if let Some(slot) = kept.get_mut(side) {
                    *slot = Some(base);
                }
                base += 1;
                side += 1;
            }
            base = base_lines.end;
            side = side_lines.end;
        }
        kept
    }
}

/// The base and the two sides of a stretch.
pub(super) struct Parts<'v, 'a> {
    /// The lines of the base.
    pub(super) base: &'v [Line<'a>],
    /// The current side.
    pub(super) current: Version<'v, 'a>,
    /// The incoming side.
    pub(super) incoming: Version<'v, 'a>,
}

/// Where a line written comes from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Source {
    /// A line of the base both sides kept.
    Kept,
    /// A change of the current side alone.
    Current,
    /// A change of the incoming side alone.
    Incoming,
    /// Both sides wrote it.
    Both,
}

/// A line written, and where it comes from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Written<'a> {
    /// The line, as one side has it in the file.
    pub(super) line: Line<'a>,
    /// Where it comes from.
    pub(super) source: Source,
}

/// The lines put together, and how.
pub(super) struct Combined<'a> {
    /// The lines written.
    pub(super) lines: Vec<Written<'a>>,
    /// Whether the two sides changed the same lines differently somewhere.
    pub(super) met: bool,
    /// Whether a rule of spaces was used.
    pub(super) spaces: bool,
}

/// Which changes of the two sides go together.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Grouping {
    /// Changes that share a line of the base or are next to each other: the whole stretch.
    Touch,
    /// Changes that share a line of the base: what is between two runs of lines the sides have alike.
    Overlap,
}

/// Which side a change is of.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Side {
    /// The current side.
    Current,
    /// The incoming side.
    Incoming,
}

/// Changes of the two sides that go together, and the lines of the base they cover.
struct Group {
    /// The lines of the base, from the first line of its first change to the last of its last.
    base: Range<usize>,
    /// The changes of the current side, in order.
    current: Vec<SideChange>,
    /// The changes of the incoming side, in order.
    incoming: Vec<SideChange>,
}

/// The changes of the two sides to a stretch, put together; none when a group cannot be.
pub(super) fn combine<'a>(parts: &Parts<'_, 'a>, grouping: Grouping, pairing: Pairing<'_>) -> Option<Combined<'a>> {
    let mut combined = Combined {
        lines: Vec::new(),
        met: false,
        spaces: false,
    };
    let mut base_next = 0;
    for group in groups(&parts.current.changes, &parts.incoming.changes, grouping) {
        if group.base.start < base_next {
            return None;
        }
        kept(parts, base_next..group.base.start, &mut combined.lines);
        put_together(parts, &group, grouping, pairing, &mut combined)?;
        base_next = group.base.end;
    }
    kept(parts, base_next..parts.base.len(), &mut combined.lines);
    Some(combined)
}

/// Writes lines of the base that neither side changed, as the current side has them.
fn kept<'a>(parts: &Parts<'_, 'a>, base: Range<usize>, lines: &mut Vec<Written<'a>>) {
    let start = side_at(&parts.current.changes, base.start, |change| {
        change.base.start <= base.start
    });
    let side = parts.current.lines.get(start..start + base.len()).unwrap_or_default();
    lines.extend(side.iter().map(|line| Written {
        line: *line,
        source: Source::Kept,
    }));
}

/// Where a side is just before a line of the base that no change of it covers, before any insertion there.
fn side_before(changes: &[SideChange], base: usize) -> usize {
    side_at(changes, base, |change| change.base.start < base)
}

/// Where a side is at a place of the base that no change of it covers, once the changes before it are counted.
fn side_at(changes: &[SideChange], base: usize, is_before: impl Fn(&SideChange) -> bool) -> usize {
    changes
        .iter()
        .rev()
        .find(|change| is_before(change))
        .map_or(base, |change| change.side.end + base.saturating_sub(change.base.end))
}

/// A side's lines over the lines of the base of a group, and its changes counted from the start of the group.
fn in_group<'v, 'a>(version: &Version<'v, 'a>, members: &[SideChange], base: &Range<usize>) -> Option<Version<'v, 'a>> {
    let start = side_before(&version.changes, base.start);
    let end = members.last().map_or(start + base.len(), |last| {
        last.side.end + base.end.saturating_sub(last.base.end)
    });
    Some(Version {
        lines: version.lines.get(start..end)?,
        changes: members
            .iter()
            .map(|change| SideChange {
                base: change.base.start - base.start..change.base.end - base.start,
                side: change.side.start - start..change.side.end - start,
            })
            .collect(),
    })
}

/// Writes a group put together.
fn put_together<'a>(
    parts: &Parts<'_, 'a>,
    group: &Group,
    grouping: Grouping,
    pairing: Pairing<'_>,
    combined: &mut Combined<'a>,
) -> Option<()> {
    let local = Parts {
        base: parts.base.get(group.base.clone())?,
        current: in_group(&parts.current, &group.current, &group.base)?,
        incoming: in_group(&parts.incoming, &group.incoming, &group.base)?,
    };
    let (current, incoming) = (local.current.lines, local.incoming.lines);
    if group.incoming.is_empty() {
        combined.lines.extend(write(current, Source::Current));
    } else if group.current.is_empty() {
        combined.lines.extend(write(incoming, Source::Incoming));
    } else if texts(current) == texts(incoming) {
        combined.lines.extend(write(current, Source::Both));
    } else if grouping == Grouping::Overlap {
        return None;
    } else if let Some(lines) = meet(&local, pairing) {
        combined.met = true;
        combined.lines.extend(lines);
    } else {
        let (lines, source) = spaces(&local)?;
        combined.met = true;
        combined.spaces = true;
        combined.lines.extend(write(lines, source));
    }
    Some(())
}

/// Some lines written, all from the same place.
fn write<'l, 'a>(lines: &'l [Line<'a>], source: Source) -> impl Iterator<Item = Written<'a>> + 'l {
    lines.iter().map(move |line| Written { line: *line, source })
}

/// The rules of spaces on a group where the two sides met: the side to write, and where its lines come from.
fn spaces<'v, 'a>(group: &Parts<'v, 'a>) -> Option<(&'v [Line<'a>], Source)> {
    let (base, current, incoming) = (group.base, group.current.lines, group.incoming.lines);
    if is_same_by(current, base, without_spaces_at_the_end) {
        Some((incoming, Source::Incoming))
    } else if is_same_by(incoming, base, without_spaces_at_the_end) {
        Some((current, Source::Current))
    } else if is_same_by(current, incoming, with_spaces_as_one) {
        Some((current, Source::Both))
    } else {
        None
    }
}

/// Whether two lists of lines are the same once each line is seen in a way.
fn is_same_by(left: &[Line<'_>], right: &[Line<'_>], seen: fn(&str) -> String) -> bool {
    left.len() == right.len()
        && left
            .iter()
            .zip(right)
            .all(|(left, right)| seen(left.text) == seen(right.text))
}

/// A line without the spaces and tabs at its end.
fn without_spaces_at_the_end(text: &str) -> String {
    text.trim_end_matches([' ', '\t']).to_owned()
}

/// A line with every run of spaces and tabs as one space, and none at its ends.
fn with_spaces_as_one(text: &str) -> String {
    text.split([' ', '\t'])
        .filter(|word| !word.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

/// Whether two changes go together: they touch when they share a line of the base or are next to each other; they
/// overlap when they share a line, an insertion overlapping a change of lines only strictly inside it, and two
/// insertions only at the same place.
fn goes_with(left: &Range<usize>, right: &Range<usize>, grouping: Grouping) -> bool {
    match (grouping, left.is_empty(), right.is_empty()) {
        (Grouping::Touch, _, _) => left.start <= right.end && right.start <= left.end,
        (Grouping::Overlap, true, true) => left.start == right.start,
        (Grouping::Overlap, true, false) => right.start < left.start && left.start < right.end,
        (Grouping::Overlap, false, true) => left.start < right.start && right.start < left.end,
        (Grouping::Overlap, false, false) => left.start < right.end && right.start < left.end,
    }
}

/// The groups of the changes of two sides, in the order of the base: each starts with the next change and takes every
/// change of either side that goes with one of the other side already in it, until none does.
fn groups(current: &[SideChange], incoming: &[SideChange], grouping: Grouping) -> Vec<Group> {
    let mut all: Vec<(Side, &SideChange)> = current
        .iter()
        .map(|change| (Side::Current, change))
        .chain(incoming.iter().map(|change| (Side::Incoming, change)))
        .collect();
    all.sort_by_key(|(_, change)| (change.base.start, !change.base.is_empty()));
    let mut taken = vec![false; all.len()];
    let mut groups = Vec::new();
    for first in 0..all.len() {
        if taken.get(first) == Some(&false) {
            groups.push(grow(&all, &mut taken, first, grouping));
        }
    }
    groups.sort_by_key(|group| (group.base.start, !group.base.is_empty()));
    groups
}

/// The group that starts with a change.
fn grow(all: &[(Side, &SideChange)], taken: &mut [bool], first: usize, grouping: Grouping) -> Group {
    let mut members = vec![first];
    if let Some(slot) = taken.get_mut(first) {
        *slot = true;
    }
    loop {
        let joining: Vec<usize> = all
            .iter()
            .enumerate()
            .filter(|(index, (side, change))| {
                taken.get(*index) == Some(&false)
                    && members
                        .iter()
                        .filter_map(|member| all.get(*member))
                        .any(|(other, member)| other != side && goes_with(&change.base, &member.base, grouping))
            })
            .map(|(index, _)| index)
            .collect();
        if joining.is_empty() {
            break;
        }
        for index in joining {
            if let Some(slot) = taken.get_mut(index) {
                *slot = true;
            }
            members.push(index);
        }
    }
    members.sort_unstable();
    let of = |wanted: Side| -> Vec<SideChange> {
        members
            .iter()
            .filter_map(|member| all.get(*member))
            .filter(|(side, _)| *side == wanted)
            .map(|(_, change)| (*change).clone())
            .collect()
    };
    let (current, incoming) = (of(Side::Current), of(Side::Incoming));
    let start = current.iter().chain(&incoming).map(|change| change.base.start).min();
    let end = current.iter().chain(&incoming).map(|change| change.base.end).max();
    Group {
        base: start.unwrap_or(0)..end.unwrap_or(0),
        current,
        incoming,
    }
}
