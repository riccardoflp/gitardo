//! The address that opens the pull request of a branch, or GitHub's page to create one (`github.open-pull-request`).

use std::fmt::Write as _;

use super::GitHubRepo;
use super::pulls::PullRequests;
use crate::contract::Text;
use crate::state::Upstream;

/// The branch whose pull request is looked for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BranchOf<'a> {
    /// A local branch, with the branch it follows on a remote.
    Local {
        /// Its name: `feature/login`.
        name: &'a str,
        /// Its upstream, when it has one.
        upstream: Option<&'a Upstream>,
    },
    /// A remote branch.
    Remote {
        /// Its name without its remote: `feature` for `origin/feature`.
        name: &'a str,
    },
}

/// What the link of a branch is worked out from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PullLinkInput<'a> {
    /// The repository on GitHub of the main remote; none when it is not on GitHub.
    pub repository: Option<&'a GitHubRepo>,
    /// The branch.
    pub branch: BranchOf<'a>,
    /// The open pull requests known; none known before they are loaded.
    pub pull_requests: &'a PullRequests,
}

/// What the link does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PullLinkKind {
    /// Opens the pull request of that number.
    Open(u64),
    /// Opens GitHub's page to create one.
    Create,
}

/// The entry of a menu that opens a pull request, or the page to create one, in the browser.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PullLink {
    /// What it does.
    pub kind: PullLinkKind,
    /// The words of the entry: **Open pull request #12**, **Create pull request…**.
    pub label: Text,
    /// The address it opens.
    pub url: String,
}

/// The entry that opens the pull request of a branch when one is known, or GitHub's page to create one for a local
/// branch whose upstream is there; none when the main remote is not on GitHub, a local branch has no upstream or one
/// that is gone, or a remote branch has no pull request known.
///
/// Feature: github.open-pull-request
#[must_use]
pub fn pull_request_link(input: &PullLinkInput<'_>) -> Option<PullLink> {
    let repository = input.repository?;
    let (name, may_create) = match input.branch {
        BranchOf::Local { upstream, .. } => {
            let upstream = upstream.filter(|upstream| !upstream.is_gone)?;
            let name = upstream
                .remote_branch
                .strip_prefix("refs/heads/")
                .unwrap_or(&upstream.remote_branch);
            (name, true)
        }
        BranchOf::Remote { name } => (name, false),
    };
    if let Some(pull_request) = input.pull_requests.of_branch(name) {
        return Some(PullLink {
            kind: PullLinkKind::Open(pull_request.number),
            label: Text::from(format!("Open pull request #{}", pull_request.number)),
            url: pull_request.url.clone(),
        });
    }
    may_create.then(|| PullLink {
        kind: PullLinkKind::Create,
        label: Text::from("Create pull request\u{2026}"),
        url: compare_url(repository, name),
    })
}

/// GitHub's page to open a pull request of a branch: `https://github.com/<owner>/<repo>/compare/<branch>?expand=1`,
/// each part of the branch name between `/` percent-encoded and the `/` kept.
pub(crate) fn compare_url(repository: &GitHubRepo, branch: &str) -> String {
    let branch: Vec<String> = branch.split('/').map(percent_encoded).collect();
    format!(
        "https://github.com/{}/{}/compare/{}?expand=1",
        repository.owner,
        repository.repo,
        branch.join("/")
    )
}

/// The two sides of a comparison as GitHub knows them, and how they are compared.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ComparedOnGitHub<'a> {
    /// The first side: a branch on GitHub, a tag, or a full hash.
    pub first: &'a str,
    /// The second side.
    pub second: &'a str,
    /// **Since they diverged** (`...`) or **Everything that differs** (`..`).
    pub mode: crate::compare::CompareMode,
}

/// GitHub's page of a comparison: `https://github.com/<owner>/<repo>/compare/<first>...<second>`, with `..` in place
/// of `...` for **Everything that differs**; each part of a name between `/` percent-encoded and the `/` kept
/// (`compare.open-on-github`).
#[must_use]
pub fn comparison_url(repository: &GitHubRepo, compared: &ComparedOnGitHub<'_>) -> String {
    let encoded = |name: &str| -> String { name.split('/').map(percent_encoded).collect::<Vec<_>>().join("/") };
    let separator = match compared.mode {
        crate::compare::CompareMode::SinceDiverged => "...",
        crate::compare::CompareMode::Everything => "..",
    };
    format!(
        "https://github.com/{}/{}/compare/{}{separator}{}",
        repository.owner,
        repository.repo,
        encoded(compared.first),
        encoded(compared.second)
    )
}

/// A part of an address with every byte but letters, digits, `-`, `.`, `_` and `~` written `%XX`.
fn percent_encoded(part: &str) -> String {
    let mut encoded = String::with_capacity(part.len());
    for byte in part.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~') {
            encoded.push(char::from(byte));
        } else {
            let _ = write!(encoded, "%{byte:02X}");
        }
    }
    encoded
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests {
    use super::{BranchOf, PullLink, PullLinkInput, PullLinkKind, pull_request_link};
    use crate::contract::Text;
    use crate::github::{GitHubRepo, PullRequest, PullRequests};
    use crate::state::Upstream;

    fn repository() -> GitHubRepo {
        GitHubRepo {
            owner: String::from("owner"),
            repo: String::from("app"),
        }
    }

    fn upstream(name: &str, is_gone: bool) -> Upstream {
        Upstream {
            reference: format!("refs/remotes/origin/{name}"),
            remote: String::from("origin"),
            remote_branch: format!("refs/heads/{name}"),
            ahead: 0,
            behind: 0,
            is_gone,
        }
    }

    fn known(number: u64, branch: &str) -> PullRequests {
        let mut pull_requests = PullRequests::default();
        let _ = pull_requests.by_branch.insert(
            branch.to_owned(),
            PullRequest {
                number,
                title: String::from("A change"),
                url: format!("https://github.com/owner/app/pull/{number}"),
                is_draft: false,
                branch: branch.to_owned(),
                checks: None,
            },
        );
        pull_requests
    }

    fn link(branch: BranchOf<'_>, pull_requests: &PullRequests) -> Option<PullLink> {
        let repository = repository();
        pull_request_link(&PullLinkInput {
            repository: Some(&repository),
            branch,
            pull_requests,
        })
    }

    fn create(url: &str) -> PullLink {
        PullLink {
            kind: PullLinkKind::Create,
            label: Text::from("Create pull request\u{2026}"),
            url: url.to_owned(),
        }
    }

    #[test]
    fn github_open_pull_request__a_branch_with_a_pull_request_opens_it() {
        let upstream = upstream("feature", false);
        let branch = BranchOf::Local {
            name: "feature",
            upstream: Some(&upstream),
        };
        assert_eq!(
            link(branch, &known(12, "feature")),
            Some(PullLink {
                kind: PullLinkKind::Open(12),
                label: Text::from("Open pull request #12"),
                url: String::from("https://github.com/owner/app/pull/12"),
            })
        );
    }

    #[test]
    fn github_open_pull_request__a_branch_with_none_opens_the_page_to_create_one() {
        let upstream = upstream("feature/login", false);
        let branch = BranchOf::Local {
            name: "feature/login",
            upstream: Some(&upstream),
        };
        assert_eq!(
            link(branch, &known(3, "other")),
            Some(create("https://github.com/owner/app/compare/feature/login?expand=1"))
        );
    }

    #[test]
    fn github_open_pull_request__each_part_of_the_name_is_percent_encoded() {
        let upstream = upstream("fix#1", false);
        let branch = BranchOf::Local {
            name: "fix#1",
            upstream: Some(&upstream),
        };
        assert_eq!(
            link(branch, &PullRequests::default()),
            Some(create("https://github.com/owner/app/compare/fix%231?expand=1"))
        );
        let accented = super::tests::upstream("team/caf\u{e9} 2", false);
        let branch = BranchOf::Local {
            name: "x",
            upstream: Some(&accented),
        };
        assert_eq!(
            link(branch, &PullRequests::default()),
            Some(create(
                "https://github.com/owner/app/compare/team/caf%C3%A9%202?expand=1"
            ))
        );
    }

    #[test]
    fn github_open_pull_request__a_branch_without_an_upstream_or_whose_upstream_is_gone_has_no_entry() {
        let no_upstream = BranchOf::Local {
            name: "feature",
            upstream: None,
        };
        assert_eq!(link(no_upstream, &known(12, "feature")), None);
        let gone = upstream("feature", true);
        let gone = BranchOf::Local {
            name: "feature",
            upstream: Some(&gone),
        };
        assert_eq!(link(gone, &known(12, "feature")), None);
    }

    #[test]
    fn github_open_pull_request__a_remote_branch_has_an_entry_only_with_a_pull_request() {
        let branch = BranchOf::Remote { name: "feature" };
        assert_eq!(link(branch, &PullRequests::default()), None);
        let opened = link(branch, &known(7, "feature")).unwrap();
        assert_eq!(
            (opened.kind, opened.label),
            (PullLinkKind::Open(7), Text::from("Open pull request #7"))
        );
    }

    #[test]
    fn github_open_pull_request__before_the_pull_requests_are_loaded_a_pushed_branch_can_create_one() {
        let upstream = upstream("feature", false);
        let branch = BranchOf::Local {
            name: "feature",
            upstream: Some(&upstream),
        };
        assert_eq!(
            link(branch, &PullRequests::default()),
            Some(create("https://github.com/owner/app/compare/feature?expand=1"))
        );
        assert_eq!(
            pull_request_link(&PullLinkInput {
                repository: None,
                branch,
                pull_requests: &PullRequests::default(),
            }),
            None,
            "nothing when the main remote is not on GitHub"
        );
    }
}
