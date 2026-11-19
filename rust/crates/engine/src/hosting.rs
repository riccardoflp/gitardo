//! The open merge requests of a project on a GitLab, each with the state of the pipeline of its last commit, read from
//! its GraphQL API (`hosting.read-merge-requests`).

use std::fmt;

use http::{Cancel, Http, Request, Url};
use json::{Member, Value};
use model::github::{Checks, PullRequest, PullRequests, Token};
use model::hosting::HostingRepo;

/// What is asked of a GitLab: the 100 open merge requests updated last, each with its number in the project, title,
/// address, whether it is a draft, its branch, the projects it goes from and into, and the state of its pipeline.
pub const MERGE_REQUESTS_QUERY: &str = "query($path: ID!) { project(fullPath: $path) { mergeRequests(state: opened, \
first: 100, sort: UPDATED_DESC) { nodes { iid title webUrl draft sourceBranch sourceProjectId targetProjectId \
headPipeline { status } } } } }";

/// Whose merge requests are read, with what.
#[derive(Debug, Clone)]
pub struct MergeRequestsRequest {
    /// The project on its GitLab.
    pub repository: HostingRepo,
    /// The token it is read with.
    pub token: Token,
    /// What cancels the request: the token of the job that sends it.
    pub cancel: Cancel,
}

/// What went wrong reading the pull requests of a hosting service.
#[derive(Debug)]
pub enum HostingError {
    /// The request could not be sent, or its answer read.
    Http(http::Error),
    /// The service answered with a status that is not 2xx.
    Status {
        /// The name of the service: `GitLab`.
        service: &'static str,
        /// The status: 401.
        status: u16,
        /// The status with its reason: `401 Unauthorized`.
        line: String,
    },
    /// The service answered with errors: the message of the first.
    Refused(String),
    /// The answer is not what was asked for.
    Unreadable(String),
}

impl HostingError {
    /// Whether the service refused the token (status 401), so that it is not kept.
    #[must_use]
    pub fn is_unauthorized(&self) -> bool {
        matches!(self, HostingError::Status { status: 401, .. })
    }
}

impl fmt::Display for HostingError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HostingError::Http(error) => error.fmt(formatter),
            HostingError::Status { service, line, .. } => write!(formatter, "{service} answered {line}"),
            HostingError::Refused(message) => formatter.write_str(message),
            HostingError::Unreadable(reason) => write!(formatter, "The answer could not be read: {reason}"),
        }
    }
}

impl std::error::Error for HostingError {}

/// The open merge requests of a project on a GitLab: one `POST` of [`MERGE_REQUESTS_QUERY`] to `<site>/api/graphql`,
/// at the address of the pages of the main remote's GitLab, with the token in the `Authorization` header only.
///
/// Feature: hosting.read-merge-requests
///
/// # Errors
///
/// When the request fails, GitLab answers a status that is not 2xx or with errors, or the answer cannot be read.
pub fn read_merge_requests(http: &Http, request: &MergeRequestsRequest) -> Result<PullRequests, HostingError> {
    let body = ask_gitlab(http, request, MERGE_REQUESTS_QUERY)?;
    parse_merge_requests(&body)
}

/// Sends a query to the GraphQL API of the GitLab of a project, and gives the body of its answer.
pub(crate) fn ask_gitlab(http: &Http, request: &MergeRequestsRequest, query: &str) -> Result<Vec<u8>, HostingError> {
    let endpoint = format!("{}/api/graphql", request.repository.site());
    let url = Url::parse(&endpoint).map_err(HostingError::Http)?;
    let body = json::to_compact_text(&query_body(query, &request.repository.path));
    let sent = Request::post(url, body.into_bytes())
        .header("Content-Type", "application/json")
        .header("User-Agent", "gitardo-desktop")
        .secret_header("Authorization", &format!("Bearer {}", request.token.0));
    let response = http.send(&sent, &request.cancel).map_err(HostingError::Http)?;
    if !response.head.is_success() {
        return Err(HostingError::Status {
            service: "GitLab",
            status: response.head.status,
            line: response.head.status_line(),
        });
    }
    Ok(response.body)
}

/// `{"query": <the query>, "variables": {"path": <the path of the project>}}`.
fn query_body(query: &str, path: &str) -> Value {
    let member = |name: &str, value: Value| Member {
        name: name.to_owned(),
        value,
    };
    Value::Object(vec![
        member("query", Value::String(query.to_owned())),
        member(
            "variables",
            Value::Object(vec![member("path", Value::String(path.to_owned()))]),
        ),
    ])
}

/// The merge requests of an answer of GitLab, one per branch name, the first in the list (the one updated last); those
/// whose source project is not their target project come from a fork and are left out.
///
/// # Errors
///
/// When the answer has errors, or is not what was asked for.
pub fn parse_merge_requests(body: &[u8]) -> Result<PullRequests, HostingError> {
    let mut pull_requests = PullRequests::default();
    for node in merge_request_nodes(body)? {
        let Some(merge_request) = merge_request(&node) else {
            continue;
        };
        let project = |name: &str| node.get(name).and_then(Value::as_number).and_then(json::Number::as_u64);
        if project("sourceProjectId") == project("targetProjectId") {
            let _first = pull_requests
                .by_branch
                .entry(merge_request.branch.clone())
                .or_insert(merge_request);
        }
    }
    Ok(pull_requests)
}

/// The nodes of the merge requests of an answer of GitLab.
pub(crate) fn merge_request_nodes(body: &[u8]) -> Result<Vec<Value>, HostingError> {
    let text = std::str::from_utf8(body).map_err(|error| HostingError::Unreadable(error.to_string()))?;
    let answer = json::parse(text).map_err(|error| HostingError::Unreadable(error.to_string()))?;
    if let Some(first) = answer
        .get("errors")
        .and_then(Value::as_array)
        .and_then(<[Value]>::first)
    {
        let message = first
            .get("message")
            .and_then(Value::as_str)
            .unwrap_or("GitLab refused the request.");
        return Err(HostingError::Refused(message.to_owned()));
    }
    let nodes = answer
        .get("data")
        .and_then(|data| data.get("project"))
        .and_then(|project| project.get("mergeRequests"))
        .and_then(|merge_requests| merge_requests.get("nodes"))
        .and_then(Value::as_array)
        .ok_or_else(|| HostingError::Unreadable(String::from("it has no merge requests")))?;
    Ok(nodes.to_vec())
}

/// A merge request of the answer; none when a field it needs is missing. GitLab gives its number in the project,
/// `iid`, as a string.
pub(crate) fn merge_request(node: &Value) -> Option<PullRequest> {
    let text = |name: &str| node.get(name).and_then(Value::as_str).map(str::to_owned);
    let number = node.get("iid").and_then(|iid| {
        iid.as_str()
            .and_then(|digits| digits.parse().ok())
            .or_else(|| iid.as_number().and_then(json::Number::as_u64))
    })?;
    let status = node
        .get("headPipeline")
        .and_then(|pipeline| pipeline.get("status"))
        .and_then(Value::as_str);
    Some(PullRequest {
        number,
        title: text("title").unwrap_or_default(),
        url: text("webUrl")?,
        is_draft: node.get("draft").and_then(Value::as_bool).unwrap_or(false),
        branch: text("sourceBranch")?,
        checks: status.and_then(pipeline),
    })
}

/// What the state of a pipeline says.
fn pipeline(status: &str) -> Option<Checks> {
    match status {
        "SUCCESS" => Some(Checks::Passed),
        "FAILED" => Some(Checks::Failed),
        "CREATED" | "WAITING_FOR_RESOURCE" | "PREPARING" | "PENDING" | "RUNNING" | "SCHEDULED" => Some(Checks::Running),
        // Any other state, such as CANCELED, SKIPPED or MANUAL, says nothing of the changes.
        _ => None,
    }
}

#[cfg(test)]
mod tests;
