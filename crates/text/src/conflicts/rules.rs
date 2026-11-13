//! The conflicts of a file that need no choice between the two sides (`conflicts.resolve-by-rules`): two changes side
//! by side, the same change twice, or the same lines spaced differently. Every line written is a line of one of the
//! two sides, as it is in the file; anything that is not plainly one of these cases is left as git marked it.

mod combine;
mod meet;
mod sides;

#[cfg(test)]
mod tests;

use std::collections::BTreeSet;
use std::ops::Range;

use combine::{Combined, Grouping, Parts, Source, combine};
use sides::{Direction, Paired, Pairing, Places, Read, rebuild, texts};

use super::markers::{Line, MarkedConflict, parse_markers, split_lines};

/// The largest file the rules look into: 2 MB.
const LARGEST_FILE: usize = 2 * 1024 * 1024;

/// The most lines the file, its base or either side may have for the rules to look into it.
const MOST_LINES: usize = 50_000;

/// What the rules are given: the text of the file as it is on disk, and the three versions git keeps in the index.
pub struct RulesInput<'a, 'k> {
    /// The file as it is now on disk, with git's markers in it.
    pub file: &'a [u8],
    /// The base, `git show :1:<file>`; empty when there is none (both sides added the file).
    pub base: &'a [u8],
    /// The current side, `git show :2:<file>`.
    pub current: &'a [u8],
    /// The incoming side, `git show :3:<file>`.
    pub incoming: &'a [u8],
    /// Asked between steps of the diffs: false stops them, and the file is left alone. The caller reads the clock, so
    /// that this crate does not.
    pub keep_going: &'k dyn Fn() -> bool,
}

/// What the rules did with a file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RuleResolutions<'a> {
    /// The file was not looked into, and none of its conflicts is resolved.
    LeftAlone(LeftAlone),
    /// Each conflict of the file, in order, resolved or not.
    LookedInto(Vec<RuleConflict<'a>>),
}

/// Why a file was not looked into.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LeftAlone {
    /// It is larger than 2 MB.
    TooLarge,
    /// It has no `<<<<<<<` in it.
    NoMarkers,
    /// It has a NUL byte.
    Binary,
    /// It, its base or a side is not valid UTF-8.
    NotUtf8,
    /// It, its base or a side has more than 50,000 lines.
    TooManyLines,
    /// Its markers do not pair up.
    MarkersDoNotPair,
    /// Pairing its lines with those of the base took longer than its caller allowed.
    TooComplex,
}

/// A conflict of the file, as `conflicts.parse-markers` found it, and what the rules did with it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuleConflict<'a> {
    /// The conflict as git marked it.
    pub conflict: MarkedConflict,
    /// How it is resolved; none when it is left as git marked it.
    pub resolution: Option<RuleResolution<'a>>,
    /// What the base and each side have around it, where the sides are known to be at the same place; none when that
    /// place cannot be found.
    pub stretch: Option<Stretch<'a>>,
}

/// A conflict resolved: the lines of the file it replaces, and the lines to put in their place.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuleResolution<'a> {
    /// Why it could be resolved.
    pub case: RuleCase,
    /// The lines of the file replaced, counted from 0: the markers, and the lines both sides share between them and
    /// the anchors.
    pub replaced: Range<usize>,
    /// The lines to put in their place, each with what ends it in the file.
    pub lines: Vec<Line<'a>>,
}

/// Why a conflict could be resolved, as the review page and the questions name it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RuleCase {
    /// Spaces or line endings only.
    SpacesOrLineEndings,
    /// The same change on both sides.
    SameChange,
    /// A change on one side only.
    OneSide,
    /// Changes to lines next to each other.
    NextToEachOther,
}

/// What the base and each side have from where the two sides are known to be at the same place before a conflict to
/// where they are again after it: what the review page shows each side changed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Stretch<'a> {
    /// Where its lines are in the base, counted from 0.
    pub base_lines: Range<usize>,
    /// The lines of the base.
    pub base: Vec<Line<'a>>,
    /// The lines of the current side.
    pub current: Vec<Line<'a>>,
    /// The lines of the incoming side.
    pub incoming: Vec<Line<'a>>,
}

/// The conflicts of a file that need no choice, resolved, and why each one could be.
///
/// The file is left alone when it is larger than 2 MB, has no `<<<<<<<`, has a NUL byte, is not UTF-8, has more than
/// 50,000 lines (it, its base or either side), or when its markers do not pair up. Otherwise each conflict is resolved
/// by the rules of the card, paired from the top and from the bottom, and left when the two do not agree.
///
/// Feature: conflicts.resolve-by-rules
#[must_use]
pub fn resolve_by_rules<'a>(input: &RulesInput<'a, '_>) -> RuleResolutions<'a> {
    match looked_into(input) {
        Ok(conflicts) => RuleResolutions::LookedInto(conflicts),
        Err(why) => RuleResolutions::LeftAlone(why),
    }
}

/// Each conflict of a file, with what the rules did with it; why the file is left alone when it is.
fn looked_into<'a>(input: &RulesInput<'a, '_>) -> Result<Vec<RuleConflict<'a>>, LeftAlone> {
    if input.file.len() > LARGEST_FILE {
        return Err(LeftAlone::TooLarge);
    }
    if !input.file.windows(7).any(|signs| signs == b"<<<<<<<") {
        return Err(LeftAlone::NoMarkers);
    }
    if input.file.contains(&0) {
        return Err(LeftAlone::Binary);
    }
    let decoded = [input.file, input.base, input.current, input.incoming].map(std::str::from_utf8);
    let [Ok(file), Ok(base), Ok(current), Ok(incoming)] = decoded else {
        return Err(LeftAlone::NotUtf8);
    };
    let [file_lines, base, current, incoming] = [file, base, current, incoming].map(split_lines);
    if [&file_lines, &base, &current, &incoming]
        .iter()
        .any(|lines| lines.len() > MOST_LINES)
    {
        return Err(LeftAlone::TooManyLines);
    }
    let conflicts = parse_markers(file).ok_or(LeftAlone::MarkersDoNotPair)?;
    let rebuilt = rebuild(&file_lines, &conflicts);
    let (own_current, own_incoming) = (texts(&current), texts(&incoming));
    let read = Read {
        file: &file_lines,
        base: &base,
        own_current: &own_current,
        own_incoming: &own_incoming,
        conflicts: &conflicts,
        rebuilt: &rebuilt,
    };
    let paired = |direction| {
        let pairing = Pairing {
            direction,
            keep_going: input.keep_going,
        };
        Paired::new(&read, pairing).ok_or(LeftAlone::TooComplex)
    };
    let both = [paired(Direction::FromTheTop)?, paired(Direction::FromTheBottom)?];
    Ok(each_conflict(&both))
}

/// What the rules did with each conflict, paired from the top and from the bottom.
fn each_conflict<'a>(both: &[Paired<'_, 'a>; 2]) -> Vec<RuleConflict<'a>> {
    let [top, _] = both;
    let mut replaced: Vec<Range<usize>> = Vec::new();
    let mut found = Vec::new();
    for (index, conflict) in top.read.conflicts.iter().enumerate() {
        let stretch = top.places(index).and_then(|places| stretch(top, &places));
        let resolution = (!conflict.is_broken)
            .then(|| agreed(both, index))
            .flatten()
            .filter(|resolution| {
                replaced
                    .iter()
                    .all(|other| other.end <= resolution.replaced.start || resolution.replaced.end <= other.start)
            });
        replaced.extend(resolution.as_ref().map(|resolution| resolution.replaced.clone()));
        found.push(RuleConflict {
            conflict: conflict.clone(),
            resolution,
            stretch,
        });
    }
    found
}

/// The resolution of a conflict when pairing from the top and from the bottom give the same lines over the same lines
/// of the file.
fn agreed<'a>(both: &[Paired<'_, 'a>; 2], index: usize) -> Option<RuleResolution<'a>> {
    let [top, bottom] = both;
    let from_the_top = resolved(top, index)?;
    let from_the_bottom = resolved(bottom, index)?;
    (from_the_top.replaced == from_the_bottom.replaced && from_the_top.lines == from_the_bottom.lines)
        .then_some(from_the_top)
}

/// What the base and each side have in the stretch of a conflict.
fn stretch<'a>(paired: &Paired<'_, 'a>, places: &Places) -> Option<Stretch<'a>> {
    let parts = paired.parts(places)?;
    Some(Stretch {
        base_lines: places.base.clone(),
        base: parts.base.to_vec(),
        current: parts.current.lines.to_vec(),
        incoming: parts.incoming.lines.to_vec(),
    })
}

/// A conflict resolved with the sides paired in one direction; none when it is left.
fn resolved<'a>(paired: &Paired<'_, 'a>, index: usize) -> Option<RuleResolution<'a>> {
    let conflict = paired.read.conflicts.get(index)?;
    let places = paired.places(index)?;
    if !paired.sides_have_it(&places) {
        return None;
    }
    let parts = paired.parts(&places)?;
    if !paired.is_shared_outside(&places, &parts, index) || !has_base_written(paired, &places, &parts, conflict) {
        return None;
    }
    let combined = combine(&parts, Grouping::Touch, paired.pairing)?;
    if is_written_twice(&combined) {
        return None;
    }
    let between_markers = |lines: &Range<usize>| texts(paired.read.file.get(lines.clone()).unwrap_or_default());
    if !combined.met && between_markers(&conflict.current) != between_markers(&conflict.incoming) {
        return None;
    }
    let lines: Vec<Line<'a>> = combined.lines.iter().map(|written| written.line).collect();
    Some(RuleResolution {
        case: case(&combined, &lines, &parts),
        replaced: places.file,
        lines,
    })
}

/// Whether the base of the stretch is, when git wrote the base between the markers, the lines of the base both sides
/// keep above the markers, then the base git wrote, then those both keep below them: a stretch that is not is paired
/// otherwise than git paired it, and what it resolves is not what git stopped for.
fn has_base_written(
    paired: &Paired<'_, '_>,
    places: &Places,
    parts: &Parts<'_, '_>,
    conflict: &MarkedConflict,
) -> bool {
    let Some(written) = &conflict.base else {
        return true;
    };
    let written = texts(paired.read.file.get(written.clone()).unwrap_or_default());
    let kept = parts.current.kept_lines();
    let above = conflict.span.start.saturating_sub(places.file.start);
    let below = places.file.end.saturating_sub(conflict.span.end);
    let base_of = |kept: &[Option<usize>]| -> Vec<&str> {
        kept.iter()
            .flatten()
            .filter_map(|line| parts.base.get(*line))
            .map(|line| line.text)
            .collect()
    };
    let upper = base_of(kept.get(..above).unwrap_or_default());
    let lower = base_of(kept.get(kept.len().saturating_sub(below)..).unwrap_or_default());
    let expected: Vec<&str> = upper.into_iter().chain(written).chain(lower).collect();
    texts(parts.base) == expected
}

/// Whether a line was written by the current side in a change of its own and the same line by the incoming side in a
/// change of its own: either one change was read as two or two as one, and the result would have that line twice.
fn is_written_twice(combined: &Combined<'_>) -> bool {
    let by = |source: Source| -> BTreeSet<&str> {
        combined
            .lines
            .iter()
            .filter(|written| written.source == source)
            .map(|written| written.line.text)
            .collect()
    };
    !by(Source::Current).is_disjoint(&by(Source::Incoming))
}

/// Why a conflict could be resolved.
fn case(combined: &Combined<'_>, lines: &[Line<'_>], parts: &Parts<'_, '_>) -> RuleCase {
    let (current, incoming) = (parts.current.lines, parts.incoming.lines);
    let result = texts(lines);
    let (is_current, is_incoming) = (result == texts(current), result == texts(incoming));
    if combined.spaces {
        RuleCase::SpacesOrLineEndings
    } else if is_current && is_incoming && lines == current && lines == incoming {
        RuleCase::SameChange
    } else if is_current && is_incoming {
        RuleCase::SpacesOrLineEndings
    } else if is_current || is_incoming {
        RuleCase::OneSide
    } else {
        RuleCase::NextToEachOther
    }
}
