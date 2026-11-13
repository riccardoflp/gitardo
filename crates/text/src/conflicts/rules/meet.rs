//! Where the two sides met: the lines they have alike, written once when both sides put them at the same lines of the
//! base, and what is between two of them put together on its own.
//!
//! A place in the base is counted in halves: `2 × k` is just before the line `k` of the base and `2 × k + 1` is that
//! line itself. A line a side kept is at its line; a line a side wrote in place of the lines `[b, e)` of the base,
//! after `j` lines of its own, can only stand among those lines, after `j` of them: anywhere from `2 × min(b + j, e)`
//! to `2 × e`.

use super::combine::{Grouping, Parts, Source, Version, Written, combine};
use super::sides::{Pairing, texts};

/// Where a part of a group starts or ends: in the base and in each side.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Cut {
    /// The line of the base.
    base: usize,
    /// The line of the current side.
    current: usize,
    /// The line of the incoming side.
    incoming: usize,
}

/// The lines of a group where the two sides changed the same lines differently, put together; none when a line they
/// have alike is not at the same lines of the base for both, or what is between two such lines cannot be put together.
pub(super) fn meet<'a>(group: &Parts<'_, 'a>, pairing: Pairing<'_>) -> Option<Vec<Written<'a>>> {
    let base_length = group.base.len();
    let current_places = places(&group.current, base_length);
    let incoming_places = places(&group.incoming, base_length);
    let paired = pairing.paired(&texts(group.current.lines), &texts(group.incoming.lines))?;
    let mut written = Vec::new();
    let mut after = Cut {
        base: 0,
        current: 0,
        incoming: 0,
    };
    let mut bound = 0;
    for (incoming, current) in paired
        .iter()
        .enumerate()
        .filter_map(|(incoming, current)| current.map(|current| (incoming, current)))
    {
        let point = point(*current_places.get(current)?, *incoming_places.get(incoming)?, bound)?;
        let before = Cut {
            base: point / 2,
            current,
            incoming,
        };
        between(group, after, before, pairing, &mut written)?;
        written.push(Written {
            line: *group.current.lines.get(current)?,
            source: Source::Both,
        });
        bound = point + point % 2;
        after = Cut {
            base: point.div_ceil(2),
            current: current + 1,
            incoming: incoming + 1,
        };
    }
    let end = Cut {
        base: base_length,
        current: group.current.lines.len(),
        incoming: group.incoming.lines.len(),
    };
    between(group, after, end, pairing, &mut written)?;
    Some(written)
}

/// Where each line of a side may stand in the base, from the first place to the last, in halves of lines.
fn places(version: &Version<'_, '_>, base_length: usize) -> Vec<(usize, usize)> {
    let mut found = Vec::with_capacity(version.lines.len());
    let mut base = 0;
    let at = |kept: usize| (2 * kept + 1, 2 * kept + 1);
    for change in &version.changes {
        found.extend((base..change.base.start).map(at));
        found.extend((0..change.side.len()).map(|before| {
            (
                2 * (change.base.start + before).min(change.base.end),
                2 * change.base.end,
            )
        }));
        base = change.base.end;
    }
    found.extend((base..base_length).map(at));
    found
}

/// Where a line both sides have stands: the first line of the base both allow, else the first place between lines,
/// not before the bound; none when they allow no place in common.
fn point(current: (usize, usize), incoming: (usize, usize), bound: usize) -> Option<usize> {
    let low = current.0.max(incoming.0).max(bound);
    let high = current.1.min(incoming.1);
    if low > high {
        None
    } else if low % 2 == 1 || low == high {
        Some(low)
    } else {
        Some(low + 1)
    }
}

/// Puts together what is between two lines the sides have alike, against the lines of the base left between them,
/// grouping changes that overlap.
fn between<'a>(
    group: &Parts<'_, 'a>,
    from: Cut,
    to: Cut,
    pairing: Pairing<'_>,
    written: &mut Vec<Written<'a>>,
) -> Option<()> {
    let base = group.base.get(from.base..to.base)?;
    let current = group.current.lines.get(from.current..to.current)?;
    let incoming = group.incoming.lines.get(from.incoming..to.incoming)?;
    let parts = Parts {
        base,
        current: pairing.version(base, current)?,
        incoming: pairing.version(base, incoming)?,
    };
    written.extend(combine(&parts, Grouping::Overlap, pairing)?.lines);
    Some(())
}
