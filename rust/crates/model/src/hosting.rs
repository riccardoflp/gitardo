//! The hosting services a remote can be on, GitHub, GitLab, Bitbucket, Azure DevOps, Gitea and Forgejo: which one the
//! main remote is on and the project there (`hosting.detect-remote`), the addresses of its pages
//! (`hosting.web-links`), the links that open or create the pull request of a branch (`hosting.open-pull-request`),
//! and the offer to create one after the first push of a branch (`hosting.offer-after-push`).

mod detect;
mod links;
mod offer;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_links;

pub use detect::{SELF_HOSTED_KINDS, detect_remote, self_hosted_kind};
pub use links::{HostingPullLinkInput, NamedSide, SideKind, pull_request_link, web_link};
pub use offer::{HostingPushOutcome, offer_after_push};

use std::collections::BTreeMap;

/// A hosting service a remote can be on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum HostingService {
    /// github.com.
    GitHub,
    /// gitlab.com, or a GitLab of one's own.
    GitLab,
    /// bitbucket.org.
    Bitbucket,
    /// Azure DevOps: dev.azure.com, and the older `<org>.visualstudio.com`.
    AzureDevOps,
    /// A Gitea of one's own.
    Gitea,
    /// codeberg.org, or a Forgejo of one's own.
    Forgejo,
}

impl HostingService {
    /// Its name, as the menus say it: **Open on Azure DevOps**.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            HostingService::GitHub => "GitHub",
            HostingService::GitLab => "GitLab",
            HostingService::Bitbucket => "Bitbucket",
            HostingService::AzureDevOps => "Azure DevOps",
            HostingService::Gitea => "Gitea",
            HostingService::Forgejo => "Forgejo",
        }
    }

    /// Whether it names them merge requests, numbered `!12`, as GitLab does; the others name them pull requests,
    /// numbered `#12`.
    #[must_use]
    pub fn has_merge_requests(self) -> bool {
        self == HostingService::GitLab
    }

    /// What a pull request is called there: `merge request` on GitLab, `pull request` elsewhere.
    #[must_use]
    pub fn pull_request_words(self) -> &'static str {
        if self.has_merge_requests() {
            "merge request"
        } else {
            "pull request"
        }
    }

    /// The number of a pull request as the service writes it: `!12` on GitLab, `#12` elsewhere.
    #[must_use]
    pub fn number(self, number: u64) -> String {
        let sign = if self.has_merge_requests() { '!' } else { '#' };
        format!("{sign}{number}")
    }

    /// Whether it keeps the commits of each pull request under a reference of its own, which a fetch can bring:
    /// `refs/pull/<n>/head` on GitHub, Gitea and Forgejo, `refs/merge-requests/<n>/head` on GitLab. Bitbucket keeps
    /// none, and Azure DevOps only the merge (Q1153).
    #[must_use]
    pub fn pull_request_reference(self, number: u64) -> Option<String> {
        match self {
            HostingService::GitHub | HostingService::Gitea | HostingService::Forgejo => {
                Some(format!("refs/pull/{number}/head"))
            }
            HostingService::GitLab => Some(format!("refs/merge-requests/{number}/head")),
            HostingService::Bitbucket | HostingService::AzureDevOps => None,
        }
    }

    /// Whether its page of a comparison shows only what changed since the two diverged, as Bitbucket, Gitea and
    /// Forgejo do, and not everything that differs.
    #[must_use]
    pub fn compares_only_since_diverged(self) -> bool {
        matches!(
            self,
            HostingService::Bitbucket | HostingService::Gitea | HostingService::Forgejo
        )
    }
}

/// A GitLab, a Gitea or a Forgejo of one's own, as `hosting.selfHosted` names it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelfHosted {
    /// The service: GitLab, Gitea or Forgejo.
    pub service: HostingService,
    /// The address of its web pages, when it is on another name than the remote's: `https://git.example.com`.
    pub web: Option<String>,
}

/// The hosting services of one's own, by their host name in lower case (`hosting.selfHosted`).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct KnownHosts {
    /// Each host name, with its service and the address of its pages.
    pub by_host: BTreeMap<String, SelfHosted>,
}

/// A project on a hosting service: the one of the main remote (`hosting.detect-remote`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostingRepo {
    /// The service.
    pub service: HostingService,
    /// The address of the project on the web, without `/` at its end: `https://gitlab.com/group/sub/app`.
    pub web: String,
    /// The host name its API is asked at: `gitlab.com`.
    pub host: String,
    /// Its path on the service: `group/sub/app`; `org/Project/app` on Azure DevOps.
    pub path: String,
    /// The name of the remote: `origin`.
    pub remote: String,
    /// The URL the remote is fetched from.
    pub url: String,
    /// The branch `refs/remotes/<remote>/HEAD` names, the target of a new pull request on Gitea and Forgejo; none when
    /// the repository does not have it, and `main` is taken.
    pub default_branch: Option<String>,
}

impl HostingRepo {
    /// The branch a new pull request goes into on Gitea and Forgejo: the default branch of the remote, else `main`.
    #[must_use]
    pub fn default_branch(&self) -> &str {
        self.default_branch.as_deref().unwrap_or("main")
    }
}

/// A page of a hosting service that Gitardo opens in the browser.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HostingPage<'a> {
    /// The pull request of that number.
    PullRequest(u64),
    /// The page that creates a pull request of a branch, by the name it has on the remote.
    Create(&'a str),
    /// The comparison of two sides.
    Compare {
        /// The first side.
        first: NamedSide<'a>,
        /// The second side.
        second: NamedSide<'a>,
        /// **Since they diverged** or **Everything that differs**.
        mode: crate::compare::CompareMode,
    },
}
