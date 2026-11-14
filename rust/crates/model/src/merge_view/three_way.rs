//! The regions of a file in conflict: the changes of each side grouped where they touch, the result git would have
//! written from them, and where each region is in the result as it is now.

use std::ops::Range;

use super::{MarkedBlock, MergeRegions, Region, RegionKind, RegionState};
use crate::diff::{LineChange, WordMarks};

/// The minimal line diff from a text to another; none when it takes longer than its time.
pub type LineDiff<'a> = &'a dyn Fn(&[&str], &[&str]) -> Option<Vec<LineChange>>;

/// The four texts of a file in conflict, as lines with `\r\n` read as `\n`, the conflicts git still marks in the
/// result, and the diffs the regions are made with.
pub struct ThreeWayInput<'a> {
    /// The lines of the base; none when both sides added the file.
    pub base: &'a [&'a str],
    /// The lines of the current side.
    pub current: &'a [&'a str],
    /// The lines of the incoming side.
    pub incoming: &'a [&'a str],
    /// The lines of the result, as it is now.
    pub result: &'a [&'a str],
    /// The conflicts git still marks in the result (`conflicts.parse-markers`).
    pub marked: &'a [MarkedBlock],
    /// The minimal line diff from a text to another (`diff.line-diff`); none when it takes longer than its time.
    pub line_diff: LineDiff<'a>,
    /// The words that changed in two paired lines (`diff.word-diff`).
    pub word_marks: &'a dyn Fn(&str, &str) -> WordMarks,
}

/// The changes of each side that touch, grouped: by their places among the changes of each side.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Group {
    /// The changes of the current side; an empty range at the number of those before it when it has none.
    current: Range<usize>,
    /// The changes of the incoming side.
    incoming: Range<usize>,
}

/// The base, the two sides and the result aligned: what each side changed of the base, which of those changes
/// conflict, and where each one is in the result as it is now. The changes of the two sides that touch (they share a
/// line of the base, or are next to each other) are one region; the result git would have written is diffed with the
/// real one to find each region in it; a conflict git still marks covers its whole block; what the result has in each
/// other region is the base, a side, both sides in either order, or else edited. None when a diff takes longer than its
/// time.
///
/// Feature: merge-view.three-way
#[must_use]
pub fn three_way(input: &ThreeWayInput<'_>) -> Option<MergeRegions> {
    let current_changes = (input.line_diff)(input.base, input.current)?;
    let incoming_changes = (input.line_diff)(input.base, input.incoming)?;
    let mut regions: Vec<Region> = groups(&current_changes, &incoming_changes)
        .iter()
        .map(|group| region_of(group, (&current_changes, &incoming_changes), input))
        .collect();
    let (expected, places) = written(&regions, input);
    let changes = (input.line_diff)(&expected, input.result)?;
    for (region, place) in regions.iter_mut().zip(places) {
        region.result = carried(&place, &changes);
    }
    let mut regions = with_marked(regions, input.marked);
    for region in &mut regions {
        if region.marked.is_none() {
            region.state = state_of(region, input);
        }
        add_marks(region, input);
    }
    Some(MergeRegions { regions })
}

/// The changes of the two sides grouped, in the order of the base: a change joins the group when it touches a change of
/// the other side already in it.
fn groups(current: &[LineChange], incoming: &[LineChange]) -> Vec<Group> {
    let mut found: Vec<Group> = Vec::new();
    let (mut next_current, mut next_incoming) = (0, 0);
    let mut ends: (Option<usize>, Option<usize>) = (None, None);
    loop {
        let (is_current, change) = match (current.get(next_current), incoming.get(next_incoming)) {
            (Some(first), Some(second)) => {
                if (first.left.start, first.left.end) <= (second.left.start, second.left.end) {
                    (true, first)
                } else {
                    (false, second)
                }
            }
            (Some(first), None) => (true, first),
            (None, Some(second)) => (false, second),
            (None, None) => break,
        };
        let other_end = if is_current { ends.1 } else { ends.0 };
        let joins = !found.is_empty() && other_end.is_some_and(|end| change.left.start <= end);
        if !joins {
            found.push(Group {
                current: next_current..next_current,
                incoming: next_incoming..next_incoming,
            });
            ends = (None, None);
        }
        if let Some(group) = found.last_mut() {
            if is_current {
                next_current += 1;
                group.current.end = next_current;
                ends.0 = Some(ends.0.map_or(change.left.end, |end| end.max(change.left.end)));
            } else {
                next_incoming += 1;
                group.incoming.end = next_incoming;
                ends.1 = Some(ends.1.map_or(change.left.end, |end| end.max(change.left.end)));
            }
        }
    }
    found
}

/// The region of a group: its stretch of the base, and the stretch of each side that stands for it.
fn region_of(group: &Group, (current, incoming): (&[LineChange], &[LineChange]), input: &ThreeWayInput<'_>) -> Region {
    let members = || {
        current
            .get(group.current.clone())
            .unwrap_or_default()
            .iter()
            .chain(incoming.get(group.incoming.clone()).unwrap_or_default())
    };
    let start = members().map(|change| change.left.start).min().unwrap_or(0);
    let end = members().map(|change| change.left.end).max().unwrap_or(start);
    let base = start..end;
    let current_lines = side_range(current, &group.current, &base);
    let incoming_lines = side_range(incoming, &group.incoming, &base);
    let kind = match (group.current.is_empty(), group.incoming.is_empty()) {
        (false, true) => RegionKind::Current,
        (true, false) => RegionKind::Incoming,
        _ if lines(input.current, &current_lines) == lines(input.incoming, &incoming_lines) => RegionKind::Both,
        _ => RegionKind::Conflict,
    };
    Region {
        kind,
        base,
        current: current_lines,
        incoming: incoming_lines,
        result: 0..0,
        state: RegionState::Edited,
        marked: None,
        current_marks: Vec::new(),
        incoming_marks: Vec::new(),
    }
}

/// The lines of a side that stand for a stretch of the base, given the changes of that side in the group.
fn side_range(changes: &[LineChange], members: &Range<usize>, base: &Range<usize>) -> Range<usize> {
    let in_group = changes.get(members.clone()).unwrap_or_default();
    if let (Some(first), Some(last)) = (in_group.first(), in_group.last()) {
        let start = first.right.start.saturating_sub(first.left.start - base.start);
        return start..last.right.end + (base.end - last.left.end);
    }
    let before = members.start.checked_sub(1).and_then(|place| changes.get(place));
    let start = before.map_or(base.start, |change| change.right.end + (base.start - change.left.end));
    start..start + base.len()
}

/// The lines of a text at a range, empty past its end.
fn lines<'a>(text: &[&'a str], range: &Range<usize>) -> Vec<&'a str> {
    text.get(range.clone()).unwrap_or_default().to_vec()
}

/// The result git would have written from the regions, and where each region is in it: the current side's lines
/// between regions, the changed side's lines in a region of one side, the current side's in a region of both, and the
/// two sides between markers for a conflict.
fn written<'a>(regions: &[Region], input: &ThreeWayInput<'a>) -> (Vec<&'a str>, Vec<Range<usize>>) {
    let mut expected: Vec<&'a str> = Vec::new();
    let mut places = Vec::new();
    let mut after = 0;
    for region in regions {
        expected.extend(lines(input.current, &(after..region.current.start.max(after))));
        let start = expected.len();
        match region.kind {
            RegionKind::Current | RegionKind::Both => expected.extend(lines(input.current, &region.current)),
            RegionKind::Incoming => expected.extend(lines(input.incoming, &region.incoming)),
            RegionKind::Conflict => {
                expected.push("<<<<<<<");
                expected.extend(lines(input.current, &region.current));
                expected.push("=======");
                expected.extend(lines(input.incoming, &region.incoming));
                expected.push(">>>>>>>");
            }
        }
        places.push(start..expected.len());
        after = region.current.end.max(after);
    }
    expected.extend(lines(input.current, &(after..input.current.len().max(after))));
    (expected, places)
}

/// Where a stretch of the result git would have written is in the real result: a place inside a changed stretch, or at
/// an insertion, goes to the start of what replaced it for the start of the stretch and to its end for its end.
fn carried(place: &Range<usize>, changes: &[LineChange]) -> Range<usize> {
    let start = carried_place(place.start, changes, |change| change.right.start);
    let end = carried_place(place.end, changes, |change| change.right.end).max(start);
    start..end
}

/// Where a place of the result git would have written is in the real result.
fn carried_place(place: usize, changes: &[LineChange], inside: fn(&LineChange) -> usize) -> usize {
    let mut carried = place;
    for change in changes {
        let is_inside = change.left.start < place && place < change.left.end;
        let is_insertion = change.left.is_empty() && change.left.start == place;
        if is_inside || is_insertion {
            return inside(change);
        }
        if change.left.end <= place {
            carried = change.right.end + (place - change.left.end);
        } else {
            break;
        }
    }
    carried
}

/// The regions with each conflict git still marks made one unresolved region that covers its whole block: the regions
/// it overlaps put together, or a region of its own when it overlaps none.
fn with_marked(mut regions: Vec<Region>, marked: &[MarkedBlock]) -> Vec<Region> {
    for block in marked {
        let overlapping: Vec<usize> = (0..regions.len())
            .filter(|&place| {
                regions
                    .get(place)
                    .is_some_and(|region| overlaps(&region.result, &block.span))
            })
            .filter(|&place| regions.get(place).is_some_and(|region| region.marked.is_none()))
            .collect();
        let merged = if let (Some(&first), Some(&last)) = (overlapping.first(), overlapping.last()) {
            let together: Vec<Region> = regions.drain(first..=last).collect();
            (first, joined(&together))
        } else {
            let place = regions
                .iter()
                .position(|region| region.result.start >= block.span.end)
                .unwrap_or(regions.len());
            (
                place,
                placed(place.checked_sub(1).and_then(|before| regions.get(before)), block),
            )
        };
        let (place, mut region) = merged;
        region.kind = RegionKind::Conflict;
        region.result = block.span.clone();
        region.state = RegionState::Unresolved;
        region.marked = Some(block.clone());
        regions.insert(place, region);
    }
    regions
}

/// Whether a region of the result overlaps a block: an empty region is in a block when it is at one of its lines.
fn overlaps(region: &Range<usize>, block: &Range<usize>) -> bool {
    if region.is_empty() {
        block.start <= region.start && region.start < block.end
    } else {
        region.start < block.end && block.start < region.end
    }
}

/// Regions put together: from the start of the first to the end of the last, in the base and in each side.
fn joined(together: &[Region]) -> Region {
    let span = |range: fn(&Region) -> &Range<usize>| {
        let start = together.iter().map(|region| range(region).start).min().unwrap_or(0);
        let end = together.iter().map(|region| range(region).end).max().unwrap_or(start);
        start..end
    };
    Region {
        kind: RegionKind::Conflict,
        base: span(|region| &region.base),
        current: span(|region| &region.current),
        incoming: span(|region| &region.incoming),
        result: span(|region| &region.result),
        state: RegionState::Unresolved,
        marked: None,
        current_marks: Vec::new(),
        incoming_marks: Vec::new(),
    }
}

/// A region for a block that overlaps none: empty in the base and the sides, as far after the region before it as the
/// block is after it in the result.
fn placed(before: Option<&Region>, block: &MarkedBlock) -> Region {
    let gap = before.map_or(block.span.start, |region| {
        block.span.start.saturating_sub(region.result.end)
    });
    let at = |end: fn(&Region) -> usize| before.map_or(gap, |region| end(region) + gap);
    let empty = |end: fn(&Region) -> usize| at(end)..at(end);
    Region {
        kind: RegionKind::Conflict,
        base: empty(|region| region.base.end),
        current: empty(|region| region.current.end),
        incoming: empty(|region| region.incoming.end),
        result: block.span.clone(),
        state: RegionState::Unresolved,
        marked: None,
        current_marks: Vec::new(),
        incoming_marks: Vec::new(),
    }
}

/// What the result has in a region that git does not mark.
fn state_of(region: &Region, input: &ThreeWayInput<'_>) -> RegionState {
    let result = lines(input.result, &region.result);
    let base = lines(input.base, &region.base);
    let current = lines(input.current, &region.current);
    let incoming = lines(input.incoming, &region.incoming);
    let both = |first: &[&'_ str], second: &[&'_ str]| -> Vec<String> {
        first.iter().chain(second).map(|line| (*line).to_owned()).collect()
    };
    let result_lines: Vec<String> = result.iter().map(|line| (*line).to_owned()).collect();
    if result == base {
        RegionState::Base
    } else if result == current {
        RegionState::Current
    } else if result == incoming {
        RegionState::Incoming
    } else if result_lines == both(&current, &incoming) {
        RegionState::CurrentThenIncoming
    } else if result_lines == both(&incoming, &current) {
        RegionState::IncomingThenCurrent
    } else {
        RegionState::Edited
    }
}

/// The words each side changed in the region, on its lines paired in order with the base's.
fn add_marks(region: &mut Region, input: &ThreeWayInput<'_>) {
    let base = lines(input.base, &region.base);
    let marks = |side: &[&str]| -> Vec<Vec<Range<usize>>> {
        if side == base.as_slice() {
            return vec![Vec::new(); side.len()];
        }
        side.iter()
            .enumerate()
            .map(|(place, line)| {
                base.get(place)
                    .map_or_else(Vec::new, |paired| (input.word_marks)(paired, line).1)
            })
            .collect()
    };
    region.current_marks = marks(&lines(input.current, &region.current));
    region.incoming_marks = marks(&lines(input.incoming, &region.incoming));
}
