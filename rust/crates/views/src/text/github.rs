//! The words of the pull requests of GitHub (`github.load-pull-requests`, `github.read-pull-requests`,
//! `graph.pull-request-badge`, `github.open-pull-request`, `github.offer-after-push`).

/// The entry of the **…** menu that loads the pull requests.
pub const LOAD_PULL_REQUESTS: &str = "Load pull requests from GitHub";
/// The command of Quick search that loads the pull requests.
pub const SIGN_IN: &str = "Sign in to GitHub";
/// What the Activity tab says while the pull requests are read.
pub const READING_PULL_REQUESTS: &str = "Reading the pull requests from GitHub";
/// The line under the tooltip of a badge.
pub const CLICK_TO_OPEN: &str = "Click to open it on GitHub";

/// What is said, or logged, when the pull requests could not be read: `Could not read pull requests. GitHub answered
/// 401 Unauthorized`.
#[must_use]
pub fn could_not_read(said: &str) -> String {
    format!("Could not read pull requests. {said}")
}

/// What is said when the browser of the system could not open a pull request.
#[must_use]
pub fn could_not_open(said: &str) -> String {
    format!("Could not open the browser. {said}")
}

/// The first line of the tooltip of a badge: `Pull request #12: Add the parser · checks passed`.
#[must_use]
pub fn badge_tooltip(number: u64, title: &str, is_draft: bool, checks: Option<model::github::Checks>) -> String {
    let kind = if is_draft { "Draft pull request" } else { "Pull request" };
    let checks = match checks {
        Some(model::github::Checks::Passed) => " \u{b7} checks passed",
        Some(model::github::Checks::Failed) => " \u{b7} checks failed",
        Some(model::github::Checks::Running) => " \u{b7} checks running",
        None => "",
    };
    format!("{kind} #{number}: {title}{checks}")
}

/// The text of a badge: `#12`.
#[must_use]
pub fn badge_text(number: u64) -> String {
    format!("#{number}")
}
