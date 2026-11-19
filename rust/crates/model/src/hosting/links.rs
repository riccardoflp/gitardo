//! The addresses of the pages of a hosting service (`hosting.web-links`).

use super::{HostingPage, HostingRepo, HostingService};
use crate::compare::CompareMode;
use crate::github::link::percent_encoded;

/// What a side of a comparison is on the service.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SideKind {
    /// A branch, by the name it has there.
    Branch,
    /// A tag.
    Tag,
    /// A commit, by its full hash.
    Commit,
}

/// A side of a comparison by the name it has on the service.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NamedSide<'a> {
    /// The name: `feature/login`, `v1.0`, or a full hash.
    pub name: &'a str,
    /// What it is.
    pub kind: SideKind,
}

/// The address of a page of the project on its service, `<web>` being the address of the project: a pull request, the
/// page that creates one for a branch, or a comparison. Each part of a name between `/` is percent-encoded, the `/`
/// kept. On a service that compares only since the two diverged, a comparison of everything that differs is given
/// since they diverged all the same.
///
/// Feature: hosting.web-links
#[must_use]
pub fn web_link(repo: &HostingRepo, page: &HostingPage<'_>) -> String {
    let web = &repo.web;
    match *page {
        HostingPage::PullRequest(number) => match repo.service {
            HostingService::GitHub => format!("{web}/pull/{number}"),
            HostingService::GitLab => format!("{web}/-/merge_requests/{number}"),
            HostingService::Bitbucket => format!("{web}/pull-requests/{number}"),
            HostingService::AzureDevOps => format!("{web}/pullrequest/{number}"),
            HostingService::Gitea | HostingService::Forgejo => format!("{web}/pulls/{number}"),
        },
        HostingPage::Create(branch) => create_link(repo, &encoded(branch)),
        HostingPage::Compare { first, second, mode } => compare_link(repo, (first, second), mode),
    }
}

/// The page that creates a pull request of a branch, its name encoded.
fn create_link(repo: &HostingRepo, branch: &str) -> String {
    let web = &repo.web;
    match repo.service {
        HostingService::GitHub => format!("{web}/compare/{branch}?expand=1"),
        HostingService::GitLab => format!("{web}/-/merge_requests/new?merge_request%5Bsource_branch%5D={branch}"),
        HostingService::Bitbucket => format!("{web}/pull-requests/new?source={branch}"),
        HostingService::AzureDevOps => format!("{web}/pullrequestcreate?sourceRef={branch}"),
        HostingService::Gitea | HostingService::Forgejo => {
            format!("{web}/compare/{}...{branch}", encoded(repo.default_branch()))
        }
    }
}

/// The page of a comparison of two sides.
fn compare_link(repo: &HostingRepo, (first, second): (NamedSide<'_>, NamedSide<'_>), mode: CompareMode) -> String {
    let web = &repo.web;
    let (a, b) = (encoded(first.name), encoded(second.name));
    let is_everything = mode == CompareMode::Everything;
    match repo.service {
        HostingService::GitHub => {
            let separator = if is_everything { ".." } else { "..." };
            format!("{web}/compare/{a}{separator}{b}")
        }
        HostingService::GitLab if is_everything => format!("{web}/-/compare/{a}...{b}?straight=true"),
        HostingService::GitLab => format!("{web}/-/compare/{a}...{b}"),
        HostingService::Bitbucket => format!("{web}/branches/compare/{b}%0D{a}"),
        HostingService::AzureDevOps => format!(
            "{web}/branchCompare?baseVersion={}{a}&targetVersion={}{b}",
            azure_kind(first.kind),
            azure_kind(second.kind)
        ),
        HostingService::Gitea | HostingService::Forgejo => format!("{web}/compare/{a}...{b}"),
    }
}

/// How Azure DevOps marks what a version is: `GB` a branch, `GT` a tag, `GC` a commit.
fn azure_kind(kind: SideKind) -> &'static str {
    match kind {
        SideKind::Branch => "GB",
        SideKind::Tag => "GT",
        SideKind::Commit => "GC",
    }
}

/// A name with each part between `/` percent-encoded, the `/` kept.
fn encoded(name: &str) -> String {
    name.split('/').map(percent_encoded).collect::<Vec<_>>().join("/")
}
