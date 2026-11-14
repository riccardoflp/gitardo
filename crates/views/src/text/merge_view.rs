//! The words of the Merge page (`docs/features/17-diff-and-merge-views.md`).

use std::fmt::Write as _;

use model::merge_view::{ProposedBy, RuleReason};

/// The name of the page.
pub const PAGE: &str = "Merge";
/// The name of the rows of the page for the screen reader.
pub const LINES: &str = "Lines of the merge";
/// The word before what the current side is.
pub const CURRENT: &str = "Current";
/// The word before what the incoming side is.
pub const INCOMING: &str = "Incoming";
/// The word after the path of the result.
pub const RESULT: &str = "Result";
/// The label of the column of the base.
pub const BASE: &str = "Base \u{2014} What the two sides started from";
/// The button that shows the base.
pub const SHOW_BASE: &str = "Show the base";
/// The button that hides the base.
pub const HIDE_BASE: &str = "Hide the base";
/// **→** on the current side.
pub const ACCEPT_CURRENT: &str = "Accept current";
/// **+** on the current side.
pub const ACCEPT_CURRENT_FIRST: &str = "Accept both, current first";
/// **←** on the incoming side.
pub const ACCEPT_INCOMING: &str = "Accept incoming";
/// **+** on the incoming side.
pub const ACCEPT_INCOMING_FIRST: &str = "Accept both, incoming first";
/// **✦** on the band of a conflict.
pub const ACCEPT_PROPOSAL: &str = "Accept the proposal";
/// The tooltip of the button with two ticks when no region would change.
pub const NO_SIMPLE: &str = "No changes to accept without a choice";
/// The tooltip of the sparkle when nothing is proposed for a conflict still marked.
pub const NOTHING_PROPOSED: &str = "Nothing is proposed for the conflicts left";
/// The sparkle, for the screen reader.
pub const ACCEPT_PROPOSALS: &str = "Accept the proposals";
/// The button at the bottom of the result.
pub const MARK_RESOLVED: &str = "Mark as resolved";
/// Its tooltip.
pub const MARK_RESOLVED_TOOLTIP: &str = "Save the file and stage it, which tells git that its conflicts are resolved";
/// What the bar says once no conflict is marked.
pub const NO_CONFLICTS_LEFT: &str = "No conflicts left";
/// What the bar says when the file had no conflict.
pub const NO_CONFLICTS: &str = "No conflicts";
/// What a file in conflict with no version on one side says, and why the page does not open.
pub const NOT_IN_CONFLICT: &str = "This file is not in conflict with a change on each side.";
/// The question asked before a very large file is shown.
pub const LARGE_QUESTION: &str = "This file is very large and the merge may be slow to show. Continue?";
/// What the page says while it is read.
pub const LOADING: &str = "Loading\u{2026}";

/// The title of the page in its trail: **app.rs (Current ↔ Incoming)**.
#[must_use]
pub fn title(file: &str) -> String {
    format!("{file} ({CURRENT} \u{2194} {INCOMING})")
}

/// The label of a side's column: **Current — The branch checked out**.
#[must_use]
pub fn side_label(side: &str, role: &str) -> String {
    format!("{side} \u{2014} {role}")
}

/// The label of the result's column: **src/app.rs — Result**.
#[must_use]
pub fn result_label(path: &str) -> String {
    format!("{path} \u{2014} {RESULT}")
}

/// How many conflicts are left: **2 conflicts left**, **1 conflict left**.
#[must_use]
pub fn conflicts_left(count: usize) -> String {
    if count == 1 {
        String::from("1 conflict left")
    } else {
        format!("{count} conflicts left")
    }
}

/// The tooltip of the button with two ticks: **Accept the 3 changes that need no choice: of one side, or the same on
/// both**.
#[must_use]
pub fn accept_simple(count: usize) -> String {
    let changes = if count == 1 {
        String::from("the change")
    } else {
        format!("the {count} changes")
    };
    format!("Accept {changes} that need no choice: of one side, or the same on both")
}

/// The entry of the context menu of a line that opens the file at the line of the result it stands for
/// (`merge-view.open-at-line`).
pub const JUMP_TO_SOURCE: &str = "Jump to source";

/// The tooltip of the sparkle, from who proposes for each conflict still marked: **Accept what is proposed for the 2
/// conflicts, by the rules**, **…, by the AI**, or **…, 1 of them by the AI**.
#[must_use]
pub fn accept_proposals(proposed: &[&ProposedBy]) -> String {
    let count = proposed.len();
    let by_ai = proposed.iter().filter(|by| matches!(by, ProposedBy::Ai { .. })).count();
    let conflicts = if count == 1 {
        String::from("the conflict")
    } else {
        format!("the {count} conflicts")
    };
    let by = match by_ai {
        0 => String::from("by the rules"),
        _ if by_ai == count => String::from("by the AI"),
        _ => format!("{by_ai} of them by the AI"),
    };
    format!("Accept what is proposed for {conflicts}, {by}")
}

/// The tooltip of **✦**: **Accept the proposal of the rules — Changes to lines next to each other**, or **Accept the
/// proposal of \<model\>, with high confidence — \<reason\> One line is new: neither side has it.**
#[must_use]
pub fn proposal_tooltip(by: &ProposedBy) -> String {
    match by {
        ProposedBy::Rules(reason) => format!("Accept the proposal of the rules \u{2014} {}", reason_words(*reason)),
        ProposedBy::Ai {
            model,
            reason,
            confidence,
            new_lines,
        } => {
            let sure = crate::text::review::confidence(*confidence);
            let mut said = format!("Accept the proposal of {model}, with {sure} confidence \u{2014} {reason}");
            match new_lines {
                0 => {}
                1 => said.push_str(" One line is new: neither side has it."),
                _ => {
                    let _ = write!(said, " {new_lines} lines are new: neither side has them.");
                }
            }
            said
        }
    }
}

/// Why the rules could resolve a conflict, as the review page names it.
#[must_use]
pub fn reason_words(reason: RuleReason) -> &'static str {
    match reason {
        RuleReason::SpacesOrLineEndings => "Spaces or line endings only",
        RuleReason::SameChange => "The same change on both sides",
        RuleReason::OneSide => "A change on one side only",
        RuleReason::NextToEachOther => "Changes to lines next to each other",
    }
}
