//! The words of the bar of views (`graph.view-bar`).

/// The button that comes back to the timeline.
pub const TIMELINE: &str = "Timeline";
/// The button that opens the Changes page.
pub const CHANGES: &str = "Changes";
/// The button that opens the Files page.
pub const FILES: &str = "Files";
/// The button that asks what to compare.
pub const COMPARE: &str = "Compare";
/// The button that shows the bottom panel at its Activity tab, and hides it.
pub const ACTIVITY: &str = "Activity";
/// The button that opens the settings page.
pub const SETTINGS: &str = "Settings";
/// The name of the bar, for a screen reader.
pub const BAR: &str = "Views";

/// The number on **Changes**: the changed files, `99+` beyond 99.
#[must_use]
pub fn changed_count(count: usize) -> String {
    if count > 99 {
        String::from("99+")
    } else {
        count.to_string()
    }
}
