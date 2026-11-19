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
/// **Open on GitLab**, on the hosting service named (`compare.open-on-service`).
#[must_use]
pub fn open_on(service: &str) -> String {
    format!("Open on {service}")
}
/// Its tooltip.
#[must_use]
pub fn open_on_tooltip(service: &str) -> String {
    format!("Compare the two on {service}, once they are pushed")
}
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

/// The title of the comparison chooser opened to start a comparison (`compare.chooser`).
pub const CHOOSER_TITLE: &str = "Compare branches, tags or commits";
/// Its title when it chooses another side of a comparison page (`compare.change-side`).
pub const CHANGE_SIDE_TITLE: &str = "Choose another side";
/// The placeholder of **From** while it is empty.
pub const CHOOSE_FIRST: &str = "Choose the first side";
/// The placeholder of **To** while it is empty.
pub const CHOOSE_SECOND: &str = "Choose the side to compare it with";
/// The placeholder of the field that filters the list of the chooser.
pub const FIND_SIDE: &str = "Find a branch, a tag or a commit, or type a revision";
/// The list of the chooser, for the screen reader.
pub const SIDES: &str = "What the side can be";
/// The heading of what is checked out, and of the working tree.
pub const CHECKED_OUT: &str = "Checked out";
/// The heading of the local branches.
pub const BRANCHES: &str = "Branches";
/// The heading of the remote branches.
pub const REMOTE_BRANCHES: &str = "Remote branches";
/// The heading of the tags.
pub const TAGS: &str = "Tags";
/// The heading of the last commits of the branch checked out.
pub const RECENT_COMMITS: &str = "Recent commits";
/// What the chooser says under its field when git knows no commit by the name typed.
pub const UNKNOWN_REVISION: &str = "Git does not know a commit by this name.";
/// The heading of the comparisons opened before, at the foot of the chooser.
pub const OPENED_BEFORE: &str = "Opened before";
/// The button that lists the comparisons opened before that do not fit on the line.
pub const MORE: &str = "More\u{2026}";
/// **Compare**, the main button of the chooser.
pub const COMPARE: &str = "Compare";
/// **Cancel**.
pub const CANCEL: &str = "Cancel";
/// The column of the comparison beside its diffs, for the screen reader (`compare.beside-diff`).
pub const COLUMN: &str = "Comparison beside the diff";

/// The row of a revision typed that git knows: **Use `HEAD~3`**.
#[must_use]
pub fn use_revision(typed: &str) -> String {
    format!("Use {typed}")
}

/// A comparison opened before, at the foot of the chooser: `<first> ↔ <second>`, with its path when it was narrowed.
#[must_use]
pub fn opened_before(first: &str, second: &str, path: Option<&str>) -> String {
    let mut said = format!("{first} \u{2194} {second}");
    if let Some(path) = path {
        let _ = write!(said, " \u{b7} {path}");
    }
    said
}

/// The fold of the commits in the column beside a diff: **Commits · 2 in feature, 1 in main**.
#[must_use]
pub fn column_commits((first, only_in_first): (&str, usize), (second, only_in_second): (&str, usize)) -> String {
    format!("{COMMITS} \u{b7} {only_in_second} in {second}, {only_in_first} in {first}")
}

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
