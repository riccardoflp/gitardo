//! GitHub: the repository on GitHub that the main remote names, if it names one (`github.detect-remote`), its open
//! pull requests, the link that opens the pull request of a branch (`github.open-pull-request`), and the offer to
//! create one after the first push of a branch (`github.offer-after-push`).

pub(crate) mod link;
mod offer;
mod pulls;

pub use link::{BranchOf, ComparedOnGitHub, PullLink, PullLinkInput, PullLinkKind, comparison_url, pull_request_link};
pub use offer::{Notice, PushOutcome, offer_after_push};
pub use pulls::{Checks, HelperAsked, PullRequest, PullRequests, Token};

use crate::remote::Remote;

/// A repository on github.com.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GitHubRepo {
    /// Its owner: a user or an organization.
    pub owner: String,
    /// Its name, without `.git`.
    pub repo: String,
}

/// The repository on github.com of the main remote: `origin` when there is one, otherwise the first remote listed.
/// Nothing when that remote is on another host.
///
/// Feature: github.detect-remote
#[must_use]
pub fn detect_remote(remotes: &[Remote]) -> Option<GitHubRepo> {
    on_github(main_remote(remotes)?.url.trim())
}

/// The main remote of a repository: `origin` when there is one, otherwise the first remote listed.
#[must_use]
pub fn main_remote(remotes: &[Remote]) -> Option<&Remote> {
    remotes
        .iter()
        .find(|remote| remote.name == "origin")
        .or_else(|| remotes.first())
}

/// The owner and the repository of a URL on github.com: `https://github.com/<owner>/<repo>`, with `http`, with a user
/// before the host, `ssh://git@github.com/<owner>/<repo>` or `git@github.com:<owner>/<repo>`, each with or without
/// `.git` and a `/` at the end.
fn on_github(url: &str) -> Option<GitHubRepo> {
    let path = if let Some(rest) = url.strip_prefix("git@github.com:") {
        rest
    } else {
        let rest = url
            .strip_prefix("https://")
            .or_else(|| url.strip_prefix("http://"))
            .or_else(|| url.strip_prefix("ssh://"))?;
        let (authority, path) = rest.split_once('/')?;
        let host = authority.rsplit_once('@').map_or(authority, |(_, host)| host);
        if !host.eq_ignore_ascii_case("github.com") {
            return None;
        }
        path
    };
    let path = path.strip_suffix('/').unwrap_or(path);
    let path = path.strip_suffix(".git").unwrap_or(path);
    let (owner, repo) = path.split_once('/')?;
    let is_name = |part: &str| !part.is_empty() && !part.contains('/');
    (is_name(owner) && is_name(repo)).then(|| GitHubRepo {
        owner: owner.to_owned(),
        repo: repo.to_owned(),
    })
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests {
    use super::{GitHubRepo, detect_remote};
    use crate::remote::Remote;

    fn remote(name: &str, url: &str) -> Remote {
        Remote {
            name: name.to_owned(),
            url: url.to_owned(),
        }
    }

    fn of(url: &str) -> Option<GitHubRepo> {
        detect_remote(&[remote("origin", url)])
    }

    fn app() -> GitHubRepo {
        GitHubRepo {
            owner: String::from("owner"),
            repo: String::from("app"),
        }
    }

    #[test]
    fn github_detect_remote__https_scp_and_ssh_urls_give_the_owner_and_the_repository() {
        for url in [
            "https://github.com/owner/app.git",
            "git@github.com:owner/app.git",
            "ssh://git@github.com/owner/app",
            " http://github.com/owner/app ",
        ] {
            assert_eq!(of(url), Some(app()), "{url}");
        }
    }

    #[test]
    fn github_detect_remote__a_user_before_the_host_and_a_slash_at_the_end() {
        assert_eq!(of("https://user@github.com/owner/app/"), Some(app()));
    }

    #[test]
    fn github_detect_remote__another_host_gives_nothing() {
        for url in [
            "https://gitlab.com/owner/app.git",
            "git@example.com:owner/app.git",
            "https://github.example.com/owner/app.git",
            "https://github.com/owner",
            "/srv/git/app.git",
        ] {
            assert_eq!(of(url), None, "{url}");
        }
    }

    #[test]
    fn github_detect_remote__origin_is_the_main_remote_even_when_another_is_on_github() {
        let remotes = [
            remote("upstream", "https://github.com/owner/app.git"),
            remote("origin", "https://example.com/owner/app.git"),
        ];
        assert_eq!(detect_remote(&remotes), None);
    }

    #[test]
    fn github_detect_remote__with_only_a_fork_on_github_its_repository_is_the_result() {
        assert_eq!(
            detect_remote(&[remote("fork", "git@github.com:owner/app.git")]),
            Some(app())
        );
        assert_eq!(detect_remote(&[]), None);
    }

    #[test]
    fn github_detect_remote__renaming_the_github_remote_away_from_origin_takes_the_repository_away() {
        let before = [
            remote("origin", "https://github.com/owner/app.git"),
            remote("mirror", "https://example.com/owner/app.git"),
        ];
        assert_eq!(detect_remote(&before), Some(app()));
        let after = [
            remote("github", "https://github.com/owner/app.git"),
            remote("origin", "https://example.com/owner/app.git"),
        ];
        assert_eq!(detect_remote(&after), None);
    }
}
