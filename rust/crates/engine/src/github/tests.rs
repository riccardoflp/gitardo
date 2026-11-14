#![allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]

use http::test_server::{Answer, TestServer, answer};
use http::{Cancel, Http, Proxies};
use model::github::{Checks, GitHubRepo, PullRequests, Token};

use super::{GitHubError, PullsRequest, parse_pull_requests, read_pull_requests};

/// A pull request of an answer of GitHub, as the query asks for it.
fn node(number: u64, branch: &str, owner: &str, state: Option<&str>) -> String {
    let rollup = state.map_or_else(|| String::from("null"), |state| format!("{{\"state\": \"{state}\"}}"));
    format!(
        "{{\"number\": {number}, \"title\": \"Change {number}\", \"url\": \"https://github.com/owner/app/pull/{number}\", \
         \"isDraft\": false, \"headRefName\": \"{branch}\", \"headRepositoryOwner\": {{\"login\": \"{owner}\"}}, \
         \"commits\": {{\"nodes\": [{{\"commit\": {{\"statusCheckRollup\": {rollup}}}}}]}}}}"
    )
}

fn answer_of(nodes: &[String]) -> String {
    format!(
        "{{\"data\": {{\"repository\": {{\"pullRequests\": {{\"nodes\": [{}]}}}}}}}}",
        nodes.join(", ")
    )
}

fn parsed(nodes: &[String]) -> PullRequests {
    parse_pull_requests(answer_of(nodes).as_bytes(), "owner").unwrap()
}

fn request() -> PullsRequest {
    PullsRequest {
        repository: GitHubRepo {
            owner: String::from("owner"),
            repo: String::from("app"),
        },
        token: Token(String::from("ghp_secret")),
        cancel: Cancel::new(),
    }
}

/// What reaches `api.github.com` goes to the server of the test, which answers for it over TLS.
fn http_to(server: &TestServer) -> Http {
    Http::new(TestServer::trust(), Proxies::none()).with_address("api.github.com", 443, server.address())
}

#[test]
fn github_read_pull_requests__one_pull_request_per_branch_and_none_from_a_fork() {
    let pull_requests = parsed(&[
        node(3, "feature", "owner", Some("SUCCESS")),
        node(4, "fix", "someone-else", Some("SUCCESS")),
        node(5, "feature", "Owner", Some("FAILURE")),
    ]);
    assert_eq!(pull_requests.by_branch.len(), 1);
    let feature = pull_requests.of_branch("feature").unwrap();
    assert_eq!((feature.number, feature.checks), (3, Some(Checks::Passed)));
    assert_eq!(pull_requests.of_branch("fix"), None);
}

#[test]
fn github_read_pull_requests__the_states_of_the_checks() {
    let pull_requests = parsed(&[
        node(1, "error", "owner", Some("ERROR")),
        node(2, "expected", "owner", Some("EXPECTED")),
        node(3, "none", "owner", None),
        node(4, "failure", "owner", Some("FAILURE")),
        node(5, "pending", "owner", Some("PENDING")),
    ]);
    let checks = |branch: &str| pull_requests.of_branch(branch).unwrap().checks;
    assert_eq!(checks("error"), Some(Checks::Failed));
    assert_eq!(checks("expected"), Some(Checks::Running));
    assert_eq!(checks("none"), None);
    assert_eq!(checks("failure"), Some(Checks::Failed));
    assert_eq!(checks("pending"), Some(Checks::Running));
}

#[test]
fn github_read_pull_requests__a_draft_is_marked_draft() {
    let draft = node(8, "draft", "owner", None).replace("\"isDraft\": false", "\"isDraft\": true");
    let pull_requests = parsed(&[draft, node(9, "ready", "owner", None)]);
    assert!(pull_requests.of_branch("draft").unwrap().is_draft);
    assert!(!pull_requests.of_branch("ready").unwrap().is_draft);
}

#[test]
fn github_read_pull_requests__a_status_or_an_error_of_github_is_said() {
    let server = TestServer::secure(vec![
        Answer::Bytes(answer("403 Forbidden", &[], b"{}")),
        Answer::Bytes(answer("200 OK", &[], br#"{"errors":[{"message":"Bad credentials"}]}"#)),
    ]);
    let forbidden = read_pull_requests(&http_to(&server), &request()).unwrap_err();
    assert_eq!(forbidden.to_string(), "GitHub answered 403 Forbidden");
    assert!(!forbidden.is_unauthorized());
    let refused = read_pull_requests(&http_to(&server), &request()).unwrap_err();
    assert!(matches!(&refused, GitHubError::Refused(_)));
    assert_eq!(refused.to_string(), "Bad credentials");
}

#[test]
fn github_read_pull_requests__one_post_to_api_github_com_with_the_token_in_a_header_only() {
    let body = answer_of(&[node(3, "feature", "owner", Some("SUCCESS"))]);
    let server = TestServer::secure(vec![Answer::Bytes(answer("200 OK", &[], body.as_bytes()))]);
    let pull_requests = read_pull_requests(&http_to(&server), &request()).unwrap();
    assert_eq!(pull_requests.of_branch("feature").unwrap().number, 3);
    let received = server.received();
    assert_eq!(received.len(), 1);
    let sent = received.first().unwrap();
    assert_eq!(sent.request_line(), "POST /graphql HTTP/1.1");
    assert_eq!(sent.header("host").as_deref(), Some("api.github.com"));
    assert_eq!(sent.header("authorization").as_deref(), Some("Bearer ghp_secret"));
    assert_eq!(sent.header("content-type").as_deref(), Some("application/json"));
    assert_eq!(sent.header("user-agent").as_deref(), Some("gitardo-desktop"));
    let body = json::parse(std::str::from_utf8(&sent.body).unwrap()).unwrap();
    assert!(
        body.get("query")
            .and_then(json::Value::as_str)
            .unwrap()
            .contains("pullRequests(first: 100, states: OPEN")
    );
    let variables = body.get("variables").unwrap();
    assert_eq!(variables.get("owner").and_then(json::Value::as_str), Some("owner"));
    assert_eq!(variables.get("repo").and_then(json::Value::as_str), Some("app"));
    assert!(
        !format!("{:?}", request()).contains("ghp_secret"),
        "the token is never shown"
    );
}

#[test]
fn github_read_pull_requests__a_refused_token_is_told_apart() {
    let server = TestServer::secure(vec![Answer::Bytes(answer("401 Unauthorized", &[], b"{}"))]);
    let error = read_pull_requests(&http_to(&server), &request()).unwrap_err();
    assert!(error.is_unauthorized());
    assert_eq!(error.to_string(), "GitHub answered 401 Unauthorized");
}
