//! The words of the timeline (`graph.view`) and of the inspector beside it.

/// The timeline, for the screen reader.
pub const TIMELINE: &str = "Timeline";
/// The list of the commits, for the screen reader.
pub const HISTORY: &str = "History";
/// What the timeline says in a repository with no commits.
pub const NO_COMMITS_YET: &str = "No commits yet";
/// What the timeline says when it lists no commits otherwise.
pub const NO_COMMITS_TO_SHOW: &str = "No commits to show";
/// What the timeline says while the first commits are read.
pub const LOADING_COMMITS: &str = "Loading commits\u{2026}";
/// The inspector, for the screen reader.
pub const INSPECTOR: &str = "Selected commit";

/// The second line of a row: who wrote the commit and when.
#[must_use]
pub fn author_and_date(author: &str, date: &str) -> String {
    format!("{author} \u{b7} {date}")
}
