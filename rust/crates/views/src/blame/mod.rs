//! The blame in the File page: who last changed the line under the cursor, at its end (`blame.current-line`), and who
//! last changed each line, in a column before the text (`blame.file`), each with a hover that names the commit and
//! links to it; and what the page asks of the operations of the blame and of the history.

mod card;
mod gutter;
mod note;

use std::ops::Range;

use model::blame::BlameLine;
use model::contract::FeatureId;
use ui::{CodePlaces, CodeRows, Color};

use crate::file_view::FilePageId;

pub use gutter::{file_view, runs_of};
pub use note::current_line_view;

/// Something asked of the blame of a File page.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BlameIntent {
    /// Run an operation of the blame or of the history on a File page: on the line of its cursor, or on a line given,
    /// from 1, by a link of the hover.
    Run(FilePageId, FeatureId, Option<usize>),
    /// The hover of the blame of a line, from 0, opened; or none, closed.
    Hover(FilePageId, Option<usize>),
}

/// The blame a File page shows.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PageBlame<'a> {
    /// Whether the page shows a file of a repository, whose blame can be asked for: its title has the person.
    pub can_blame: bool,
    /// The blame of each line, shown in the column before the text.
    pub file: Option<FileBlame<'a>>,
    /// What the end of the line under the cursor says.
    pub note: Option<LineNote<'a>>,
    /// The line, from 0, whose hover is open.
    pub hovered: Option<usize>,
    /// Whether whitespace and moved lines are ignored: the funnel is filled.
    pub ignores_whitespace: bool,
    /// Whether every file shows its blame: the pin is pinned.
    pub is_on_every_file: bool,
}

impl PageBlame<'_> {
    /// How many columns the column of the blame takes before the text: none while the file shows no blame.
    #[must_use]
    pub fn gutter_columns(&self) -> usize {
        self.file.map_or(0, |file| file.width + 1)
    }
}

/// The blame of each line of a file, as the column before its text shows it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FileBlame<'a> {
    /// Who last changed each line; none for a line git said nothing of.
    pub lines: &'a [Option<BlameLine>],
    /// What is said beside each line (`blame.format-line`); empty for a line that says nothing.
    pub texts: &'a [String],
    /// The color of the bar of each line (`blame.age-color`); none for no bar.
    pub bars: &'a [Option<Color>],
    /// How many columns the longest text takes.
    pub width: usize,
}

/// What the end of the line under the cursor says.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LineNote<'a> {
    /// The line, from 0.
    pub line: usize,
    /// What it says: `Ann Lee, 3 days ago • Fix the parser`, or `Not committed yet`.
    pub text: &'a str,
    /// Who last changed it.
    pub blame: &'a BlameLine,
}

/// What the column of the blame is drawn with, beside the editor of a File page.
#[derive(Clone, Debug, PartialEq)]
pub struct FileBlameView<'a> {
    /// The page.
    pub page: FilePageId,
    /// The blame shown.
    pub blame: PageBlame<'a>,
    /// Where the editor drew its parts.
    pub places: CodePlaces,
    /// The rows the lines of the editor take: a line that wraps takes several, and its blame stays on the first.
    pub rows: &'a CodeRows,
    /// The lines in sight.
    pub visible: Range<usize>,
    /// The line of the cursor, from 0.
    pub caret_line: usize,
    /// How many lines the file has.
    pub line_count: usize,
    /// The text of the line of the cursor, after whose end its blame is said.
    pub caret_text: &'a str,
    /// How many columns a tab goes to.
    pub tab_width: usize,
}

impl FileBlameView<'_> {
    /// Where a line in sight starts, down, and how tall all its rows are.
    fn line_place(&self, line: usize) -> (f32, f32) {
        let rows_above = self
            .rows
            .first_row(line)
            .saturating_sub(self.rows.first_row(self.visible.start));
        let height = self.places.line_height;
        (
            self.places.first_top + crate::number::float_of_count(rows_above) * height,
            crate::number::float_of_count(self.rows.rows_in(line)) * height,
        )
    }
}

/// The commit of a line, when it is committed.
fn commit_of(line: Option<&Option<BlameLine>>) -> Option<&model::CommitId> {
    match line {
        Some(Some(BlameLine::Committed(commit))) => Some(&commit.id),
        _ => None,
    }
}
