//! The words of comparisons: the entries that start one, the comparison page and the comparison in the inspector
//! (`compare.`).

use std::fmt::Write as _;

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

/// The name of the comparison page in the trail, and its heading (`compare.page`).
pub const PAGE: &str = "Comparison";
/// What the page says until the first answer.
pub const COMPARING: &str = "Comparing\u{2026}";
/// What the page says when git could not compare the two.
pub const FAILED: &str = "Git could not compare the two.";
/// Beside a side whose name no longer stands for a commit.
pub const GONE: &str = "gone";
/// The arrows that swap the two sides (`compare.swap`).
pub const SWAP: &str = "Swap";
/// Their tooltip.
pub const SWAP_TOOLTIP: &str = "Compare in the other direction";
/// **Since they diverged** (`compare.set-mode`).
pub const SINCE_DIVERGED: &str = "Since they diverged";
/// **Everything that differs**.
pub const EVERYTHING: &str = "Everything that differs";
/// The tooltip of **Since they diverged** when the two have no merge base.
pub const NEVER_MEET: &str = "Their histories never meet";
/// The placeholder of the box that narrows a comparison (`compare.narrow-to-path`).
pub const NARROW_PLACEHOLDER: &str = "Only the changes of this file or folder";
/// The button that shows every file again.
pub const SHOW_EVERY_FILE: &str = "Show the changes of every file";
/// **Open in the Diff page**, which opens every file listed.
pub const OPEN_IN_DIFF: &str = "Open in the Diff page";
/// **Copy as patch** (`compare.copy-patch`).
pub const COPY_AS_PATCH: &str = "Copy as patch";
/// Its tooltip.
pub const COPY_AS_PATCH_TOOLTIP: &str = "Copy the changes as a patch that git apply takes";
/// **Open on GitHub** (`compare.open-on-github`).
pub const OPEN_ON_GITHUB: &str = "Open on GitHub";
/// Its tooltip.
pub const OPEN_ON_GITHUB_TOOLTIP: &str = "Compare the two on GitHub, once they are pushed";
/// **Commits** (`compare.commits`).
pub const COMMITS: &str = "Commits";
/// The heading of the commits when they were too many to be matched.
pub const TOO_MANY: &str = "Too many to look for the same changes on the other side";
/// **Swap** in the inspector (`compare.in-place`).
pub const IN_PLACE_SWAP: &str = "Swap";
/// **Open as a page** in the inspector.
pub const OPEN_AS_PAGE: &str = "Open as a page";
/// Its tooltip.
pub const OPEN_AS_PAGE_TOOLTIP: &str = "How far apart the two are, and every change, on a page";
/// The button that closes the comparison in the inspector.
pub const CLOSE: &str = "Close the comparison";
/// **From** in the inspector.
pub const FROM: &str = "From";
/// **To** in the inspector.
pub const TO: &str = "To";
/// The tooltip of the short hash of a side in the inspector, which shows that commit in the timeline.
pub const SHOW_THIS_COMMIT: &str = "Show this commit";
/// **Loading…** in the inspector.
pub const LOADING: &str = "Loading\u{2026}";
/// The button that opens all the files of the comparison in the inspector in one Diff page.
pub const OPEN_ALL: &str = "Open all changes in one Diff page";

/// The title of a comparison page: `<first> ↔ <second>`, with ` · <last part of the path>` when narrowed.
#[must_use]
pub fn title(first: &str, second: &str, path: Option<&str>) -> String {
    let mut title = format!("{first} \u{2194} {second}");
    if let Some(path) = path {
        let last = path.rsplit('/').next().unwrap_or(path);
        title.push_str(" \u{b7} ");
        title.push_str(last);
    }
    title
}

/// The heading of the commits only in a side: **Only in `<side>`** with their count, **N of M change `<path>`** when
/// narrowed and the count before is known, and **, K also in `<other>`** when some are.
#[must_use]
pub fn only_in(
    side: &str,
    (count, whole): (usize, Option<usize>),
    path: Option<&str>,
    (also, other): (usize, &str),
) -> String {
    let mut heading = match (path, whole) {
        (Some(path), Some(whole)) => format!("Only in {side} \u{b7} {count} of {whole} change {path}"),
        (Some(path), None) => format!("Only in {side} \u{b7} {count} change {path}"),
        (None, _) => format!("Only in {side} \u{b7} {count}"),
    };
    if also > 0 {
        let _ = write!(heading, ", {also} also in {other}");
    }
    heading
}

/// What a commit whose change the other side has says.
#[must_use]
pub fn also_in(other: &str) -> String {
    format!("also in {other}")
}

/// The tooltip of the arrow that takes a file of the second side (`compare.take-file`).
#[must_use]
pub fn take_file(second: &str, is_deleted: bool) -> String {
    if is_deleted {
        format!("Delete it from the working tree, as {second} has no such file")
    } else {
        format!("Take the version of {second} into the working tree")
    }
}

/// **N changed files**.
#[must_use]
pub fn changed_files(count: usize) -> String {
    if count == 1 {
        String::from("1 changed file")
    } else {
        format!("{count} changed files")
    }
}

/// The tooltip of a mode button: the command it stands for, with the names of the sides.
#[must_use]
pub fn mode_tooltip(first: &str, second: &str, is_since: bool) -> String {
    if is_since {
        format!("git diff {first}...{second}")
    } else {
        format!("git diff {first} {second}")
    }
}

/// The tooltip of a commit whose change the other side has.
#[must_use]
pub fn also_in_tooltip(other: &str) -> String {
    format!("{other} has the same change in a commit of its own: it was cherry-picked, or rebased")
}

/// **Show all N commits…** at the end of a group of commits cut at 300.
#[must_use]
pub fn show_all_commits(count: usize) -> String {
    format!("Show all {count} commits\u{2026}")
}

/// **The newest N are listed.**, under a group with more commits than were read.
#[must_use]
pub fn newest_listed(count: usize) -> String {
    format!("The newest {count} are listed.")
}
