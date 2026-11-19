//! The words of what a rewrite changed, commit by commit: the page and the entries that open it (`range-diff.`).

/// The name of the page in the trail when it has no branch.
pub const PAGE: &str = "What changed";
/// What the page says until the pairs are read.
pub const COMPARING: &str = "Comparing\u{2026}";
/// The line of the commits before.
pub const BEFORE: &str = "Before";
/// The line of the commits now.
pub const NOW: &str = "Now";
/// What changed in a pair: its message.
pub const MESSAGE: &str = "message";
/// What changed in a pair: what it changes.
pub const CHANGES: &str = "changes";
/// The tooltip of the icon of a pair the same.
pub const SAME: &str = "The same";
/// The tooltip of the icon of a pair changed.
pub const CHANGED: &str = "Changed";
/// The tooltip of the icon of a pair added.
pub const ADDED: &str = "Added";
/// The tooltip of the icon of a pair dropped.
pub const DROPPED: &str = "Dropped";

/// **What its last rewrite changed** in the menu of a local branch (`range-diff.compare`).
pub const LAST_REWRITE: &str = "What its last rewrite changed";
/// **What the force push changed** in the menu of a remote branch (`range-diff.compare`).
pub const FORCE_PUSH: &str = "What the force push changed";

/// **Compare with `<upstream>` commit by commit** in the menu of a local branch and of **Sync** (`range-diff.compare`).
#[must_use]
pub fn compare_with_upstream(upstream: &str) -> String {
    format!("Compare with {upstream} commit by commit")
}

/// The name of the page in its tab and its title row: `feature · what changed`.
#[must_use]
pub fn title(branch: &str) -> String {
    format!("{branch} \u{b7} what changed")
}

/// A line of commits: `Before: 4 commits from 3c6e730`.
#[must_use]
pub fn line(label: &str, count: usize, from: &str) -> String {
    let commits = if count == 1 { "commit" } else { "commits" };
    format!("{label}: {count} {commits} from {from}")
}

/// What changed in a pair: **message**, **changes** or both.
#[must_use]
pub fn changed_parts(message: bool, changes: bool) -> Option<String> {
    match (message, changes) {
        (true, true) => Some(format!("{MESSAGE} and {CHANGES}")),
        (true, false) => Some(MESSAGE.to_owned()),
        (false, true) => Some(CHANGES.to_owned()),
        (false, false) => None,
    }
}

/// What is said when the force push of a fetch was found: "`origin/feature` was force pushed."
#[must_use]
pub fn force_pushed(remote_branch: &str) -> String {
    format!("`{remote_branch}` was force pushed.")
}
