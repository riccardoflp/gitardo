//! The words of the Diff page (`diff.page`, `diff.navigate`, `diff.collapse-unchanged`), of the buttons of its gutter
//! (`diff.stage-change`, `diff.unstage-change`, `diff.revert-change`) and of its title buttons (`diff.stage-file`,
//! `diff.unstage-file`, `diff.discard-file`).

/// The page, for the screen reader.
pub const PAGE: &str = "Diff";
/// The lines of the page, for the screen reader.
pub const LINES: &str = "Lines of the diff";
/// The side of the working tree.
pub const WORKING_TREE: &str = "Working tree";
/// The side of the index.
pub const INDEX: &str = "Index";
/// The side of what is checked out.
pub const HEAD: &str = "HEAD";
/// A side where the file is not.
pub const EMPTY: &str = "Empty";
/// The name over the column of the common ancestor of a file in conflict (`conflicts.open-file`).
pub const BASE: &str = "Base";
/// The arrow that goes to the previous change.
pub const PREVIOUS_CHANGE: &str = "Previous change (Shift+F7)";
/// The arrow that goes to the next change.
pub const NEXT_CHANGE: &str = "Next change (F7)";
/// What the bar says of a diff with no change.
pub const NO_CHANGES: &str = "No changes";
/// The fold button of the bar.
pub const COLLAPSE_UNCHANGED: &str = "Collapse unchanged regions";
/// The tooltip of the bar of a fold.
pub const CLICK_TO_EXPAND: &str = "Click to expand";
/// The hint after `F7` on the last change.
pub const NEXT_FILE_HINT: &str = "Press F7 again to go to the next file";
/// The hint after `Shift+F7` on the first change.
pub const PREVIOUS_FILE_HINT: &str = "Press Shift+F7 again to go to the previous file";
/// What `F7` says when no other file is changed.
pub const NO_OTHER_FILES: &str = "No other changed files.";
/// What a diff of a binary file says, and why the page does not open.
pub const LOOKS_BINARY: &str = "This file looks binary.";
/// What a diff too long to compute says, and why the page does not open.
pub const TOO_MANY_DIFFERENCES: &str = "This file has too many differences to display.";
/// What the line over the two pictures of an image says when they decode to the same pixels (`diff.image`).
pub const SAME_PIXELS: &str = "Only the bytes of the file changed; the pixels are the same.";
/// The title button that shows the pictures of an SVG.
pub const IMAGE: &str = "Image";
/// What **Image** does.
pub const IMAGE_TOOLTIP: &str = "Show the pictures of the two versions";
/// The title button that shows the diff of the lines of an SVG.
pub const TEXT: &str = "Text";
/// What **Text** does.
pub const TEXT_TOOLTIP: &str = "Show the diff of the lines of the file";

/// What the line over the two pictures of an image says: **640 × 480 → 1280 × 960 · 24 KB → 81 KB**.
#[must_use]
pub fn image_change((size_before, size_after): (&str, &str), (weight_before, weight_after): (&str, &str)) -> String {
    format!("{size_before} \u{2192} {size_after} \u{b7} {weight_before} \u{2192} {weight_after}")
}

/// What a diff git could not read says.
pub const COULD_NOT_READ: &str = "Git could not read this file.";
/// What a diff being read says.
pub const LOADING: &str = "Loading\u{2026}";
/// What a file with no change says.
pub const NO_CHANGE: &str = "No changes.";
/// The question asked before a very large diff is computed.
pub const LARGE_QUESTION: &str = "This file is very large and the diff may be slow. Continue?";
/// The button that computes it.
pub const CONTINUE: &str = "Continue";
/// The button that opens nothing.
pub const CANCEL: &str = "Cancel";
/// The tooltip of a change already in the index.
pub const STAGED: &str = "Staged";
/// The **+** button of a change in the gutter.
pub const STAGE_CHANGE: &str = "Stage this change";
/// The **−** button of a change in the gutter.
pub const UNSTAGE_CHANGE: &str = "Unstage this change";
/// The **−** button of a change of the working tree staged already.
pub const STAGED_UNSTAGE_CHANGE: &str = "Staged \u{2014} unstage this change";
/// The **⟲** button of a change in the gutter.
pub const REVERT_CHANGE: &str = "Revert this change";
/// The title button that stages the file.
pub const STAGE_FILE: &str = "Stage file";
/// The title button that opens the file in the File page (`diff.open-file`).
pub const OPEN_FILE: &str = "Open file";
/// The title button that unstages the file.
pub const UNSTAGE_FILE: &str = "Unstage file";
/// The title button that discards the changes of the file.
pub const DISCARD_FILE: &str = "Discard changes";
/// The entry of the context menu of a side that opens the file at the line of the caret (`diff.open-at-line`).
pub const JUMP_TO_SOURCE: &str = "Jump to source";
/// The key of **Jump to source**.
pub const JUMP_TO_SOURCE_KEYS: &str = "F4";

/// The title of the Diff page of a file: **app.rs (HEAD ↔ Working tree)**.
#[must_use]
pub fn title(file: &str, left: &str, right: &str) -> String {
    format!("{file} ({left} \u{2194} {right})")
}

/// Where the current change is among them: **3 / 7**, or **– / 7** before one is chosen.
#[must_use]
pub fn position(current: Option<usize>, count: usize) -> String {
    match current {
        Some(current) => format!("{} / {count}", current + 1),
        None => format!("\u{2013} / {count}"),
    }
}

/// The bar of a fold: **⋯ 12 unchanged lines**.
#[must_use]
pub fn unchanged_lines(count: usize) -> String {
    if count == 1 {
        String::from("\u{22ef} 1 unchanged line")
    } else {
        format!("\u{22ef} {count} unchanged lines")
    }
}

/// The path of a renamed file: `old/path → new/path`.
#[must_use]
pub fn renamed(old: &str, new: &str) -> String {
    format!("{old} \u{2192} {new}")
}
