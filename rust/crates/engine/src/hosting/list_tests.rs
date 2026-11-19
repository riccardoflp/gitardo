#![allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]

use http::test_server::{Answer, TestServer, answer};
use http::{Cancel, Http, Proxies};
use model::github::Token;
use model::hosting::{HostingRepo, HostingService, PullListEntry};

use super::list::{PullListRequest, read_pull_request_list};
use super::tests::{answer_of, node};

/// A request for the project `owner/app` on a service, at a host name.
fn request(service: HostingService, host: &str) -> PullListRequest {
    PullListRequest {
        repository: HostingRepo {
            service,
            web: format!("https://{host}/owner/app"),
            host: host.to_owned(),
            path: String::from("owner/app"),
            remote: String::from("origin"),
            url: format!("https://{host}/owner/app.git"),
            default_branch: None,
        },
        token: Token(String::from("secret")),
        cancel: Cancel::new(),
    }
}

/// What reaches a host name goes to the server of the test, which answers for it over TLS.
fn http_to(server: &TestServer, host: &str) -> Http {
    Http::new(TestServer::trust(), Proxies::none()).with_address(host, 443, server.address())
}

#[test]
fn hosting_read_pull_request_list__on_github_a_pull_request_of_the_repository_and_one_from_a_fork() {
    let body = "{\"data\": {\"repository\": {\"pullRequests\": {\"nodes\": [\
        {\"number\": 4, \"title\": \"Fix the parser\", \"headRefName\": \"fix-parser\", \"isCrossRepository\": true, \
         \"author\": {\"login\": \"contributor\"}, \"headRepository\": {\"nameWithOwner\": \"contributor/app\"}}, \
        {\"number\": 3, \"title\": \"Add a feature\", \"headRefName\": \"feature\", \"isCrossRepository\": false, \
         \"author\": {\"login\": \"ada\"}, \"headRepository\": {\"nameWithOwner\": \"owner/app\"}}]}}}}";
    let server = TestServer::secure(vec![Answer::Bytes(answer("200 OK", &[], body.as_bytes()))]);
    let listed = read_pull_request_list(
        &http_to(&server, "api.github.com"),
        &request(HostingService::GitHub, "github.com"),
    )
    .unwrap();
    let fork = PullListEntry {
        number: 4,
        title: String::from("Fix the parser"),
        author: String::from("contributor"),
        branch: String::from("fix-parser"),
        fork: Some(String::from("contributor/app")),
    };
    assert_eq!(listed.first(), Some(&fork));
    assert_eq!(fork.fork_owner(), Some("contributor"));
    assert_eq!(
        listed.get(1).map(|entry| (entry.number, entry.fork.clone())),
        Some((3, None))
    );
    let received = server.received();
    assert_eq!(received.len(), 1);
    assert_eq!(received[0].header("host").as_deref(), Some("api.github.com"));
    assert_eq!(received[0].header("authorization").as_deref(), Some("Bearer secret"));
}

#[test]
fn hosting_read_pull_request_list__on_gitlab_each_merge_request_with_its_author_and_its_source_project() {
    let body = answer_of(&[node(7, "fix", true, None), node(3, "feature", false, Some("SUCCESS"))]);
    let server = TestServer::secure(vec![Answer::Bytes(answer("200 OK", &[], body.as_bytes()))]);
    let listed = read_pull_request_list(
        &http_to(&server, "example.test"),
        &request(HostingService::GitLab, "example.test"),
    )
    .unwrap();
    let summary: Vec<_> = listed
        .iter()
        .map(|entry| {
            (
                entry.number,
                entry.author.as_str(),
                entry.branch.as_str(),
                entry.fork.as_deref(),
            )
        })
        .collect();
    assert_eq!(
        summary,
        [
            (7, "author7", "fix", Some("contributor/app")),
            (3, "author3", "feature", None)
        ]
    );
    let received = server.received();
    assert_eq!(received[0].request_line(), "POST /api/graphql HTTP/1.1");
    assert!(String::from_utf8_lossy(&received[0].body).contains("sourceProject { fullPath }"));
}

#[test]
fn hosting_read_pull_request_list__on_bitbucket_it_reads_nothing_and_gives_an_empty_list() {
    let server = TestServer::secure(Vec::new());
    for service in [
        HostingService::Bitbucket,
        HostingService::AzureDevOps,
        HostingService::Gitea,
    ] {
        let listed = read_pull_request_list(&http_to(&server, "example.test"), &request(service, "example.test"));
        assert_eq!(listed.unwrap(), []);
    }
    assert_eq!(server.received().len(), 0, "nothing is sent");
}
