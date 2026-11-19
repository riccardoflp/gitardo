//! The words of the pull requests of GitHub (`github.load-pull-requests`, `github.read-pull-requests`,
//! `graph.pull-request-badge`, `github.open-pull-request`, `github.offer-after-push`), and of the merge requests of a
//! GitLab (`hosting.load-merge-requests`, `hosting.read-merge-requests`).

use model::hosting::HostingService;

/// The entry of the **…** menu that loads the pull requests.
pub const LOAD_PULL_REQUESTS: &str = "Load pull requests from GitHub";
/// The command of Quick search that loads the pull requests.
pub const SIGN_IN: &str = "Sign in to GitHub";
/// What the Activity tab says while the pull requests are read.
pub const READING_PULL_REQUESTS: &str = "Reading the pull requests from GitHub";
/// What the Activity tab says while the merge requests of a GitLab are read.
pub const READING_MERGE_REQUESTS: &str = "Reading the merge requests from GitLab";
/// The entry of the **…** menu that loads the merge requests of a GitLab (`hosting.load-merge-requests`).
pub const LOAD_MERGE_REQUESTS: &str = "Load merge requests from GitLab";
/// The command of Quick search that loads the merge requests of a GitLab.
pub const SIGN_IN_TO_GITLAB: &str = "Sign in to GitLab";
/// The line under the tooltip of a badge: **Click to open it on GitHub**.
#[must_use]
pub fn click_to_open(service: HostingService) -> String {
    format!("Click to open it on {}", service.name())
}

/// What is said, or logged, when the pull requests could not be read: `Could not read pull requests. GitHub answered
/// 401 Unauthorized`.
#[must_use]
pub fn could_not_read(said: &str) -> String {
    format!("Could not read pull requests. {said}")
}

/// What is said, or logged, when the merge requests of a GitLab could not be read: `Could not read merge requests.
/// GitLab answered 401 Unauthorized`.
#[must_use]
pub fn could_not_read_merge_requests(said: &str) -> String {
    format!("Could not read merge requests. {said}")
}

/// What is said when the browser of the system could not open a pull request.
#[must_use]
pub fn could_not_open(said: &str) -> String {
    format!("Could not open the browser. {said}")
}

/// The first line of the tooltip of a badge: `Pull request #12: Add the parser · checks passed`; on GitLab `Merge
/// request !12: Add the parser · pipeline passed`.
#[must_use]
pub fn badge_tooltip(service: HostingService, pull_request: &model::github::PullRequest) -> String {
    let kind = match (pull_request.is_draft, service.has_merge_requests()) {
        (true, true) => "Draft merge request",
        (false, true) => "Merge request",
        (true, false) => "Draft pull request",
        (false, false) => "Pull request",
    };
    let what = if service.has_merge_requests() {
        "pipeline"
    } else {
        "checks"
    };
    let checks = match pull_request.checks {
        Some(model::github::Checks::Passed) => format!(" \u{b7} {what} passed"),
        Some(model::github::Checks::Failed) => format!(" \u{b7} {what} failed"),
        Some(model::github::Checks::Running) => format!(" \u{b7} {what} running"),
        None => String::new(),
    };
    let number = service.number(pull_request.number);
    format!("{kind} {number}: {}{checks}", pull_request.title)
}

/// The text of a badge: `#12`, `!12` on GitLab.
#[must_use]
pub fn badge_text(service: HostingService, number: u64) -> String {
    service.number(number)
}
