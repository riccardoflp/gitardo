//! The offer to create a pull request after the first push of a branch, on a hosting service other than GitHub
//! (`hosting.offer-after-push`).

use super::{HostingPage, HostingRepo, HostingService, web_link};
use crate::contract::Text;
use crate::github::{Notice, PullRequests};

/// How a push ended, and what was known before it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HostingPushOutcome<'a> {
    /// Whether it succeeded.
    pub succeeded: bool,
    /// The branch checked out, which was pushed; none when `HEAD` is detached.
    pub branch: Option<&'a str>,
    /// Whether the branch had an upstream before the push; one that had none got it from the push.
    pub had_upstream: bool,
    /// The project of the main remote; none when it is on no service Gitardo knows.
    pub repository: Option<&'a HostingRepo>,
    /// The open merge requests known.
    pub pull_requests: &'a PullRequests,
}

/// The offer to create a pull request, **Create merge request** on GitLab, when a push succeeded, a branch is checked
/// out, it had no upstream before, the main remote is on a service other than GitHub and no open merge request is
/// known for the branch's name; nothing otherwise, and nothing on GitHub, where `github.offer-after-push` offers it.
///
/// Feature: hosting.offer-after-push
#[must_use]
pub fn offer_after_push(input: &HostingPushOutcome<'_>) -> Option<Notice> {
    let branch = input.branch?;
    let repository = input
        .repository
        .filter(|repository| repository.service != HostingService::GitHub)?;
    if !input.succeeded || input.had_upstream || input.pull_requests.of_branch(branch).is_some() {
        return None;
    }
    Some(Notice {
        message: Text::from(format!("Pushed {branch}.")),
        button: Text::from(format!("Create {}", repository.service.pull_request_words())),
        url: web_link(repository, &HostingPage::Create(branch)),
    })
}
