//! The words of the review of the conflicts (`review.`): the page, its file list and filters, the cards, the choices,
//! the columns, the files deleted on one side and the report of what was written.

use model::ai::Confidence;
use model::conflicts::Side;
use model::review::{ChoiceKind, OriginCounts, ProposalWarning, RuleCase, Shape};
use model::state::Operation;

mod written;

pub use written::*;

/// **Review the resolutions of the conflicts** in Quick search.
pub const COMMAND: &str = "Review the resolutions of the conflicts";
/// The name of the page, before the name of the repository folder.
pub const PAGE: &str = "Conflicts";

/// The title of the page in the trail: **Conflicts · \<repository folder name\>**.
#[must_use]
pub fn title(folder: &str) -> String {
    format!("{PAGE} \u{b7} {folder}")
}

/// The head of the page: **Conflicts of the merge** (of the rebase, …).
#[must_use]
pub fn head(operation: Option<Operation>) -> &'static str {
    match operation {
        Some(Operation::Merge) => "Conflicts of the merge",
        Some(Operation::Rebase) => "Conflicts of the rebase",
        Some(Operation::CherryPick) => "Conflicts of the cherry-pick",
        Some(Operation::Revert) => "Conflicts of the revert",
        Some(Operation::Patch) => "Conflicts of the patch",
        None => "Conflicts of the changes applied",
    }
}

/// The operation in words, after **Continue the** and in **the merge can continue**.
#[must_use]
pub fn operation_name(operation: Operation) -> &'static str {
    match operation {
        Operation::Merge => "merge",
        Operation::Rebase => "rebase",
        Operation::CherryPick => "cherry-pick",
        Operation::Revert => "revert",
        Operation::Patch => "patch",
    }
}

/// `1 conflict`, `5 conflicts`, and so on for any noun whose plural takes an `s`.
#[must_use]
pub fn counted(count: usize, noun: &str) -> String {
    if count == 1 {
        format!("1 {noun}")
    } else {
        format!("{count} {noun}s")
    }
}

/// What the page counts, for the sentence under its head.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Summary {
    /// The conflicts.
    pub conflicts: usize,
    /// The files they are in.
    pub files: usize,
    /// Those the rules propose.
    pub by_rules: usize,
    /// Those the AI proposes.
    pub by_ai: usize,
    /// Those chosen by the user.
    pub chosen: usize,
    /// Those with nothing proposed.
    pub nothing: usize,
    /// The files deleted on one side and changed on the other.
    pub deleted: usize,
}

/// *5 conflicts in 2 files: 3 proposed by the rules, 1 by the AI, 1 with nothing proposed; 1 file deleted on one side
/// and changed on the other. Nothing is written until you write what you accepted.*
#[must_use]
pub fn summary(summary: &Summary) -> String {
    let mut parts = Vec::new();
    if summary.by_rules > 0 {
        parts.push(format!("{} proposed by the rules", summary.by_rules));
    }
    if summary.by_ai > 0 {
        parts.push(format!("{} by the AI", summary.by_ai));
    }
    if summary.chosen > 0 {
        parts.push(format!("{} chosen by you", summary.chosen));
    }
    if summary.nothing > 0 {
        parts.push(format!("{} with nothing proposed", summary.nothing));
    }
    let mut said = format!(
        "{} in {}",
        counted(summary.conflicts, "conflict"),
        counted(summary.files, "file")
    );
    if !parts.is_empty() {
        said.push_str(": ");
        said.push_str(&parts.join(", "));
    }
    if summary.deleted > 0 {
        said.push_str("; ");
        said.push_str(&counted(summary.deleted, "file"));
        said.push_str(" deleted on one side and changed on the other");
    }
    said.push_str(". Nothing is written until you write what you accepted.");
    said
}

/// What the head says when neither the rules nor the AI may propose.
pub const NOTHING_SWITCHED_ON: &str =
    "Neither the rules nor the AI are switched on in the settings: nothing is proposed.";
/// Why the page does not open with both settings off.
pub const SWITCHED_OFF: &str =
    "Resolving conflicts automatically is experimental, and it is switched off in the settings.";
/// The button that opens the settings, beside [`SWITCHED_OFF`].
pub const OPEN_SETTINGS: &str = "Open settings";

/// **1 proposal has a warning: look at it and accept one by one.**
#[must_use]
pub fn with_warnings(count: usize) -> String {
    if count == 1 {
        String::from("1 proposal has a warning: look at it and accept one by one.")
    } else {
        format!("{count} proposals have a warning: look at them and accept one by one.")
    }
}

/// While the files are read.
pub const READING: &str = "Reading the files in conflict\u{2026}";
/// When they could not be read.
pub const FAILED: &str = "Git could not read the files in conflict.";
/// When no file is in conflict.
pub const NO_CONFLICT: &str = "No file is in conflict.";

/// **Accept all proposed** (`review.accept-all`).
pub const ACCEPT_ALL: &str = "Accept all proposed";
/// Its tooltip.
pub const ACCEPT_ALL_TOOLTIP: &str = "Accept every resolution proposed, from the rules and from the AI";
/// **Clear** (`review.clear`).
pub const CLEAR: &str = "Clear";
/// Its tooltip.
pub const CLEAR_TOOLTIP: &str = "Take back every acceptance";
/// **Ask the AI about the rest (N)**, with `conflicts.ai`.
#[must_use]
pub fn ask_the_rest(count: usize) -> String {
    format!("Ask the AI about the rest ({count})")
}
/// **Cards** (`review.columns`).
pub const CARDS: &str = "Cards";
/// Its tooltip.
pub const CARDS_TOOLTIP: &str = "Show the two sides of each conflict over what is proposed, with the lines around it";
/// **Columns** (`review.columns`).
pub const COLUMNS: &str = "Columns";
/// Its tooltip.
pub const COLUMNS_TOOLTIP: &str = "Show each conflict in three columns, as the Merge page does: the current side, what is proposed and the incoming side, joined by bands";
/// **Show the result** (`review.show-result`).
pub const SHOW_RESULT: &str = "Show the result";
/// Its tooltip.
pub const SHOW_RESULT_TOOLTIP: &str = "Show each file beside what the accepted resolutions make of it";
/// The title of the Diff page of the result.
pub const RESULT_PAGE: &str = "Conflicts resolved as accepted";
/// The name of the column of the result in that page.
pub const RESULT: &str = "Resolved as accepted";

/// **Write N resolutions** (`review.write`), or **Write the accepted** while nothing is.
#[must_use]
pub fn write(count: usize) -> String {
    match count {
        0 => String::from("Write the accepted"),
        1 => String::from("Write 1 resolution"),
        count => format!("Write {count} resolutions"),
    }
}
/// Its tooltip.
pub const WRITE_TOOLTIP: &str =
    "Write the accepted resolutions in the files, and stage the files left with no conflict";

/// The filters over the file list (`review.file-list`).
pub const ALL: &str = "All";
/// The tooltip of **All**.
pub const ALL_TOOLTIP: &str = "Show every conflict";
/// The filter of the conflicts not accepted.
pub const NOT_ACCEPTED: &str = "Not accepted";
/// The filter of the conflicts with nothing proposed.
pub const NOTHING_PROPOSED: &str = "Nothing proposed";
/// The filter of the proposals of the AI.
pub const FROM_THE_AI: &str = "From the AI";
/// A filter with its count: **All 4**.
#[must_use]
pub fn filter(name: &str, count: usize) -> String {
    format!("{name} {count}")
}
/// The entry that shows every file again.
pub const ALL_FILES: &str = "All files";
/// The tooltip of the tick of a file whose conflicts are all accepted.
pub const ALL_ACCEPTED: &str = "Every conflict of it is accepted";
/// The tooltip of the yellow dot of a file with conflicts that have nothing proposed.
pub const SOME_UNPROPOSED: &str = "Some of its conflicts have nothing proposed";
/// How many conflicts of a file are accepted: **2 of 3**.
#[must_use]
pub fn accepted_of(accepted: usize, total: usize) -> String {
    format!("{accepted} of {total}")
}
/// The group of the files deleted on one side.
pub const DELETED_ON_ONE_SIDE: &str = "Deleted on one side";
/// The group of the other files in conflict.
pub const ALSO_IN_CONFLICT: &str = "Also in conflict";
/// What is said under it.
pub const ALSO_NOTE: &str =
    "Binary files, very large files, and files both sides added are resolved in an editor, or by taking one side.";
/// When a filter leaves nothing.
pub const NOTHING_TO_SHOW: &str = "No file has a conflict to show here.";
/// The name of the file list for the screen reader.
pub const FILE_LIST: &str = "Files in conflict";

/// The heading of the middle column.
pub const PROPOSED: &str = "Proposed";
/// **Accept the file** (`review.accept-file`).
pub const ACCEPT_FILE: &str = "Accept the file";
/// Its tooltip.
pub const ACCEPT_FILE_TOOLTIP: &str = "Accept every resolution proposed in this file";
/// The arrow that folds a conflict or a file.
pub const FOLD: &str = "Fold";
/// The arrow that unfolds it.
pub const UNFOLD: &str = "Unfold";

/// **Conflict at line N**.
#[must_use]
pub fn conflict_at(line: usize) -> String {
    format!("Conflict at line {line}")
}
/// The tooltip of the line number of a card.
pub const OPEN_AT_LINE: &str = "Open the file at this line";

/// The name of a side in a sentence: its branch or commit, else what it is.
#[must_use]
pub fn side_name(side: &Side) -> String {
    side.name.clone().unwrap_or_else(|| side.role.words().to_lowercase())
}

/// What the heading of a card says of a proposal, from its shape and its counts; **Nothing proposed yet** without one.
#[must_use]
pub fn heading(shape: Option<Shape>, counts: &OriginCounts, (current, incoming): (&str, &str)) -> String {
    let Some(shape) = shape else {
        return String::from("Nothing proposed yet");
    };
    let own = |count: usize| {
        if count == 1 {
            String::from("1 line")
        } else {
            format!("{count} lines")
        }
    };
    match shape {
        Shape::Nothing => String::from("Leaves out the lines of both sides"),
        Shape::Current => format!("Takes only the current side, {current}"),
        Shape::Incoming => format!("Takes only the incoming side, {incoming}"),
        Shape::CurrentFirst => format!("Keeps both sides: {current} first, then {incoming}"),
        Shape::IncomingFirst => format!("Keeps both sides: {incoming} first, then {current}"),
        Shape::Mixed if counts.current + counts.incoming + counts.both + counts.base == 0 => {
            format!("Writes {} of its own in place of both sides", own(counts.new))
        }
        Shape::Mixed if counts.new > 0 => {
            format!("Takes lines of both sides, and writes {} of its own", own(counts.new))
        }
        Shape::Mixed => String::from("Takes lines of both sides"),
    }
}

/// The counts under the heading: **4 from main · 1 on both sides · 4 from feature/map-view · 0 new**, a count of 0
/// left out but for the new lines of a proposal of the AI.
#[must_use]
pub fn origin_counts(counts: &OriginCounts, (current, incoming): (&str, &str), is_from_ai: bool) -> String {
    let mut parts = Vec::new();
    if counts.current > 0 {
        parts.push(format!("{} from {current}", counts.current));
    }
    if counts.both > 0 {
        parts.push(format!("{} on both sides", counts.both));
    }
    if counts.incoming > 0 {
        parts.push(format!("{} from {incoming}", counts.incoming));
    }
    if counts.base > 0 {
        parts.push(format!("{} as before", counts.base));
    }
    if counts.new > 0 || is_from_ai {
        parts.push(format!("{} new", counts.new));
    }
    parts.join(" \u{b7} ")
}

/// What the pointer on a line from the current side says: **From main**.
#[must_use]
pub fn from_side(name: &str) -> String {
    format!("From {name}")
}
/// What the pointer on a line of both sides says.
pub const ON_BOTH_SIDES: &str = "On both sides";
/// What the pointer on a line of the base says.
pub const AS_BEFORE: &str = "As it was before either side changed it";
/// What the pointer on a line neither side has says.
pub const NEW_LINE: &str = "New: neither side has this line";

/// A warning in words, at the top of a proposal (`review.proposal-warnings`).
#[must_use]
pub fn warning(warning: &ProposalWarning) -> String {
    match warning {
        ProposalWarning::SetTwice { name, .. } => format!("{name} is set twice"),
        ProposalWarning::NotJson => String::from("Not valid JSON"),
    }
}
/// The warnings of a proposal in one line: the one there is, or **2 warnings**.
#[must_use]
pub fn warnings(warnings: &[ProposalWarning]) -> Option<String> {
    match warnings {
        [] => None,
        [only] => Some(warning(only)),
        many => Some(counted(many.len(), "warning")),
    }
}

/// Before the choices, when something is proposed (`review.choose`).
pub const INSTEAD: &str = "Instead:";
/// Before the choices, when nothing is.
pub const CHOOSE: &str = "Choose:";
/// The button of a choice.
#[must_use]
pub fn choice(kind: ChoiceKind) -> &'static str {
    match kind {
        ChoiceKind::CurrentOnly => "Current only",
        ChoiceKind::IncomingOnly => "Incoming only",
        ChoiceKind::BothCurrentFirst => "Both, current first",
        ChoiceKind::BothIncomingFirst => "Both, incoming first",
        ChoiceKind::Written => "Edit",
    }
}
/// The reason of a choice, beside **Your choice**.
#[must_use]
pub fn choice_reason(kind: ChoiceKind) -> &'static str {
    match kind {
        ChoiceKind::CurrentOnly => "Only the current side",
        ChoiceKind::IncomingOnly => "Only the incoming side",
        ChoiceKind::BothCurrentFirst => "Both sides, the current one first",
        ChoiceKind::BothIncomingFirst => "Both sides, the incoming one first",
        ChoiceKind::Written => "Written by you",
    }
}
/// The button that drops a choice.
pub const DROP_CHOICE: &str = "Drop your choice";
/// The button that takes the lines written.
pub const USE_LINES: &str = "Use these lines";
/// The button that leaves them.
pub const CANCEL: &str = "Cancel";
/// The box the lines are written in.
pub const LINES_BOX: &str = "The lines in place of the conflict";

/// **Compare with the sides**.
pub const COMPARE_SIDES: &str = "Compare with the sides";
/// **Hide the sides**.
pub const HIDE_SIDES: &str = "Hide the sides";
/// **Show the base**.
pub const SHOW_BASE: &str = "Show the base";
/// Its tooltip.
pub const SHOW_BASE_TOOLTIP: &str = "Show what the lines were before either side changed them";
/// The heading of the base.
pub const BASE: &str = "Base \u{b7} before either side changed it";
/// The heading of a side: **Current · main**, with the subject of its commit after it.
#[must_use]
pub fn side_heading(is_current: bool, side: &Side) -> String {
    let role = if is_current { "Current" } else { "Incoming" };
    let mut said = match &side.name {
        Some(name) => format!("{role} \u{b7} {name}"),
        None => format!("{role} \u{b7} {}", side.role.words()),
    };
    if let Some(subject) = &side.subject {
        said.push_str(" \u{2014} ");
        said.push_str(subject);
    }
    said
}

/// Where a proposal comes from: **Rules**.
pub const RULES: &str = "Rules";
/// **AI**.
pub const AI: &str = "AI";
/// **Your choice**.
pub const YOUR_CHOICE: &str = "Your choice";
/// The reason of a proposal of the rules: its case.
#[must_use]
pub fn rule_case(case: RuleCase) -> &'static str {
    case.words()
}
/// How sure the AI said it is.
#[must_use]
pub fn confidence(confidence: Confidence) -> &'static str {
    match confidence {
        Confidence::High => "high",
        Confidence::Medium => "medium",
        Confidence::Low => "low",
    }
}
/// What is said of the lines of the AI neither side has.
pub const WRITTEN_BY_MODEL: &str = "Written by the language model: look at it before you accept it";
/// A proposal of nothing.
pub const DELETES_LINES: &str = "Nothing: the lines are deleted";
/// A card with nothing proposed.
pub const NOTHING_IS_PROPOSED: &str =
    "Nothing is proposed: the two sides changed the same lines in different ways. Choose a side, or write the lines.";

/// **Resolve by hand**.
pub const RESOLVE_BY_HAND: &str = "Resolve by hand";
/// Its tooltip.
pub const RESOLVE_BY_HAND_TOOLTIP: &str = "Resolve it in the Merge page, choosing for each change";
/// **Accept** (`review.accept`).
pub const ACCEPT: &str = "Accept";
/// Its tooltip.
pub const ACCEPT_TOOLTIP: &str = "Put this in place of the conflict when the accepted resolutions are written";
/// **Accepted**, which takes it back.
pub const ACCEPTED: &str = "Accepted";
