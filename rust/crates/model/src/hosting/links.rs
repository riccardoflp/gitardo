//! The addresses of the pages of a hosting service (`hosting.web-links`), and the link that opens or creates the pull
//! request of a branch there (`hosting.open-pull-request`).

use super::{HostingPage, HostingRepo, HostingService};
use crate::compare::CompareMode;
use crate::contract::Text;
use crate::github::link::percent_encoded;
use crate::github::{BranchOf, PullLink, PullLinkKind, PullRequests};

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

/// What the link of a branch on a hosting service is worked out from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HostingPullLinkInput<'a> {
    /// The project of the main remote; none when it is on no service Gitardo knows.
    pub repository: Option<&'a HostingRepo>,
    /// The branch.
    pub branch: BranchOf<'a>,
    /// The open merge requests known, by the name of their branch; none known before they are loaded, and none read on
    /// the services other than GitLab.
    pub pull_requests: &'a PullRequests,
}

/// The entry that opens the merge request of a branch on GitLab when one is known, or the page of the service that
/// creates a pull request for a local branch whose upstream is on the main remote and not gone; none on GitHub, where
/// `github.open-pull-request` makes it, for a local branch with no such upstream, and for a remote branch with no merge
/// request known.
///
/// Feature: hosting.open-pull-request
#[must_use]
pub fn pull_request_link(input: &HostingPullLinkInput<'_>) -> Option<PullLink> {
    let repository = input
        .repository
        .filter(|repository| repository.service != HostingService::GitHub)?;
    let service = repository.service;
    let (name, may_create) = match input.branch {
        BranchOf::Local { upstream, .. } => {
            let upstream = upstream.filter(|upstream| !upstream.is_gone && upstream.remote == repository.remote)?;
            let name = upstream
                .remote_branch
                .strip_prefix("refs/heads/")
                .unwrap_or(&upstream.remote_branch);
            (name, true)
        }
        BranchOf::Remote { name } => (name, false),
    };
    let words = service.pull_request_words();
    if let Some(pull_request) = input.pull_requests.of_branch(name) {
        return Some(PullLink {
            kind: PullLinkKind::Open(pull_request.number),
            label: Text::from(format!("Open {words} {}", service.number(pull_request.number))),
            url: pull_request.url.clone(),
        });
    }
    may_create.then(|| PullLink {
        kind: PullLinkKind::Create,
        label: Text::from(format!("Create {words}\u{2026}")),
        url: web_link(repository, &HostingPage::Create(name)),
    })
}
