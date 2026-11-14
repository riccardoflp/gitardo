//! The offer to create a pull request after the first push of a branch (`github.offer-after-push`).

use super::GitHubRepo;
use super::link::compare_url;
use super::pulls::PullRequests;
use crate::contract::Text;

/// How a push ended, and what was known before it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PushOutcome<'a> {
    /// Whether it succeeded.
    pub succeeded: bool,
    /// The branch checked out, which was pushed; none when `HEAD` is detached.
    pub branch: Option<&'a str>,
    /// Whether the branch had an upstream before the push; one that had none got it from the push.
    pub had_upstream: bool,
    /// The repository on GitHub of the main remote; none when it is not on GitHub.
    pub repository: Option<&'a GitHubRepo>,
    /// The open pull requests known.
    pub pull_requests: &'a PullRequests,
}

/// What is said once a branch was pushed for the first time, with a button that opens a page.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Notice {
    /// The sentence: "Pushed feature."
    pub message: Text,
    /// The words of the button: **Create pull request**.
    pub button: Text,
    /// The address the button opens: GitHub's page to create a pull request of the branch.
    pub url: String,
}

/// The offer to create a pull request, when a push succeeded, a branch is checked out, it had no upstream before, the
/// main remote is on GitHub and no open pull request is known for the branch's name; nothing otherwise.
///
/// Feature: github.offer-after-push
#[must_use]
pub fn offer_after_push(input: &PushOutcome<'_>) -> Option<Notice> {
    let branch = input.branch?;
    let repository = input.repository?;
    if !input.succeeded || input.had_upstream || input.pull_requests.of_branch(branch).is_some() {
        return None;
    }
    Some(Notice {
        message: Text::from(format!("Pushed {branch}.")),
        button: Text::from("Create pull request"),
        url: compare_url(repository, branch),
    })
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests {
    use super::{Notice, PushOutcome, offer_after_push};
    use crate::contract::Text;
    use crate::github::{GitHubRepo, PullRequests};

    fn repository() -> GitHubRepo {
        GitHubRepo {
            owner: String::from("owner"),
            repo: String::from("app"),
        }
    }

    fn pushed<'a>(repository: Option<&'a GitHubRepo>, pull_requests: &'a PullRequests) -> PushOutcome<'a> {
        PushOutcome {
            succeeded: true,
            branch: Some("feature"),
            had_upstream: false,
            repository,
            pull_requests,
        }
    }

    #[test]
    fn github_offer_after_push__the_first_push_of_a_branch_offers_to_create_a_pull_request() {
        let repository = repository();
        let none = PullRequests::default();
        assert_eq!(
            offer_after_push(&pushed(Some(&repository), &none)),
            Some(Notice {
                message: Text::from("Pushed feature."),
                button: Text::from("Create pull request"),
                url: String::from("https://github.com/owner/app/compare/feature?expand=1"),
            })
        );
    }

    #[test]
    fn github_offer_after_push__a_second_push_offers_nothing() {
        let repository = repository();
        let none = PullRequests::default();
        let second = PushOutcome {
            had_upstream: true,
            ..pushed(Some(&repository), &none)
        };
        assert_eq!(offer_after_push(&second), None);
    }

    #[test]
    fn github_offer_after_push__a_repository_not_on_github_offers_nothing() {
        let none = PullRequests::default();
        assert_eq!(offer_after_push(&pushed(None, &none)), None);
    }

    #[test]
    fn github_offer_after_push__a_push_that_failed_offers_nothing() {
        let repository = repository();
        let none = PullRequests::default();
        let failed = PushOutcome {
            succeeded: false,
            ..pushed(Some(&repository), &none)
        };
        assert_eq!(offer_after_push(&failed), None);
        let detached = PushOutcome {
            branch: None,
            ..pushed(Some(&repository), &none)
        };
        assert_eq!(offer_after_push(&detached), None, "nothing with no branch checked out");
    }
}
