#![allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]

use http::test_server::{Answer, TestServer, answer};
use http::{Cancel, Http, Proxies};
use model::github::{Checks, PullRequests, Token};
use model::hosting::{HostingRepo, HostingService};

use super::{MergeRequestsRequest, parse_merge_requests, read_merge_requests};

/// A merge request of an answer of GitLab, as the query asks for it, from the project 1 or from a fork, project 2.
pub(super) fn node(number: u64, branch: &str, is_from_fork: bool, status: Option<&str>) -> String {
    let pipeline = status.map_or_else(
        || String::from("null"),
        |status| format!("{{\"status\": \"{status}\"}}"),
    );
    let source = if is_from_fork { 2 } else { 1 };
    format!(
        "{{\"iid\": \"{number}\", \"title\": \"Change {number}\", \
         \"webUrl\": \"https://gitlab.com/group/app/-/merge_requests/{number}\", \"draft\": false, \
         \"sourceBranch\": \"{branch}\", \"sourceProjectId\": {source}, \"targetProjectId\": 1, \
         \"headPipeline\": {pipeline}, \"author\": {{\"username\": \"author{number}\"}}, \
         \"sourceProject\": {{\"fullPath\": \"{}\"}}}}",
        if is_from_fork { "contributor/app" } else { "group/app" }
    )
}

pub(super) fn answer_of(nodes: &[String]) -> String {
    format!(
        "{{\"data\": {{\"project\": {{\"mergeRequests\": {{\"nodes\": [{}]}}}}}}}}",
        nodes.join(", ")
    )
}

fn parsed(nodes: &[String]) -> PullRequests {
    parse_merge_requests(answer_of(nodes).as_bytes()).unwrap()
}

pub(super) fn request() -> MergeRequestsRequest {
    MergeRequestsRequest {
        repository: HostingRepo {
            service: HostingService::GitLab,
            web: String::from("https://example.test/group/app"),
            host: String::from("example.test"),
            path: String::from("group/app"),
            remote: String::from("origin"),
            url: String::from("git@example.test:group/app.git"),
            default_branch: None,
        },
        token: Token(String::from("glpat-secret")),
        cancel: Cancel::new(),
    }
}

/// What reaches `example.test`, a GitLab of one's own whose name the certificate of the test server has, goes to the
/// server of the test, which answers for it over TLS; nothing else is reached.
pub(super) fn http_to(server: &TestServer) -> Http {
    Http::new(TestServer::trust(), Proxies::none()).with_address("example.test", 443, server.address())
}

#[test]
fn hosting_read_merge_requests__one_merge_request_per_branch_and_none_from_a_fork() {
    let merge_requests = parsed(&[
        node(3, "feature", false, Some("SUCCESS")),
        node(4, "fix", true, Some("SUCCESS")),
        node(5, "feature", false, Some("FAILED")),
    ]);
    assert_eq!(merge_requests.by_branch.len(), 1);
    let feature = merge_requests.of_branch("feature").unwrap();
    assert_eq!((feature.number, feature.checks), (3, Some(Checks::Passed)));
    assert_eq!(feature.url, "https://gitlab.com/group/app/-/merge_requests/3");
    assert_eq!(merge_requests.of_branch("fix"), None);
}

#[test]
fn hosting_read_merge_requests__the_states_of_the_pipeline_and_a_draft() {
    let draft = node(8, "draft", false, None).replace("\"draft\": false", "\"draft\": true");
    let merge_requests = parsed(&[
        node(1, "pending", false, Some("PENDING")),
        node(2, "canceled", false, Some("CANCELED")),
        node(3, "failed", false, Some("FAILED")),
        node(4, "running", false, Some("RUNNING")),
        draft,
    ]);
    let checks = |branch: &str| merge_requests.of_branch(branch).unwrap().checks;
    assert_eq!(checks("pending"), Some(Checks::Running));
    assert_eq!(checks("canceled"), None);
    assert_eq!(checks("failed"), Some(Checks::Failed));
    assert_eq!(checks("running"), Some(Checks::Running));
    assert!(merge_requests.of_branch("draft").unwrap().is_draft);
    assert!(!merge_requests.of_branch("failed").unwrap().is_draft);
}

#[test]
fn hosting_read_merge_requests__a_refused_token_is_said_and_told_apart() {
    let server = TestServer::secure(vec![
        Answer::Bytes(answer("401 Unauthorized", &[], b"{}")),
        Answer::Bytes(answer(
            "200 OK",
            &[],
            br#"{"errors":[{"message":"Field 'project' is missing"}]}"#,
        )),
    ]);
    let refused = read_merge_requests(&http_to(&server), &request()).unwrap_err();
    assert!(refused.is_unauthorized());
    assert_eq!(refused.to_string(), "GitLab answered 401 Unauthorized");
    let error = read_merge_requests(&http_to(&server), &request()).unwrap_err();
    assert_eq!(error.to_string(), "Field 'project' is missing");
}

#[test]
fn hosting_read_merge_requests__one_post_to_the_gitlab_of_the_main_remote_with_the_token_in_a_header_only() {
    let body = answer_of(&[node(3, "feature", false, Some("SUCCESS"))]);
    let server = TestServer::secure(vec![Answer::Bytes(answer("200 OK", &[], body.as_bytes()))]);
    let merge_requests = read_merge_requests(&http_to(&server), &request()).unwrap();
    assert_eq!(merge_requests.of_branch("feature").unwrap().number, 3);
    let received = server.received();
    assert_eq!(received.len(), 1);
    let sent = received.first().unwrap();
    assert_eq!(sent.request_line(), "POST /api/graphql HTTP/1.1");
    assert_eq!(sent.header("host").as_deref(), Some("example.test"));
    assert_eq!(sent.header("authorization").as_deref(), Some("Bearer glpat-secret"));
    let body = json::parse(std::str::from_utf8(&sent.body).unwrap()).unwrap();
    let query = body.get("query").and_then(json::Value::as_str).unwrap();
    assert!(
        query.contains("mergeRequests(state: opened, first: 100, sort: UPDATED_DESC)"),
        "{query}"
    );
    let variables = body.get("variables").unwrap();
    assert_eq!(variables.get("path").and_then(json::Value::as_str), Some("group/app"));
    assert!(
        !format!("{:?}", request()).contains("glpat-secret"),
        "the token is never shown"
    );
    let mut own = request();
    own.repository.web = String::from("https://git.example.com:8443/gitlab/group/app");
    own.repository.host = String::from("git.example.com");
    assert_eq!(
        own.repository.site(),
        "https://git.example.com:8443/gitlab",
        "the API of a GitLab of one's own"
    );
}
