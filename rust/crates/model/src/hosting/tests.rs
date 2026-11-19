#![allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]

use super::{
    HostingPage, HostingRepo, HostingService, KnownHosts, NamedSide, SelfHosted, SideKind, detect_remote, web_link,
};
use crate::compare::CompareMode;
use crate::remote::Remote;

fn remote(name: &str, url: &str) -> Remote {
    Remote {
        name: name.to_owned(),
        url: url.to_owned(),
    }
}

/// The service, the path and the address on the web of a URL of `origin`, with the hosts of one's own given.
fn of_with(url: &str, known: &KnownHosts) -> Option<(HostingService, String, String)> {
    detect_remote(&[remote("origin", url)], known).map(|repo| (repo.service, repo.path, repo.web))
}

fn of(url: &str) -> Option<(HostingService, String, String)> {
    of_with(url, &KnownHosts::default())
}

#[allow(clippy::unnecessary_wraps, reason = "it is compared with what detect_remote gives")]
fn found(service: HostingService, path: &str, web: &str) -> Option<(HostingService, String, String)> {
    Some((service, path.to_owned(), web.to_owned()))
}

/// A project on a service, at an address on the web.
pub(super) fn project(service: HostingService, web: &str) -> HostingRepo {
    HostingRepo {
        service,
        web: web.to_owned(),
        host: String::new(),
        path: String::new(),
        remote: String::from("origin"),
        url: String::new(),
        default_branch: None,
    }
}

#[test]
fn hosting_detect_remote__a_gitlab_url_gives_the_whole_path_of_the_project() {
    let expected = found(
        HostingService::GitLab,
        "group/sub/app",
        "https://gitlab.com/group/sub/app",
    );
    assert_eq!(of("git@gitlab.com:group/sub/app.git"), expected);
    assert_eq!(of("https://gitlab.com/group/sub/app"), expected);
    let repo = detect_remote(
        &[remote("origin", "git@gitlab.com:group/sub/app.git")],
        &KnownHosts::default(),
    );
    assert_eq!(
        repo.map(|repo| (repo.host, repo.remote)),
        Some((String::from("gitlab.com"), String::from("origin")))
    );
}

#[test]
fn hosting_detect_remote__bitbucket_urls_give_the_workspace_and_the_repository() {
    let expected = found(HostingService::Bitbucket, "team/app", "https://bitbucket.org/team/app");
    assert_eq!(of("https://bitbucket.org/team/app.git"), expected);
    assert_eq!(of("git@bitbucket.org:team/app.git"), expected);
    assert_eq!(of("https://user@bitbucket.org/team/app/"), expected);
}

#[test]
fn hosting_detect_remote__azure_devops_urls_give_the_page_of_the_repository_on_dev_azure_com() {
    let web = "https://dev.azure.com/org/Project/_git/app";
    let expected = found(HostingService::AzureDevOps, "org/Project/app", web);
    assert_eq!(of("https://org@dev.azure.com/org/Project/_git/app"), expected);
    assert_eq!(of("git@ssh.dev.azure.com:v3/org/Project/app"), expected);
    assert_eq!(of("https://org.visualstudio.com/Project/_git/app"), expected);
    assert_eq!(
        of("https://org.visualstudio.com/DefaultCollection/Project/_git/app"),
        expected
    );
    assert_eq!(of("https://dev.azure.com/org/Project/app"), None, "not a repository");
}

#[test]
fn hosting_detect_remote__codeberg_is_forgejo() {
    let expected = found(HostingService::Forgejo, "owner/app", "https://codeberg.org/owner/app");
    assert_eq!(of("https://codeberg.org/owner/app.git"), expected);
}

#[test]
fn hosting_detect_remote__a_host_of_ones_own_is_known_only_when_the_setting_names_it() {
    let url = "ssh://git@git.example.com:2222/team/app.git";
    assert_eq!(of(url), None);
    let mut known = KnownHosts::default();
    let gitlab = SelfHosted {
        service: HostingService::GitLab,
        web: None,
    };
    let _none = known.by_host.insert(String::from("git.example.com"), gitlab);
    let expected = found(HostingService::GitLab, "team/app", "https://git.example.com/team/app");
    assert_eq!(of_with(url, &known), expected, "the port of ssh is not that of the web");
    assert_eq!(
        of_with("https://git.example.com:8443/team/app.git", &known),
        found(
            HostingService::GitLab,
            "team/app",
            "https://git.example.com:8443/team/app"
        ),
        "the port of https is kept"
    );
    let gitea = SelfHosted {
        service: HostingService::Gitea,
        web: Some(String::from("https://code.example.com/")),
    };
    let _none = known.by_host.insert(String::from("ssh.example.com"), gitea);
    let repo = detect_remote(&[remote("origin", "git@ssh.example.com:team/app.git")], &known).unwrap();
    assert_eq!(
        (repo.service, repo.web.as_str()),
        (HostingService::Gitea, "https://code.example.com/team/app")
    );
    assert_eq!(repo.host, "code.example.com", "the API is asked where the pages are");
    assert_eq!(of("git@example.com:owner/app.git"), None);
    assert_eq!(of("C:\\repositories\\app"), None);
    assert_eq!(of("/srv/git/app.git"), None);
}

#[test]
fn hosting_detect_remote__github_gives_what_github_detect_remote_gives() {
    let expected = found(HostingService::GitHub, "owner/app", "https://github.com/owner/app");
    assert_eq!(of("https://github.com/owner/app.git"), expected);
    assert_eq!(of("git@github.com:owner/app.git"), expected);
    assert_eq!(of("https://github.com/owner"), None);
    let remotes = [
        remote("upstream", "https://gitlab.com/owner/app.git"),
        remote("origin", "https://example.com/owner/app.git"),
    ];
    assert_eq!(
        detect_remote(&remotes, &KnownHosts::default()),
        None,
        "origin is the main remote"
    );
}

#[test]
fn hosting_web_links__the_page_that_creates_a_merge_request_on_gitlab() {
    let repo = project(HostingService::GitLab, "https://gitlab.com/group/app");
    assert_eq!(
        web_link(&repo, &HostingPage::Create("feature/login")),
        "https://gitlab.com/group/app/-/merge_requests/new?merge_request%5Bsource_branch%5D=feature/login"
    );
    assert_eq!(
        web_link(&repo, &HostingPage::PullRequest(12)),
        "https://gitlab.com/group/app/-/merge_requests/12"
    );
}

#[test]
fn hosting_web_links__a_pull_request_on_azure_devops() {
    let repo = project(
        HostingService::AzureDevOps,
        "https://dev.azure.com/org/Project/_git/app",
    );
    assert_eq!(
        web_link(&repo, &HostingPage::PullRequest(7)),
        "https://dev.azure.com/org/Project/_git/app/pullrequest/7"
    );
    assert_eq!(
        web_link(&repo, &HostingPage::Create("feature")),
        "https://dev.azure.com/org/Project/_git/app/pullrequestcreate?sourceRef=feature"
    );
    let compared = HostingPage::Compare {
        first: NamedSide {
            name: "main",
            kind: SideKind::Branch,
        },
        second: NamedSide {
            name: "v1.0",
            kind: SideKind::Tag,
        },
        mode: CompareMode::SinceDiverged,
    };
    assert_eq!(
        web_link(&repo, &compared),
        "https://dev.azure.com/org/Project/_git/app/branchCompare?baseVersion=GBmain&targetVersion=GTv1.0"
    );
}

#[test]
fn hosting_web_links__bitbucket_compares_since_the_two_diverged() {
    let repo = project(HostingService::Bitbucket, "https://bitbucket.org/team/app");
    let branch = |name| NamedSide {
        name,
        kind: SideKind::Branch,
    };
    let compared = HostingPage::Compare {
        first: branch("main"),
        second: branch("feature"),
        mode: CompareMode::SinceDiverged,
    };
    assert_eq!(
        web_link(&repo, &compared),
        "https://bitbucket.org/team/app/branches/compare/feature%0Dmain"
    );
    assert_eq!(
        web_link(&repo, &HostingPage::Create("feature")),
        "https://bitbucket.org/team/app/pull-requests/new?source=feature"
    );
    assert_eq!(
        web_link(&repo, &HostingPage::PullRequest(4)),
        "https://bitbucket.org/team/app/pull-requests/4"
    );
}

#[test]
fn hosting_web_links__a_branch_named_fix_hash_1_is_encoded_on_every_service() {
    let services = [
        (
            HostingService::GitHub,
            "https://github.com/owner/app/compare/fix%231?expand=1",
        ),
        (
            HostingService::GitLab,
            "https://example.com/p/-/merge_requests/new?merge_request%5Bsource_branch%5D=fix%231",
        ),
        (
            HostingService::Bitbucket,
            "https://example.com/p/pull-requests/new?source=fix%231",
        ),
        (
            HostingService::AzureDevOps,
            "https://example.com/p/pullrequestcreate?sourceRef=fix%231",
        ),
        (HostingService::Gitea, "https://example.com/p/compare/main...fix%231"),
        (
            HostingService::Forgejo,
            "https://example.com/p/compare/develop...fix%231",
        ),
    ];
    for (service, expected) in services {
        let mut repo = project(service, "https://example.com/p");
        if service == HostingService::GitHub {
            repo.web = String::from("https://github.com/owner/app");
        }
        if service == HostingService::Forgejo {
            repo.default_branch = Some(String::from("develop"));
        }
        assert_eq!(web_link(&repo, &HostingPage::Create("fix#1")), expected, "{service:?}");
    }
}
