//! The words of comparisons: the entries that start one, the comparison page and the comparison in the inspector
//! (`compare.`).

/// **Compare with…** in the menu of a commit, a branch, a remote branch or a tag (`compare.with`).
pub const COMPARE_WITH: &str = "Compare with\u{2026}";
/// **Compare branches, tags or commits…** in the **…** menu of the header (`compare.choose`).
pub const COMPARE_BRANCHES: &str = "Compare branches, tags or commits\u{2026}";

/// **Compare with `<current branch>`**, or **Compare with HEAD** (`compare.with-current`).
#[must_use]
pub fn compare_with_current(name: &str) -> String {
    format!("Compare with {name}")
}

/// **Compare with `<short hash>`, selected before** (`compare.selected-before`).
#[must_use]
pub fn compare_with_selected(short: &str) -> String {
    format!("Compare with {short}, selected before")
}
