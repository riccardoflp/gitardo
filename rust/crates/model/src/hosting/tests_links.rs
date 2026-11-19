#![allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]

use super::tests::project;
use super::{
    HostingPullLinkInput, HostingPushOutcome, HostingRepo, HostingService, offer_after_push, pull_request_link,
};
use crate::contract::Text;
use crate::github::{BranchOf, Notice, PullLink, PullLinkKind, PullRequest, PullRequests};
use crate::state::Upstream;

fn gitlab() -> HostingRepo {
    project(HostingService::GitLab, "https://gitlab.com/group/app")
}

fn upstream(remote: &str, name: &str) -> Upstream {
    Upstream {
        reference: format!("refs/remotes/{remote}/{name}"),
        remote: remote.to_owned(),
        remote_branch: format!("refs/heads/{name}"),
        ahead: 0,
        behind: 0,
        is_gone: false,
    }
}

/// Merge request `number` of `branch`.
fn known(number: u64, branch: &str) -> PullRequests {
    let mut pull_requests = PullRequests::default();
    let _none = pull_requests.by_branch.insert(
        branch.to_owned(),
        PullRequest {
            number,
            title: String::from("A change"),
            url: format!("https://gitlab.com/group/app/-/merge_requests/{number}"),
            is_draft: false,
            branch: branch.to_owned(),
            checks: None,
        },
    );
    pull_requests
}

fn link(repository: &HostingRepo, branch: BranchOf<'_>, pull_requests: &PullRequests) -> Option<PullLink> {
    pull_request_link(&HostingPullLinkInput {
        repository: Some(repository),
        branch,
        pull_requests,
    })
}

#[test]
fn hosting_open_pull_request__a_branch_with_a_merge_request_on_gitlab_opens_it() {
    let upstream = upstream("origin", "feature");
    let branch = BranchOf::Local {
        name: "feature",
        upstream: Some(&upstream),
    };
    assert_eq!(
        link(&gitlab(), branch, &known(12, "feature")),
        Some(PullLink {
            kind: PullLinkKind::Open(12),
            label: Text::from("Open merge request !12"),
            url: String::from("https://gitlab.com/group/app/-/merge_requests/12"),
        })
    );
    let remote = BranchOf::Remote { name: "feature" };
    let opened = link(&gitlab(), remote, &known(12, "feature")).map(|link| link.kind);
    assert_eq!(opened, Some(PullLinkKind::Open(12)));
    assert_eq!(link(&gitlab(), remote, &PullRequests::default()), None);
}

#[test]
fn hosting_open_pull_request__a_branch_with_none_on_gitlab_creates_a_merge_request() {
    let upstream = upstream("origin", "feature");
    let branch = BranchOf::Local {
        name: "local-name",
        upstream: Some(&upstream),
    };
    assert_eq!(
        link(&gitlab(), branch, &PullRequests::default()),
        Some(PullLink {
            kind: PullLinkKind::Create,
            label: Text::from("Create merge request\u{2026}"),
            url: String::from(
                "https://gitlab.com/group/app/-/merge_requests/new?merge_request%5Bsource_branch%5D=feature"
            ),
        })
    );
}

#[test]
fn hosting_open_pull_request__on_bitbucket_a_pushed_branch_creates_a_pull_request_and_opens_none() {
    let bitbucket = project(HostingService::Bitbucket, "https://bitbucket.org/team/app");
    let upstream = upstream("origin", "feature");
    let branch = BranchOf::Local {
        name: "feature",
        upstream: Some(&upstream),
    };
    let created = link(&bitbucket, branch, &PullRequests::default()).unwrap();
    assert_eq!(created.label, Text::from("Create pull request\u{2026}"));
    assert_eq!(
        created.url,
        "https://bitbucket.org/team/app/pull-requests/new?source=feature"
    );
    assert_eq!(
        link(
            &bitbucket,
            BranchOf::Remote { name: "feature" },
            &PullRequests::default()
        ),
        None
    );
    let github = project(HostingService::GitHub, "https://github.com/owner/app");
    assert_eq!(
        link(&github, branch, &PullRequests::default()),
        None,
        "github.open-pull-request makes it"
    );
}

#[test]
fn hosting_open_pull_request__a_branch_with_no_upstream_has_no_entry_on_any_service() {
    let services = [
        HostingService::GitLab,
        HostingService::Bitbucket,
        HostingService::AzureDevOps,
        HostingService::Gitea,
        HostingService::Forgejo,
    ];
    let mut gone = upstream("origin", "feature");
    gone.is_gone = true;
    let elsewhere = upstream("fork", "feature");
    for service in services {
        let repository = project(service, "https://example.com/p");
        for upstream in [None, Some(&gone), Some(&elsewhere)] {
            let branch = BranchOf::Local {
                name: "feature",
                upstream,
            };
            assert_eq!(link(&repository, branch, &PullRequests::default()), None, "{service:?}");
        }
    }
}

fn pushed<'a>(repository: &'a HostingRepo, pull_requests: &'a PullRequests) -> HostingPushOutcome<'a> {
    HostingPushOutcome {
        succeeded: true,
        branch: Some("feature"),
        had_upstream: false,
        repository: Some(repository),
        pull_requests,
    }
}

#[test]
fn hosting_offer_after_push__the_first_push_on_gitlab_offers_to_create_a_merge_request() {
    let repository = gitlab();
    let none = PullRequests::default();
    assert_eq!(
        offer_after_push(&pushed(&repository, &none)),
        Some(Notice {
            message: Text::from("Pushed feature."),
            button: Text::from("Create merge request"),
            url: String::from(
                "https://gitlab.com/group/app/-/merge_requests/new?merge_request%5Bsource_branch%5D=feature"
            ),
        })
    );
    let azure = project(HostingService::AzureDevOps, "https://dev.azure.com/org/P/_git/app");
    let offered = offer_after_push(&pushed(&azure, &none)).map(|notice| notice.button);
    assert_eq!(offered, Some(Text::from("Create pull request")));
}

#[test]
fn hosting_offer_after_push__a_second_push_offers_nothing() {
    let repository = gitlab();
    let none = PullRequests::default();
    let second = HostingPushOutcome {
        had_upstream: true,
        ..pushed(&repository, &none)
    };
    assert_eq!(offer_after_push(&second), None);
    let failed = HostingPushOutcome {
        succeeded: false,
        ..pushed(&repository, &none)
    };
    assert_eq!(offer_after_push(&failed), None);
    let github = project(HostingService::GitHub, "https://github.com/owner/app");
    assert_eq!(
        offer_after_push(&pushed(&github, &none)),
        None,
        "github.offer-after-push offers it"
    );
}

#[test]
fn hosting_offer_after_push__with_a_merge_request_known_the_first_push_offers_nothing() {
    let repository = gitlab();
    let known = known(3, "feature");
    assert_eq!(offer_after_push(&pushed(&repository, &known)), None);
}
