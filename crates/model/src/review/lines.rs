//! Where the lines of a proposal come from: each line paired in order with the same line of each side, and what the
//! proposal does with the two sides as a whole.

use super::trimmed;

/// The lines each side has where the proposal goes, and those of the base when it is known.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Sides<'a> {
    /// The current side's lines.
    pub current: &'a [&'a str],
    /// The incoming side's lines.
    pub incoming: &'a [&'a str],
    /// The base's lines, when it is known.
    pub base: Option<&'a [&'a str]>,
}

/// Where a line of a proposal comes from.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Origin {
    /// The current side has it.
    Current,
    /// The incoming side has it.
    Incoming,
    /// Both sides have it.
    Both,
    /// Neither side has it, but the base does: it is as it was before either side changed it.
    Base,
    /// Nothing has it: the proposal wrote it.
    New,
    /// An empty line.
    Blank,
}

/// What a proposal does with the two sides, the empty lines and the spaces at the ends left aside.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Shape {
    /// Nothing is left: the lines are deleted.
    Nothing,
    /// Exactly the current side.
    Current,
    /// Exactly the incoming side.
    Incoming,
    /// The current side, then the incoming one.
    CurrentFirst,
    /// The incoming side, then the current one.
    IncomingFirst,
    /// Anything else.
    Mixed,
}

/// How many lines of a proposal come from each place; the empty lines are not counted.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct OriginCounts {
    /// From the current side.
    pub current: usize,
    /// From the incoming side.
    pub incoming: usize,
    /// From both sides.
    pub both: usize,
    /// From the base.
    pub base: usize,
    /// From neither side nor the base.
    pub new: usize,
}

/// Where each line of a proposal comes from, the bands that join it to the sides, and what it does with them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProposalLines {
    /// The origin of each line proposed.
    pub origins: Vec<Origin>,
    /// For each line proposed, the line of the current side it is paired with in order, when it is.
    pub from_current: Vec<Option<usize>>,
    /// For each line proposed, the line of the incoming side it is paired with in order, when it is.
    pub to_incoming: Vec<Option<usize>>,
    /// What the proposal does with the two sides.
    pub shape: Shape,
    /// How many of its lines come from each place.
    pub counts: OriginCounts,
}

/// Where the lines of a proposal come from.
///
/// Two lines are the same when they differ only in the spaces at their ends. Each line that is not empty is paired, in
/// order, with the same line of each side, as many as can be; a line paired with both comes from both, with one from
/// that side. A line that cannot be paired in order still comes from the side that has it, from both when both have
/// it, then from the base; else it is new. The shape compares the lines that are not empty with the sides' and the
/// two sides one after the other.
///
/// Feature: review.proposal-lines
#[must_use]
pub fn proposal_lines(sides: Sides<'_>, proposed: &[&str]) -> ProposalLines {
    let from_current = pairs(proposed, sides.current);
    let to_incoming = pairs(proposed, sides.incoming);
    let has = |lines: &[&str], line: &str| lines.iter().any(|known| trimmed(known) == line);
    let mut counts = OriginCounts::default();
    let origins = proposed
        .iter()
        .zip(from_current.iter().zip(&to_incoming))
        .map(|(line, (current, incoming))| {
            let line = trimmed(line);
            let origin = match (current.is_some(), incoming.is_some()) {
                _ if line.is_empty() => Origin::Blank,
                (true, true) => Origin::Both,
                (true, false) => Origin::Current,
                (false, true) => Origin::Incoming,
                (false, false) => match (has(sides.current, line), has(sides.incoming, line)) {
                    (true, true) => Origin::Both,
                    (true, false) => Origin::Current,
                    (false, true) => Origin::Incoming,
                    (false, false) if sides.base.is_some_and(|base| has(base, line)) => Origin::Base,
                    (false, false) => Origin::New,
                },
            };
            count(&mut counts, origin);
            origin
        })
        .collect();
    ProposalLines {
        origins,
        from_current,
        to_incoming,
        shape: shape(sides, proposed),
        counts,
    }
}

/// Adds a line of an origin to the counts.
fn count(counts: &mut OriginCounts, origin: Origin) {
    match origin {
        Origin::Current => counts.current += 1,
        Origin::Incoming => counts.incoming += 1,
        Origin::Both => counts.both += 1,
        Origin::Base => counts.base += 1,
        Origin::New => counts.new += 1,
        Origin::Blank => {}
    }
}

/// The lines that are not empty, without the spaces at their ends.
fn kept<'a>(lines: &[&'a str]) -> Vec<&'a str> {
    lines
        .iter()
        .map(|line| trimmed(line))
        .filter(|line| !line.is_empty())
        .collect()
}

/// What the proposal does with the two sides.
fn shape(sides: Sides<'_>, proposed: &[&str]) -> Shape {
    let proposed = kept(proposed);
    let current = kept(sides.current);
    let incoming = kept(sides.incoming);
    let joined = |first: &[&str], second: &[&str]| {
        proposed.len() == first.len() + second.len() && proposed.starts_with(first) && proposed.ends_with(second)
    };
    if proposed.is_empty() {
        Shape::Nothing
    } else if proposed == current {
        Shape::Current
    } else if proposed == incoming {
        Shape::Incoming
    } else if joined(&current, &incoming) {
        Shape::CurrentFirst
    } else if joined(&incoming, &current) {
        Shape::IncomingFirst
    } else {
        Shape::Mixed
    }
}

/// For each line proposed, the line of the side it is paired with: the longest run of the same lines, in order, the
/// empty lines left out.
fn pairs(proposed: &[&str], side: &[&str]) -> Vec<Option<usize>> {
    let proposed: Vec<&str> = proposed.iter().map(|line| trimmed(line)).collect();
    let side: Vec<&str> = side.iter().map(|line| trimmed(line)).collect();
    let same = |left: usize, right: usize| {
        proposed
            .get(left)
            .zip(side.get(right))
            .is_some_and(|(line, known)| !line.is_empty() && line == known)
    };
    let longest = Longest::of(proposed.len(), side.len(), same);
    let mut paired = vec![None; proposed.len()];
    let (mut left, mut right) = (0, 0);
    while left < proposed.len() && right < side.len() {
        if same(left, right) {
            if let Some(pair) = paired.get_mut(left) {
                *pair = Some(right);
            }
            left += 1;
            right += 1;
        } else if longest.at(left + 1, right) >= longest.at(left, right + 1) {
            left += 1;
        } else {
            right += 1;
        }
    }
    paired
}

/// For each place in the proposal and in the side, how many lines can be paired from there on.
struct Longest {
    /// The counts, a row for each line proposed and one more.
    counts: Vec<usize>,
    /// The length of a row: the lines of the side and one more.
    width: usize,
}

impl Longest {
    /// The counts of `proposed` lines against `side` lines, `same` telling which are the same.
    fn of(proposed: usize, side: usize, same: impl Fn(usize, usize) -> bool) -> Longest {
        let width = side + 1;
        let mut longest = Longest {
            counts: vec![0; (proposed + 1) * width],
            width,
        };
        for left in (0..proposed).rev() {
            for right in (0..side).rev() {
                let count = if same(left, right) {
                    longest.at(left + 1, right + 1) + 1
                } else {
                    longest.at(left + 1, right).max(longest.at(left, right + 1))
                };
                if let Some(slot) = longest.counts.get_mut(left * width + right) {
                    *slot = count;
                }
            }
        }
        longest
    }

    /// How many lines can be paired among the proposal from `left` on and the side from `right` on.
    fn at(&self, left: usize, right: usize) -> usize {
        self.counts.get(left * self.width + right).copied().unwrap_or(0)
    }
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests {
    use super::{Origin, OriginCounts, Shape, Sides, proposal_lines};

    fn sides<'a>(current: &'a [&'a str], incoming: &'a [&'a str]) -> Sides<'a> {
        Sides {
            current,
            incoming,
            base: None,
        }
    }

    #[test]
    fn review_proposal_lines__a_line_out_of_order_still_comes_from_the_side_that_has_it() {
        let found = proposal_lines(sides(&["a", "b"], &["c"]), &["b", "a", "c"]);
        assert_eq!(found.origins, [Origin::Current, Origin::Current, Origin::Incoming]);
        assert_eq!(found.shape, Shape::Mixed);
        assert_eq!(found.to_incoming, [None, None, Some(0)]);
    }

    #[test]
    fn review_proposal_lines__the_two_sides_one_after_the_other_are_current_first() {
        let current = ["  \"version\": \"1.2.1\",", "  \"shipping\": true"];
        let incoming = ["  \"version\": \"1.3.0\",", "  \"discounts\": true"];
        let proposed = [current[0], current[1], incoming[0], incoming[1]];
        let found = proposal_lines(sides(&current, &incoming), &proposed);
        assert_eq!(found.shape, Shape::CurrentFirst);
        assert_eq!(
            found.counts,
            OriginCounts {
                current: 2,
                incoming: 2,
                ..OriginCounts::default()
            }
        );
        assert_eq!(found.from_current, [Some(0), Some(1), None, None]);
        let found = proposal_lines(
            sides(&current, &incoming),
            &[incoming[0], incoming[1], current[0], current[1]],
        );
        assert_eq!(found.shape, Shape::IncomingFirst);
    }

    #[test]
    fn review_proposal_lines__indentation_and_empty_lines_are_left_aside() {
        let found = proposal_lines(sides(&["    call();"], &["other();"]), &["call();", "", "other();"]);
        assert_eq!(found.origins, [Origin::Current, Origin::Blank, Origin::Incoming]);
        assert_eq!(
            found.counts.current + found.counts.incoming,
            2,
            "the empty line is not counted"
        );
        assert_eq!(found.shape, Shape::CurrentFirst);
    }

    #[test]
    fn a_line_both_sides_have_comes_from_both_and_one_of_the_base_from_the_base() {
        let base: &[&str] = &["old();"];
        let found = proposal_lines(
            Sides {
                current: &["shared();", "mine();"],
                incoming: &["shared();", "theirs();"],
                base: Some(base),
            },
            &["shared();", "old();", "written();"],
        );
        assert_eq!(found.origins, [Origin::Both, Origin::Base, Origin::New]);
        assert_eq!(found.shape, Shape::Mixed);
    }

    #[test]
    fn the_shapes_of_one_side_and_of_nothing() {
        let both = sides(&["a"], &["b"]);
        assert_eq!(proposal_lines(both, &["a"]).shape, Shape::Current);
        assert_eq!(proposal_lines(both, &[" b "]).shape, Shape::Incoming);
        assert_eq!(proposal_lines(both, &["", "  "]).shape, Shape::Nothing);
        assert_eq!(proposal_lines(both, &[]).shape, Shape::Nothing);
    }
}
