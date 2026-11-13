//! The review of the resolutions of the conflicts (`review.page`): the head with its buttons, the files and the
//! filters (`review.file-list`), one card per conflict (`review.card`) or three columns (`review.columns`), the choices
//! (`review.choose`), the buttons that accept, and the report of what was written (`review.report`).
//!
//! The store derives what each card shows ([`ConflictCard`]); the views draw it and say what was clicked.

mod body;
mod card;
mod columns;
mod decide;
mod deleted;
mod files;
mod head;
mod lines;
#[cfg(test)]
mod tests;

use model::conflicts::SideNames;
use model::review::{ChoiceKind, DeletedFile, Origin, OriginCounts, ProposalSource, ProposalWarning, Shape, SideDiff};
use model::state::Operation;

pub use card::{CardState, card_view};
pub use columns::columns_view;
pub use decide::{accept_view, choose_view};
pub use deleted::{DeletedState, deleted_card_view};
pub use files::file_list_view;
pub use head::{FileState, accept_all_view, accept_file_view, clear_view, page_view, report_view, show_result_view};

/// The thousandths of the width each side column takes before a band is dragged (`review.columns`).
pub const EVEN: u32 = 333;

/// A conflict, wherever it is: its file, its place among the conflicts of the file, and its fingerprint
/// (`review.fingerprint`), which keeps what was accepted on it while the file is read again.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ConflictKey {
    /// The path of its file, from the top of the working tree.
    pub path: String,
    /// Its place among the conflicts of the file.
    pub index: usize,
    /// Its fingerprint.
    pub fingerprint: String,
}

/// How the conflicts are shown (`review.columns`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Layout {
    /// One card each, the sides over what is proposed.
    #[default]
    Cards,
    /// Three columns each: the current side, what is proposed and the incoming side.
    Columns,
}

/// Which conflicts the page and the list keep (`review.file-list`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Filter {
    /// Every conflict.
    #[default]
    All,
    /// The conflicts not accepted.
    NotAccepted,
    /// The conflicts with nothing proposed, and the files deleted on one side.
    NothingProposed,
    /// The conflicts the AI proposes for.
    FromTheAi,
}

/// A line of the file shown around a conflict, faded, with its number.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NumberedLine {
    /// Its number, as the file would number it with the proposal in place of the conflict.
    pub number: usize,
    /// Its text.
    pub text: String,
}

/// A line proposed, with its number and where it comes from.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProposedLine {
    /// Its number, as the file would number it with the proposal in place of the conflict, and its text.
    pub line: NumberedLine,
    /// Where it comes from (`review.proposal-lines`).
    pub origin: Origin,
    /// Whether a name set twice is on it (`review.proposal-warnings`).
    pub is_warned: bool,
}

/// What is proposed for a conflict, as its card shows it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CardProposal {
    /// Where it comes from.
    pub source: ProposalSource,
    /// What it does with the two sides.
    pub shape: Shape,
    /// How many of its lines come from each place.
    pub counts: OriginCounts,
    /// Its lines.
    pub lines: Vec<ProposedLine>,
    /// What looks like a mistake in it.
    pub warnings: Vec<ProposalWarning>,
    /// For each line proposed, the line of the current column it is joined to.
    pub from_current: Vec<Option<usize>>,
    /// For each line proposed, the line of the incoming column it is joined to.
    pub to_incoming: Vec<Option<usize>>,
}

/// A side of a conflict as a column shows it: the lines the proposal takes the place of, numbered as the file would
/// have them with that side in place of the conflict, and the 3 lines after.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct SideColumn {
    /// Its lines.
    pub lines: Vec<NumberedLine>,
    /// The 3 lines of the file after them.
    pub after: Vec<NumberedLine>,
}

/// One conflict, as its card shows it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConflictCard {
    /// Which conflict.
    pub key: ConflictKey,
    /// The line of its first marker, from 1.
    pub line: usize,
    /// What is proposed, when something is.
    pub proposal: Option<CardProposal>,
    /// The 3 lines of the file before what is proposed.
    pub before: Vec<NumberedLine>,
    /// The 3 lines of the file after it.
    pub after: Vec<NumberedLine>,
    /// What each side changed of the base (`review.side-diff`): the current side, then the incoming one.
    pub side_diffs: Box<(SideDiff, SideDiff)>,
    /// The lines of the base between the markers, when it is known.
    pub base: Option<Vec<String>>,
    /// The current column (`review.columns`).
    pub current: SideColumn,
    /// The incoming column.
    pub incoming: SideColumn,
    /// The choices offered (`review.choose`).
    pub choices: Vec<ChoiceKind>,
    /// Whether it is accepted.
    pub is_accepted: bool,
    /// What of it is shown.
    pub shown: CardShown,
}

/// What of a card is shown.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct CardShown {
    /// Whether it is folded to one line.
    pub is_folded: bool,
    /// Whether **Compare with the sides** is on.
    pub is_sides_shown: bool,
    /// Whether **Show the base** is on.
    pub is_base_shown: bool,
}

impl ConflictCard {
    /// Whether it is kept by a filter.
    #[must_use]
    pub fn is_kept_by(&self, filter: Filter) -> bool {
        match filter {
            Filter::All => true,
            Filter::NotAccepted => !self.is_accepted,
            Filter::NothingProposed => self.proposal.is_none(),
            Filter::FromTheAi => self
                .proposal
                .as_ref()
                .is_some_and(|proposal| matches!(proposal.source, ProposalSource::Ai { .. })),
        }
    }

    /// Whether its proposal could be accepted with the others: there is one, not accepted, without a warning.
    #[must_use]
    pub fn is_acceptable_with_others(&self) -> bool {
        !self.is_accepted
            && self
                .proposal
                .as_ref()
                .is_some_and(|proposal| proposal.warnings.is_empty())
    }
}

/// The conflicts of one file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FileCards {
    /// Its path, from the top of the working tree.
    pub path: String,
    /// Its conflicts.
    pub conflicts: Vec<ConflictCard>,
    /// Whether it is folded to its name.
    pub is_folded: bool,
}

impl FileCards {
    /// How many of its conflicts are accepted.
    #[must_use]
    pub fn accepted(&self) -> usize {
        self.conflicts.iter().filter(|conflict| conflict.is_accepted).count()
    }

    /// Whether some of its conflicts have nothing proposed.
    #[must_use]
    pub fn has_unproposed(&self) -> bool {
        self.conflicts.iter().any(|conflict| conflict.proposal.is_none())
    }
}

/// What the last write did (`review.report`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Report {
    /// How many conflicts were written.
    pub conflicts: usize,
    /// In how many files.
    pub files: usize,
    /// How many of them the AI proposed.
    pub from_ai: usize,
    /// How many files were left with no conflict, and staged.
    pub staged: usize,
    /// How many conflicts are left in the files written.
    pub conflicts_left: usize,
    /// How many files are still in conflict, once the repository is read again.
    pub files_left: usize,
}

/// How far the page is read.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReviewRead {
    /// Not read yet.
    Reading,
    /// Read.
    Read,
    /// Git could not read the files.
    Failed,
}

/// What the review page shows.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReviewState<'a> {
    /// Its title in the trail: **Conflicts · \<folder\>**.
    pub title: &'a str,
    /// How far it is read.
    pub read: ReviewRead,
    /// The operation that stopped.
    pub operation: Option<Operation>,
    /// What the two sides are.
    pub sides: &'a SideNames,
    /// The files with conflicts marked in them.
    pub files: &'a [FileCards],
    /// The files one side deleted and the other changed.
    pub deleted: &'a [DeletedFile],
    /// The other files in conflict.
    pub others: &'a [String],
    /// The filter chosen.
    pub filter: Filter,
    /// The conflicts accepted while **Not accepted** was on, kept in view until another filter is chosen.
    pub kept_in_view: &'a [ConflictKey],
    /// The file shown alone, when one is.
    pub file_shown: Option<&'a str>,
    /// Cards or Columns.
    pub layout: Layout,
    /// The widths of the current and the incoming columns, in thousandths of the width.
    pub widths: (u32, u32),
    /// What the last write did, until the page is asked something else.
    pub report: Option<&'a Report>,
    /// Whether the rules propose (`conflicts.autoResolve`) and the AI may be asked (`conflicts.ai`).
    pub proposers: model::review::Proposers,
    /// Whether an operation runs.
    pub is_running: bool,
    /// Whether **Continue the …** can run.
    pub can_continue: bool,
}

impl ReviewState<'_> {
    /// Every conflict of the page.
    pub fn conflicts(&self) -> impl Iterator<Item = &ConflictCard> {
        self.files.iter().flat_map(|file| &file.conflicts)
    }

    /// How many conflicts are accepted.
    #[must_use]
    pub fn accepted(&self) -> usize {
        self.conflicts().filter(|conflict| conflict.is_accepted).count()
    }

    /// Whether a conflict is kept by the filter and the file shown.
    #[must_use]
    pub fn is_shown(&self, card: &ConflictCard) -> bool {
        self.file_shown.is_none_or(|path| path == card.key.path)
            && (card.is_kept_by(self.filter) || self.kept_in_view.contains(&card.key))
    }

    /// Whether a file deleted on one side is kept by the filter and the file shown.
    #[must_use]
    pub fn is_deleted_shown(&self, file: &DeletedFile) -> bool {
        self.file_shown.is_none_or(|path| path == file.path)
            && matches!(self.filter, Filter::All | Filter::NotAccepted | Filter::NothingProposed)
    }

    /// How many conflicts a filter keeps, the files deleted on one side counting with those with nothing proposed.
    #[must_use]
    pub fn count(&self, filter: Filter) -> usize {
        let deleted = match filter {
            Filter::All | Filter::NotAccepted | Filter::NothingProposed => self.deleted.len(),
            Filter::FromTheAi => 0,
        };
        self.conflicts().filter(|conflict| conflict.is_kept_by(filter)).count() + deleted
    }

    /// How many proposals not accepted have a warning.
    #[must_use]
    pub fn warned(&self) -> usize {
        self.conflicts()
            .filter(|conflict| !conflict.is_accepted)
            .filter(|conflict| {
                conflict
                    .proposal
                    .as_ref()
                    .is_some_and(|proposal| !proposal.warnings.is_empty())
            })
            .count()
    }
}

/// What the review page asks of the store.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ReviewIntent {
    /// Open the page, of some files or of every file in conflict (`review.page`).
    Open(Option<Vec<String>>),
    /// Accept the proposal of a conflict (`review.accept`).
    Accept(ConflictKey),
    /// Take back its acceptance.
    TakeBack(ConflictKey),
    /// Accept every proposal of a file without a warning (`review.accept-file`).
    AcceptFile(String),
    /// Accept every proposal of the page without a warning (`review.accept-all`).
    AcceptAll,
    /// Take back every acceptance (`review.clear`).
    Clear,
    /// Put a choice in place of what is proposed, accepted at once (`review.choose`).
    Choose(ConflictKey, ChoiceKind),
    /// Drop a choice: what was proposed before comes back, not accepted.
    DropChoice(ConflictKey),
    /// Show or hide the sides of a card.
    ToggleSides(ConflictKey),
    /// Show or hide the base of a card.
    ToggleBase(ConflictKey),
    /// Fold or unfold a conflict by hand.
    Fold(ConflictKey, bool),
    /// Fold or unfold a file by hand.
    FoldFile(String, bool),
    /// Choose a filter.
    Filter(Filter),
    /// Show the conflicts of one file alone, or of every file.
    ShowFile(Option<String>),
    /// Show Cards or Columns.
    Layout(Layout),
    /// The widths of the current and the incoming columns, in thousandths; none makes them even again.
    Widths(Option<(u32, u32)>),
    /// Open the Diff page of the result (`review.show-result`).
    ShowResult,
    /// Write the accepted resolutions (`review.write`).
    Write,
    /// Put back the conflicts of the files written (`review.report`).
    Undo,
    /// Continue the operation (`review.report`).
    Continue,
}
