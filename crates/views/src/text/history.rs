//! The words of the operations that change the commits of a branch or bring commits onto it, in the menus of the
//! timeline, and of a selection of several commits (`merge.`, `cherry-pick.`, `revert.`, `reset.`, `message.`,
//! `selection.`).

/// **Edit message…** in the menu of a commit (`message.edit`).
pub const EDIT_MESSAGE: &str = "Edit message\u{2026}";
/// What the bar of a selection says, dimmed, after how many are selected (`selection.bar`).
pub const SELECTION_HINT: &str = "Ctrl+click adds or removes one, Shift+click selects up to there";
/// **Clear selection** in the bar of a selection.
pub const CLEAR_SELECTION: &str = "Clear selection";
/// The tooltip of **Cherry-pick onto …** in the bar of a selection.
pub const CHERRY_PICK_SELECTION_TOOLTIP: &str = "Applies them oldest first, as one operation";
/// **Copy hashes** in the menu of a selection.
pub const COPY_HASHES: &str = "Copy hashes";
/// **Compare these two commits** in the menu of a selection of two.
pub const COMPARE_TWO: &str = "Compare these two commits";
/// The name of the bar of a selection, for the screen reader.
pub const SELECTION_BAR: &str = "Selection";

/// **Merge into `<current>`** in the menu of a commit, a branch or a remote branch (`merge.into-current`).
#[must_use]
pub fn merge_into(current: &str) -> String {
    format!("Merge into {current}")
}

/// **Merge into `<current>`…**, which asks how (`merge.choose-way`).
#[must_use]
pub fn merge_into_choosing(current: &str) -> String {
    format!("Merge into {current}\u{2026}")
}

/// **Cherry-pick `<short hash>`** in the menu of a commit (`cherry-pick.commit`).
#[must_use]
pub fn cherry_pick(short: &str) -> String {
    format!("Cherry-pick {short}")
}

/// **Revert `<short hash>`** in the menu of a commit (`revert.commit`).
#[must_use]
pub fn revert(short: &str) -> String {
    format!("Revert {short}")
}

/// **Reset `<current>` to here…** in the menu of a commit (`reset.to-commit`).
#[must_use]
pub fn reset_to_here(current: &str) -> String {
    format!("Reset {current} to here\u{2026}")
}

/// **N commits selected** in the bar of a selection.
#[must_use]
pub fn commits_selected(count: usize) -> String {
    format!("{count} commits selected")
}

/// **Cherry-pick onto `<current>`** in the bar of a selection (`selection.cherry-pick`).
#[must_use]
pub fn cherry_pick_onto(current: &str) -> String {
    format!("Cherry-pick onto {current}")
}

/// **Cherry-pick N commits onto `<current>`** in the menu of a selection.
#[must_use]
pub fn cherry_pick_commits_onto(count: usize, current: &str) -> String {
    format!("Cherry-pick {count} commits onto {current}")
}
