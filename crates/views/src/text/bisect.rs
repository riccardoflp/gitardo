//! The words of the history of a repository: the reflog (`reflog.`), and a bisect, with the entries of the menu of a
//! commit, the banner above the commits, the marks of the commits and the message that names the first bad commit
//! (`bisect.`).

/// **Reflog** in the **…** menu of the header (`reflog.browse`).
pub const REFLOG: &str = "Reflog";

/// **Start bisect: this commit is bad** in the menu of a commit (`bisect.start`).
pub const START_BAD: &str = "Start bisect: this commit is bad";
/// **Start bisect: this commit is good** in the menu of a commit (`bisect.start`).
pub const START_GOOD: &str = "Start bisect: this commit is good";
/// **Bisect: mark as good** in the menu of a commit while a bisect is under way (`bisect.mark`).
pub const MARK_GOOD: &str = "Bisect: mark as good";
/// **Bisect: mark as bad** in the menu of a commit while a bisect is under way (`bisect.mark`).
pub const MARK_BAD: &str = "Bisect: mark as bad";
/// The banner before any commit is marked bad (`bisect.banner`).
pub const MARK_BAD_TO_BEGIN: &str = "Bisect: mark a bad commit to begin";
/// The banner once a bad commit is marked and no good one (`bisect.banner`).
pub const MARK_GOOD_TO_BEGIN: &str = "Bisect: mark a good commit to begin";
/// **Good** in the banner (`bisect.answer`).
pub const GOOD: &str = "Good";
/// The tooltip of **Good**.
pub const GOOD_TOOLTIP: &str = "git bisect good";
/// **Bad** in the banner (`bisect.answer`).
pub const BAD: &str = "Bad";
/// The tooltip of **Bad**.
pub const BAD_TOOLTIP: &str = "git bisect bad";
/// **Skip** in the banner (`bisect.answer`).
pub const SKIP: &str = "Skip";
/// The tooltip of **Skip**.
pub const SKIP_TOOLTIP: &str = "git bisect skip: this commit cannot be tested";
/// **Show** in the banner once the first bad commit is found.
pub const SHOW: &str = "Show";
/// **End** in the banner (`bisect.end`).
pub const END: &str = "End";
/// The tooltip of **End**.
pub const END_TOOLTIP: &str = "git bisect reset";
/// **Show in the timeline** in the message that names the first bad commit.
pub const SHOW_IN_TIMELINE: &str = "Show in the timeline";
/// **End bisect** in the message that names the first bad commit.
pub const END_BISECT: &str = "End bisect";
/// The label of a commit marked good (`bisect.marks`).
pub const GOOD_MARK: &str = "good";
/// The label of the commit marked bad.
pub const BAD_MARK: &str = "bad";
/// The label of a commit skipped.
pub const SKIPPED_MARK: &str = "skipped";
/// What the screen reader calls the banner.
pub const BANNER: &str = "Bisect";

/// The banner while commits are left to test: *Bisect: test `<short>` · N commits can still be the first bad one*.
#[must_use]
pub fn test(short: &str, remaining: u32) -> String {
    let commits = if remaining == 1 { "commit" } else { "commits" };
    format!("Bisect: test {short} \u{b7} {remaining} {commits} can still be the first bad one")
}

/// The banner once the first bad commit is found: *Bisect: the first bad commit is `<short>`*.
#[must_use]
pub fn found(short: &str) -> String {
    format!("Bisect: the first bad commit is {short}")
}

/// The message once the first bad commit is found: *Bisect: the first bad commit is `<short>` "`<subject>`".*
#[must_use]
pub fn first_bad(short: &str, subject: &str) -> String {
    format!("Bisect: the first bad commit is {short} \"{subject}\".")
}
