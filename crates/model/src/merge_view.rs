//! The Merge page of a file in conflict, as data: its regions, what each side changed of the base and where each change
//! is in the result as it is now (`merge-view.three-way`); the rows that keep the columns level; what accepting a side
//! puts in a region (`merge-view.take-side`, `merge-view.accept-simple`); and the line of the result a line of a side
//! stands for (`merge-view.open-at-line`).

mod choices;
mod rows;
mod three_way;

use std::ops::Range;

pub use choices::{Choice, Column, MergeTexts, conflicts_left, has_conflicts, result_line, simple_choices};
pub use rows::{ColumnLengths, MergeLayout, MergeRow, merge_rows};
pub use three_way::{ThreeWayInput, three_way};

/// A conflict git still marks in the result, by the places of its lines in the result, from 0.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct MarkedBlock {
    /// Its lines, from `<<<<<<<` to the line after `>>>>>>>`.
    pub span: Range<usize>,
    /// The lines of the current side, after `<<<<<<<` up to `|||||||` or `=======`.
    pub current: Range<usize>,
    /// The lines of the incoming side, after `=======` up to `>>>>>>>`.
    pub incoming: Range<usize>,
}

/// Which sides changed a region of the base.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum RegionKind {
    /// The current side alone: green.
    Current,
    /// The incoming side alone: blue.
    Incoming,
    /// Both sides, the same way: grey.
    Both,
    /// Both sides, differently: red.
    Conflict,
}

/// What the result has in a region, checked in this order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum RegionState {
    /// The lines of the base.
    Base,
    /// The lines of the current side.
    Current,
    /// The lines of the incoming side.
    Incoming,
    /// The current side's lines, then the incoming side's.
    CurrentThenIncoming,
    /// The incoming side's lines, then the current side's.
    IncomingThenCurrent,
    /// Anything else.
    Edited,
    /// A conflict git still marks.
    Unresolved,
}

/// A stretch of the base that one side or both changed, the stretch of each side that stands for it, and where it is
/// in the result.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Region {
    /// Which sides changed it.
    pub kind: RegionKind,
    /// Its lines in the base.
    pub base: Range<usize>,
    /// Its lines in the current side.
    pub current: Range<usize>,
    /// Its lines in the incoming side.
    pub incoming: Range<usize>,
    /// Its lines in the result.
    pub result: Range<usize>,
    /// What the result has there.
    pub state: RegionState,
    /// The conflict git still marks there, for an unresolved region.
    pub marked: Option<MarkedBlock>,
    /// For each line of the current side in the region, the words that changed against the line of the base it is
    /// paired with, as ranges of its bytes.
    pub current_marks: Vec<Vec<Range<usize>>>,
    /// For each line of the incoming side in the region, the words that changed against the base.
    pub incoming_marks: Vec<Vec<Range<usize>>>,
}

impl Region {
    /// Whether git still marks it as a conflict.
    #[must_use]
    pub fn is_unresolved(&self) -> bool {
        self.state == RegionState::Unresolved
    }
}

/// The regions of a file in conflict, in the order of the base.
#[derive(Clone, Debug, PartialEq, Eq, Default, Hash)]
pub struct MergeRegions {
    /// The regions.
    pub regions: Vec<Region>,
}

/// What Mark as resolved is given by the Merge page: the file, the result when it is not saved, and how many conflicts
/// git still marks in it (`merge-view.mark-resolved`).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct MergeResolution {
    /// Its path from the top of the working tree.
    pub path: String,
    /// The result, when it has changes that are not saved.
    pub unsaved: Option<Vec<u8>>,
    /// How many conflicts git still marks in the result.
    pub marked: usize,
}

/// Why the rules could resolve a conflict (`conflicts.resolve-by-rules`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum RuleReason {
    /// Spaces or line endings only.
    SpacesOrLineEndings,
    /// The same change on both sides.
    SameChange,
    /// A change on one side only.
    OneSide,
    /// Changes to lines next to each other.
    NextToEachOther,
}

/// What is proposed for a conflict git still marks: the lines of the result it replaces, and what it puts there.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Proposal {
    /// The region of the conflict, by its place.
    pub region: usize,
    /// The lines of the result it replaces: the markers, and maybe some lines around them.
    pub replaced: Range<usize>,
    /// What it puts in their place, each line with what ends it.
    pub text: String,
    /// Why the rules could resolve it.
    pub reason: RuleReason,
}
