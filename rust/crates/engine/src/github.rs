//! The open pull requests of the repository on GitHub, each with the result of the checks of its last commit, read
//! from the GraphQL API of `api.github.com` (`github.read-pull-requests`).

use std::fmt;

use http::{Cancel, Http, Request, Url};
use json::{Member, Value};
use model::github::{Checks, GitHubRepo, PullRequest, PullRequests, Token};

/// The only address the pull requests are asked of.
pub const ENDPOINT: &str = "https://api.github.com/graphql";

/// What is asked of GitHub: the 100 open pull requests updated last, each with its number, title, address, whether it
/// is a draft, its branch, the owner of its branch's repository and the checks of its last commit.
pub const QUERY: &str = "query($owner: String!, $repo: String!) { repository(owner: $owner, name: $repo) { \
pullRequests(first: 100, states: OPEN, orderBy: {field: UPDATED_AT, direction: DESC}) { nodes { number title url \
isDraft headRefName headRepositoryOwner { login } commits(last: 1) { nodes { commit { statusCheckRollup { state } } } } \
} } } }";

/// Whose pull requests are read, with what.
#[derive(Debug, Clone)]
pub struct PullsRequest {
    /// The repository on GitHub of the main remote.
    pub repository: GitHubRepo,
    /// The token it is read with.
    pub token: Token,
    /// What cancels the request: the token of the job that sends it.
    pub cancel: Cancel,
}

/// What went wrong reading the pull requests.
#[derive(Debug)]
pub enum GitHubError {
    /// The request could not be sent, or its answer read.
    Http(http::Error),
    /// GitHub answered with a status that is not 2xx.
    Status {
        /// The status: 401.
        status: u16,
        /// The status with its reason: `401 Unauthorized`.
        line: String,
    },
    /// GitHub answered with errors: the message of the first.
    Refused(String),
    /// The answer is not what was asked for.
    Unreadable(String),
}

impl GitHubError {
    /// Whether GitHub refused the token (status 401), so that it is not kept.
    #[must_use]
    pub fn is_unauthorized(&self) -> bool {
        matches!(self, GitHubError::Status { status: 401, .. })
    }
}

impl fmt::Display for GitHubError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GitHubError::Http(error) => error.fmt(formatter),
            GitHubError::Status { line, .. } => write!(formatter, "GitHub answered {line}"),
            GitHubError::Refused(message) => formatter.write_str(message),
            GitHubError::Unreadable(reason) => write!(formatter, "GitHub's answer could not be read: {reason}"),
        }
    }
}

impl std::error::Error for GitHubError {}

/// The open pull requests of a repository on GitHub: one `POST` of [`QUERY`] to [`ENDPOINT`], with the token in the
/// `Authorization` header only.
///
/// Feature: github.read-pull-requests
///
/// # Errors
///
/// When the request fails, GitHub answers a status that is not 2xx or with errors, or the answer cannot be read.
pub fn read_pull_requests(http: &Http, request: &PullsRequest) -> Result<PullRequests, GitHubError> {
    let url = Url::parse(ENDPOINT).map_err(GitHubError::Http)?;
    let body = json::to_compact_text(&query_body(&request.repository));
    let sent = Request::post(url, body.into_bytes())
        .header("Content-Type", "application/json")
        .header("User-Agent", "gitardo-desktop")
        .secret_header("Authorization", &format!("Bearer {}", request.token.0));
    let response = http.send(&sent, &request.cancel).map_err(GitHubError::Http)?;
    if !response.head.is_success() {
        return Err(GitHubError::Status {
            status: response.head.status,
            line: response.head.status_line(),
        });
    }
    parse_pull_requests(&response.body, &request.repository.owner)
}

/// `{"query": <the query>, "variables": {"owner": <owner>, "repo": <repo>}}`.
fn query_body(repository: &GitHubRepo) -> Value {
    let member = |name: &str, value: Value| Member {
        name: name.to_owned(),
        value,
    };
    Value::Object(vec![
        member("query", Value::String(QUERY.to_owned())),
        member(
            "variables",
            Value::Object(vec![
                member("owner", Value::String(repository.owner.clone())),
                member("repo", Value::String(repository.repo.clone())),
            ]),
        ),
    ])
}

/// The pull requests of an answer of GitHub, one per branch name, the first in the list (the one updated last); those
/// whose branch is in a repository of another owner than `owner`, compared without regard to case, are left out.
///
/// # Errors
///
/// When the answer has errors, or is not what was asked for.
pub fn parse_pull_requests(body: &[u8], owner: &str) -> Result<PullRequests, GitHubError> {
    let text = std::str::from_utf8(body).map_err(|error| GitHubError::Unreadable(error.to_string()))?;
    let answer = json::parse(text).map_err(|error| GitHubError::Unreadable(error.to_string()))?;
    if let Some(first) = answer
        .get("errors")
        .and_then(Value::as_array)
        .and_then(<[Value]>::first)
    {
        let message = first
            .get("message")
            .and_then(Value::as_str)
            .unwrap_or("GitHub refused the request.");
        return Err(GitHubError::Refused(message.to_owned()));
    }
    let nodes = answer
        .get("data")
        .and_then(|data| data.get("repository"))
        .and_then(|repository| repository.get("pullRequests"))
        .and_then(|pulls| pulls.get("nodes"))
        .and_then(Value::as_array)
        .ok_or_else(|| GitHubError::Unreadable(String::from("it has no pull requests")))?;
    let mut pull_requests = PullRequests::default();
    for node in nodes {
        let Some(pull_request) = pull_request(node) else {
            continue;
        };
        let head_owner = node
            .get("headRepositoryOwner")
            .and_then(|head| head.get("login"))
            .and_then(Value::as_str);
        if head_owner.is_some_and(|login| login.eq_ignore_ascii_case(owner)) {
            let _first = pull_requests
                .by_branch
                .entry(pull_request.branch.clone())
                .or_insert(pull_request);
        }
    }
    Ok(pull_requests)
}

/// A pull request of the answer; none when a field it needs is missing.
fn pull_request(node: &Value) -> Option<PullRequest> {
    let text = |name: &str| node.get(name).and_then(Value::as_str).map(str::to_owned);
    let state = node
        .get("commits")
        .and_then(|commits| commits.get("nodes"))
        .and_then(Value::as_array)
        .and_then(<[Value]>::first)
        .and_then(|last| last.get("commit"))
        .and_then(|commit| commit.get("statusCheckRollup"))
        .and_then(|rollup| rollup.get("state"))
        .and_then(Value::as_str);
    Some(PullRequest {
        number: node
            .get("number")
            .and_then(Value::as_number)
            .and_then(json::Number::as_u64)?,
        title: text("title").unwrap_or_default(),
        url: text("url")?,
        is_draft: node.get("isDraft").and_then(Value::as_bool).unwrap_or(false),
        branch: text("headRefName")?,
        checks: state.and_then(checks),
    })
}

/// What the state of the checks of a commit says.
fn checks(state: &str) -> Option<Checks> {
    match state {
        "SUCCESS" => Some(Checks::Passed),
        "FAILURE" | "ERROR" => Some(Checks::Failed),
        "PENDING" | "EXPECTED" => Some(Checks::Running),
        // Any other state says nothing of the checks GitHub knows of.
        _ => None,
    }
}

#[cfg(test)]
mod tests;
