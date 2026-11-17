//! The words of the pages and their trail.

/// The first step of every trail, which comes back to the timeline.
pub const TIMELINE: &str = "Timeline";
/// The Welcome page in its trail.
pub const WELCOME: &str = "Welcome";
/// What is between two steps of a trail.
pub const TRAIL_SEPARATOR: &str = "\u{203a}";
/// The button that closes the page shown.
pub const CLOSE_PAGE: &str = "Close the page";
/// The trail, for the screen reader.
pub const TRAIL: &str = "Trail";
/// The name of the trail in place of the pages folded between **Timeline** and the last names, which lists them
/// (`ui.page-trail`).
pub const FOLDED: &str = "\u{2026}";
