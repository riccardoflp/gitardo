//! The words of the File page (`file-view.page`), of its editor (`file-view.edit`), of the box that finds in it
//! (`file-view.find`) and of **Go to line** (`file-view.go-to-line`).

/// The page, for the screen reader.
pub const PAGE: &str = "File";
/// The dot in the trail of a file with edits not saved.
pub const UNSAVED_DOT: &str = "\u{25cf}";
/// What a page says while its file is read.
pub const READING: &str = "Reading the file\u{2026}";
/// What a page says of a file with a NUL byte.
pub const NOT_TEXT: &str = "This file is not text.";
/// The title button that opens the file in another editor.
pub const OPEN_EXTERNAL: &str = "Open in external editor";
/// The entries of the context menu of the editor.
pub const CUT: &str = "Cut";
/// Copy.
pub const COPY: &str = "Copy";
/// Paste.
pub const PASTE: &str = "Paste";
/// Find.
pub const FIND: &str = "Find";
/// Go to line, in the menu and as the title of its dialog.
pub const GO_TO_LINE: &str = "Go to line";
/// The keys of the entries of the menu.
pub const CUT_KEYS: &str = "Ctrl+X";
/// The keys of Copy.
pub const COPY_KEYS: &str = "Ctrl+C";
/// The keys of Paste.
pub const PASTE_KEYS: &str = "Ctrl+V";
/// The keys of Find.
pub const FIND_KEYS: &str = "Ctrl+F";
/// The keys of Go to line.
pub const GO_TO_LINE_KEYS: &str = "Ctrl+G";

/// The title button that shows a Markdown file laid out as it reads (`file-view.markdown-preview`).
pub const PREVIEW: &str = "Preview";
/// What **Preview** does.
pub const PREVIEW_TOOLTIP: &str = "Show the file laid out as it reads";
/// The title button that shows the text of a Markdown file, to edit it.
pub const SOURCE: &str = "Source";
/// What **Source** does.
pub const SOURCE_TOOLTIP: &str = "Show the text of the file";
/// The preview of a Markdown file, for the screen reader.
pub const PREVIEW_REGION: &str = "Preview";
/// What an image of the preview is called for the screen reader when its text says nothing.
pub const IMAGE: &str = "Image";
/// What a diagram of Mermaid is called for the screen reader when it has no title (`file-view.mermaid`).
pub const DIAGRAM: &str = "Mermaid diagram";
/// What the preview says under the code of a diagram of a kind it does not draw.
pub const DIAGRAM_UNSUPPORTED: &str = "Gitardo does not draw this kind of diagram.";
/// What the preview says under the code of a diagram too large to draw.
pub const DIAGRAM_TOO_LARGE: &str = "This diagram is too large to draw.";

/// What the preview says under the code of a diagram with an error: **This diagram has an error: Unterminated node
/// label.**
#[must_use]
pub fn diagram_error(reason: &str) -> String {
    format!("This diagram has an error: {reason}.")
}

/// The box that finds, empty, and its name for the screen reader.
pub const FIND_PLACEHOLDER: &str = "Find in the file";
/// What the box says when nothing is found.
pub const NO_RESULTS: &str = "No results";
/// What the box says of a regular expression that is not valid.
pub const INVALID_PATTERN: &str = "Not a valid regular expression";
/// The switch of match case.
pub const MATCH_CASE: &str = "Match case";
/// The switch of whole word.
pub const WHOLE_WORD: &str = "Whole word";
/// The switch of regular expressions.
pub const REGULAR_EXPRESSION: &str = "Regular expression";
/// The arrow that goes to the previous match.
pub const PREVIOUS_MATCH: &str = "Previous match (Shift+F3)";
/// The arrow that goes to the next match.
pub const NEXT_MATCH: &str = "Next match (F3)";
/// The button that closes the box.
pub const CLOSE_FIND: &str = "Close (Escape)";

/// What **Go to line** asks.
pub const GO: &str = "Go";
/// The button that closes a dialog doing nothing.
pub const CANCEL: &str = "Cancel";

/// The question asked before closing a file with edits not saved: **Save changes to app.rs?**
#[must_use]
pub fn save_changes(name: &str) -> String {
    format!("Save changes to {name}?")
}
/// Save.
pub const SAVE: &str = "Save";
/// Don't save.
pub const DONT_SAVE: &str = "Don't save";

/// Where a match is among them: **3 of 12**.
#[must_use]
pub fn match_place(current: usize, count: usize) -> String {
    format!("{current} of {count}")
}

/// What **Go to line** says of what is typed that is not a line of the file.
#[must_use]
pub fn line_between(count: usize) -> String {
    format!("Type a line number from 1 to {count}.")
}

/// The title of a file at a revision: **app.rs (3f2a1c9)**.
#[must_use]
pub fn at_revision(name: &str, short: &str) -> String {
    format!("{name} ({short})")
}
