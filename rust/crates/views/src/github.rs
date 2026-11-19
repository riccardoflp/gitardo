//! What the views are told of GitHub: the repository on GitHub of the main remote and the open pull requests, from
//! which the badges of the labels (`graph.pull-request-badge`) and the links of the menus of the branches
//! (`github.open-pull-request`) are made.

use model::github::{BranchOf, GitHubRepo, PullLink, PullLinkInput, PullRequest, PullRequests, pull_request_link};
use model::hosting::{self, HostingPullLinkInput, HostingRepo, HostingService};
use model::state::{Branch, RemoteBranch, RepositoryState};

/// The pull requests of GitHub, as the views are given them.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct GitHubShown<'a> {
    /// The repository on GitHub of the main remote; none when it is on another host, or not known yet.
    pub repository: Option<&'a GitHubRepo>,
    /// The project of the main remote on its hosting service, GitHub included; none when it is on none Gitardo knows,
    /// or not known yet (`hosting.detect-remote`).
    pub hosting: Option<&'a HostingRepo>,
    /// The open pull requests; none before they are loaded.
    pub pull_requests: Option<&'a PullRequests>,
}

impl<'a> GitHubShown<'a> {
    /// Whether the pull requests of the main remote are read: on GitHub, and the merge requests of a GitLab.
    #[must_use]
    pub fn has_pull_requests(&self) -> bool {
        self.repository.is_some() || self.hosting.is_some_and(|hosting| hosting.service.has_merge_requests())
    }

    /// The hosting service of the main remote, whose words the badges and the menus say; GitHub when it is not known.
    #[must_use]
    pub fn service(&self) -> HostingService {
        match (self.repository, self.hosting) {
            (None, Some(hosting)) => hosting.service,
            _ => HostingService::GitHub,
        }
    }

    /// The pull request of a local branch: the one of the name it was pushed under, its upstream not gone.
    #[must_use]
    pub fn of_branch(&self, branch: &Branch) -> Option<&'a PullRequest> {
        let upstream = branch.upstream.as_ref().filter(|upstream| !upstream.is_gone)?;
        let name = upstream
            .remote_branch
            .strip_prefix("refs/heads/")
            .unwrap_or(&upstream.remote_branch);
        if !self.has_pull_requests() {
            return None;
        }
        self.pull_requests?.of_branch(name)
    }

    /// The pull request of a remote branch on its own label: only when no local branch tracks it.
    #[must_use]
    pub fn of_remote_branch(&self, repository: &RepositoryState, remote: &RemoteBranch) -> Option<&'a PullRequest> {
        let reference = format!("refs/remotes/{}", remote.name);
        let is_tracked = repository.branches.iter().any(|branch| {
            branch
                .upstream
                .as_ref()
                .is_some_and(|upstream| upstream.reference == reference)
        });
        if is_tracked {
            return None;
        }
        self.of_remote_name(remote)
    }

    /// The pull request of a remote branch, by its name without its remote, tracked or not.
    #[must_use]
    pub fn of_remote_name(&self, remote: &RemoteBranch) -> Option<&'a PullRequest> {
        let name = remote_name(remote);
        if !self.has_pull_requests() {
            return None;
        }
        self.pull_requests?.of_branch(name)
    }

    /// The entry of the menu of a local branch that opens its pull request, or the page of the service to create one:
    /// GitHub's (`github.open-pull-request`), or another service's (`hosting.open-pull-request`).
    #[must_use]
    pub fn link_of_branch(&self, branch: &Branch) -> Option<PullLink> {
        let none = PullRequests::default();
        let branch = BranchOf::Local {
            name: &branch.name,
            upstream: branch.upstream.as_ref(),
        };
        self.link(branch, self.pull_requests.unwrap_or(&none))
    }

    /// The entry of the menu of a remote branch that opens its pull request, when one is known.
    #[must_use]
    pub fn link_of_remote_branch(&self, remote: &RemoteBranch) -> Option<PullLink> {
        let branch = BranchOf::Remote {
            name: remote_name(remote),
        };
        self.link(branch, self.pull_requests?)
    }

    /// The link of a branch on GitHub, or on the service of the main remote.
    fn link(&self, branch: BranchOf<'_>, pull_requests: &PullRequests) -> Option<PullLink> {
        pull_request_link(&PullLinkInput {
            repository: self.repository,
            branch,
            pull_requests,
        })
        .or_else(|| {
            hosting::pull_request_link(&HostingPullLinkInput {
                repository: self.hosting,
                branch,
                pull_requests,
            })
        })
    }
}

/// The name of a remote branch without its remote: `feature` for `origin/feature`.
fn remote_name(remote: &RemoteBranch) -> &str {
    remote
        .name
        .strip_prefix(remote.remote.as_str())
        .and_then(|rest| rest.strip_prefix('/'))
        .unwrap_or(&remote.name)
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests {
    use model::CommitId;
    use model::github::{GitHubRepo, PullRequest, PullRequests};
    use model::state::{Branch, RemoteBranch, RepositoryState, Upstream};

    use super::GitHubShown;

    /// Pull request #7 of `feature`.
    fn pull_requests() -> PullRequests {
        let mut pull_requests = PullRequests::default();
        let pull_request = PullRequest {
            number: 7,
            title: String::from("Add the parser"),
            url: String::from("https://github.com/owner/app/pull/7"),
            is_draft: false,
            branch: String::from("feature"),
            checks: None,
        };
        let _none = pull_requests.by_branch.insert(String::from("feature"), pull_request);
        pull_requests
    }

    /// A repository with `origin/feature`, tracked by `feature` or by nothing.
    fn repository(is_tracked: bool) -> (RepositoryState, RemoteBranch) {
        let commit = CommitId::parse("4b830a6c0ffee000000000000000000000000000").unwrap();
        let upstream = Upstream {
            reference: String::from("refs/remotes/origin/feature"),
            remote: String::from("origin"),
            remote_branch: String::from("refs/heads/feature"),
            ahead: 0,
            behind: 0,
            is_gone: false,
        };
        let branch = Branch {
            name: String::from("feature"),
            commit: commit.clone(),
            upstream: is_tracked.then_some(upstream),
            is_checked_out: false,
            date: model::Timestamp(0),
        };
        let remote = RemoteBranch {
            name: String::from("origin/feature"),
            remote: String::from("origin"),
            commit,
            date: model::Timestamp(0),
        };
        let state = RepositoryState {
            branches: vec![branch],
            remote_branches: vec![remote.clone()],
            ..RepositoryState::default()
        };
        (state, remote)
    }

    #[test]
    fn graph_pull_request_badge__a_remote_branch_tracked_has_no_badge_of_its_own_and_untracked_has_one() {
        let on_github = GitHubRepo {
            owner: String::from("owner"),
            repo: String::from("app"),
        };
        let known = pull_requests();
        let github = GitHubShown {
            repository: Some(&on_github),
            hosting: None,
            pull_requests: Some(&known),
        };
        let (tracked, remote) = repository(true);
        assert_eq!(github.of_remote_branch(&tracked, &remote), None);
        let feature = tracked.branches.first().unwrap();
        assert_eq!(
            github.of_branch(feature).map(|pull_request| pull_request.number),
            Some(7)
        );
        let (untracked, remote) = repository(false);
        let number = github
            .of_remote_branch(&untracked, &remote)
            .map(|pull_request| pull_request.number);
        assert_eq!(number, Some(7));
    }
}
