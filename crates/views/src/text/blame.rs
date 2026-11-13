//! The words of the blame in the File page (`blame.current-line`, `blame.file`) and of the history of a file or of some
//! lines asked from it (`file-history.file`, `file-history.lines`).

/// The button with a person at the top right of the File page (`blame.toggle-file`).
pub const SHOW_FILE_BLAME: &str = "Show file blame";
/// The closed eye that takes its place while the file shows its blame.
pub const HIDE_FILE_BLAME: &str = "Hide file blame";
/// The funnel while the setting is off (`blame.ignore-whitespace`).
pub const IGNORE_WHITESPACE: &str = "Ignore whitespace and moved lines in blame";
/// The funnel while the setting is on.
pub const STOP_IGNORING_WHITESPACE: &str = "Stop ignoring whitespace and moved lines in blame";
/// The pin while the setting is off (`blame.every-file`).
pub const KEEP_IN_EVERY_FILE: &str = "Keep file blame on in every file";
/// The pin while the setting is on.
pub const STOP_KEEPING_IN_EVERY_FILE: &str = "Stop keeping file blame on in every file";
/// **Toggle file blame** in the context menu of the File page.
pub const TOGGLE_FILE_BLAME: &str = "Toggle file blame";
/// The command of Quick search that turns the blame of the line under the cursor on or off.
pub const TOGGLE_INLINE_BLAME: &str = "Toggle inline blame";
/// The command of Quick search that turns ignoring whitespace and moved lines on or off.
pub const TOGGLE_IGNORE_WHITESPACE: &str = "Toggle ignoring whitespace and moved lines in blame";
/// The command of Quick search that turns the blame in every file on or off.
pub const TOGGLE_EVERY_FILE: &str = "Toggle file blame in every file";
/// **Open commit of current line as a page** in the context menu (`blame.open-line-commit`).
pub const OPEN_LINE_COMMIT: &str = "Open commit of current line as a page";
/// The keys of [`OPEN_LINE_COMMIT`].
pub const OPEN_LINE_COMMIT_KEYS: &str = if cfg!(target_os = "macos") {
    "Ctrl+Alt+B"
} else {
    "Alt+B"
};
/// **Reveal commit of current line in the timeline** in the context menu (`blame.reveal-line-commit`).
pub const REVEAL_LINE_COMMIT: &str = "Reveal commit of current line in the timeline";
/// The keys of [`REVEAL_LINE_COMMIT`].
pub const REVEAL_LINE_COMMIT_KEYS: &str = if cfg!(target_os = "macos") {
    "Ctrl+Shift+Alt+B"
} else {
    "Shift+Alt+B"
};
/// **Blame file before commit of current line** in the context menu (`blame.previous`).
pub const BLAME_BEFORE: &str = "Blame file before commit of current line";
/// **Show file history** in the context menu (`file-history.file`).
pub const SHOW_FILE_HISTORY: &str = "Show file history";
/// **Show history of selected lines** in the context menu (`file-history.lines`).
pub const SHOW_LINES_HISTORY: &str = "Show history of selected lines";
/// The link of the hover that opens the commit as a page.
pub const OPEN_AS_PAGE: &str = "Open it as a page";
/// The link of the hover that blames the file as it was before the commit (`blame.previous`).
pub const BLAME_FILE_BEFORE: &str = "Blame the file as it was before";
/// The name of the hover, for the screen reader.
pub const HOVER: &str = "Blame of the line";
/// The name of the column of the blame, for the screen reader.
pub const COLUMN: &str = "Blame";

/// The link of the hover that shows the commit in the timeline: `Show 4b830a6 in the timeline`.
#[must_use]
pub fn show_in_timeline(short: &str) -> String {
    format!("Show {short} in the timeline")
}

/// What the end of the line under the cursor says of a committed line: `Ann Lee, 3 days ago • Fix the parser`.
#[must_use]
pub fn line_note(author: &str, ago: &str, summary: &str) -> String {
    format!("{author}, {ago} \u{2022} {summary}")
}
