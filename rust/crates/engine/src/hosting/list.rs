//! The open pull requests of the main remote to pick one from, those from forks included, on GitHub and GitLab
//! (`hosting.read-pull-request-list`).

use http::{Cancel, Http, Request, Url};
use json::{Member, Value};
use model::github::Token;
use model::hosting::{HostingRepo, HostingService, PullListEntry};

use super::HostingError;

/// What is asked of GitHub: the 100 open pull requests opened last, each with its number, title, author, branch, and
/// the repository of its branch when it is not the repository asked of.
pub const GITHUB_LIST_QUERY: &str = "query($owner: String!, $repo: String!) { repository(owner: $owner, name: $repo) \
{ pullRequests(first: 100, states: OPEN, orderBy: {field: CREATED_AT, direction: DESC}) { nodes { number title \
headRefName isCrossRepository author { login } headRepository { nameWithOwner } } } } }";

/// What is asked of a GitLab: the 100 open merge requests created last, each with its number in the project, title,
/// author, branch and the project it goes from.
pub const GITLAB_LIST_QUERY: &str = "query($path: ID!) { project(fullPath: $path) { mergeRequests(state: opened, \
first: 100, sort: CREATED_DESC) { nodes { iid title sourceBranch sourceProjectId targetProjectId author { username } \
sourceProject { fullPath } } } } }";

/// Whose pull requests are listed, with what.
#[derive(Debug, Clone)]
pub struct PullListRequest {
    /// The project of the main remote on its hosting service.
    pub repository: HostingRepo,
    /// The token it is read with.
    pub token: Token,
    /// What cancels the request.
    pub cancel: Cancel,
}

/// The open pull requests of the project of the main remote, newest first, those from forks included: one `POST` to
/// the GraphQL API of GitHub or of the GitLab, with the token in the `Authorization` header only; nothing is read of
/// the other services, and the list is empty.
///
/// Feature: hosting.read-pull-request-list
///
/// # Errors
///
/// When the request fails, the service answers a status that is not 2xx or with errors, or the answer cannot be read.
pub fn read_pull_request_list(http: &Http, request: &PullListRequest) -> Result<Vec<PullListEntry>, HostingError> {
    let repository = &request.repository;
    match repository.service {
        HostingService::GitHub => {
            let (owner, repo) = repository.path.split_once('/').unwrap_or((&repository.path, ""));
            let variables = vec![member("owner", text(owner)), member("repo", text(repo))];
            let body = post(http, request, crate::github::ENDPOINT, (GITHUB_LIST_QUERY, variables))?;
            parse_github_list(&body)
        }
        HostingService::GitLab => {
            let endpoint = format!("{}/api/graphql", repository.site());
            let variables = vec![member("path", text(&repository.path))];
            let body = post(http, request, &endpoint, (GITLAB_LIST_QUERY, variables))?;
            parse_gitlab_list(&body)
        }
        _ => Ok(Vec::new()),
    }
}

/// A member of an object of JSON.
fn member(name: &str, value: Value) -> Member {
    Member {
        name: name.to_owned(),
        value,
    }
}

/// A string of JSON.
fn text(value: &str) -> Value {
    Value::String(value.to_owned())
}

/// Sends a query with its variables to a GraphQL API, and gives the body of the answer.
fn post(
    http: &Http,
    request: &PullListRequest,
    endpoint: &str,
    (query, variables): (&str, Vec<Member>),
) -> Result<Vec<u8>, HostingError> {
    let url = Url::parse(endpoint).map_err(HostingError::Http)?;
    let body = Value::Object(vec![
        member("query", text(query)),
        member("variables", Value::Object(variables)),
    ]);
    let sent = Request::post(url, json::to_compact_text(&body).into_bytes())
        .header("Content-Type", "application/json")
        .header("User-Agent", "gitardo-desktop")
        .secret_header("Authorization", &format!("Bearer {}", request.token.0));
    let response = http.send(&sent, &request.cancel).map_err(HostingError::Http)?;
    if !response.head.is_success() {
        return Err(HostingError::Status {
            service: request.repository.service.name(),
            status: response.head.status,
            line: response.head.status_line(),
        });
    }
    Ok(response.body)
}

/// The nodes at a path of an answer of GraphQL, or the message of its first error.
fn nodes(body: &[u8], path: &[&str]) -> Result<Vec<Value>, HostingError> {
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
            .unwrap_or("The service refused the request.");
        return Err(HostingError::Refused(message.to_owned()));
    }
    let mut value = Some(&answer);
    for name in path {
        value = value.and_then(|value| value.get(name));
    }
    let nodes = value
        .and_then(Value::as_array)
        .ok_or_else(|| HostingError::Unreadable(String::from("it has no pull requests")))?;
    Ok(nodes.to_vec())
}

/// A string at a path of a node.
fn string_at(node: &Value, path: &[&str]) -> Option<String> {
    let mut value = Some(node);
    for name in path {
        value = value.and_then(|value| value.get(name));
    }
    value.and_then(Value::as_str).map(str::to_owned)
}

/// The pull requests of an answer of GitHub to [`GITHUB_LIST_QUERY`].
///
/// # Errors
///
/// When the answer has errors, or is not what was asked for.
pub fn parse_github_list(body: &[u8]) -> Result<Vec<PullListEntry>, HostingError> {
    let nodes = nodes(body, &["data", "repository", "pullRequests", "nodes"])?;
    Ok(nodes
        .iter()
        .filter_map(|node| {
            let is_fork = node.get("isCrossRepository").and_then(Value::as_bool).unwrap_or(false);
            Some(PullListEntry {
                number: node
                    .get("number")
                    .and_then(Value::as_number)
                    .and_then(json::Number::as_u64)?,
                title: string_at(node, &["title"]).unwrap_or_default(),
                author: string_at(node, &["author", "login"]).unwrap_or_default(),
                branch: string_at(node, &["headRefName"])?,
                fork: is_fork
                    .then(|| string_at(node, &["headRepository", "nameWithOwner"]))
                    .flatten(),
            })
        })
        .collect())
}

/// The merge requests of an answer of GitLab to [`GITLAB_LIST_QUERY`].
///
/// # Errors
///
/// When the answer has errors, or is not what was asked for.
pub fn parse_gitlab_list(body: &[u8]) -> Result<Vec<PullListEntry>, HostingError> {
    let nodes = nodes(body, &["data", "project", "mergeRequests", "nodes"])?;
    Ok(nodes
        .iter()
        .filter_map(|node| {
            let project = |name: &str| node.get(name).and_then(Value::as_number).and_then(json::Number::as_u64);
            let is_fork = project("sourceProjectId") != project("targetProjectId");
            let number = node.get("iid").and_then(|iid| {
                iid.as_str()
                    .and_then(|digits| digits.parse().ok())
                    .or_else(|| iid.as_number().and_then(json::Number::as_u64))
            })?;
            Some(PullListEntry {
                number,
                title: string_at(node, &["title"]).unwrap_or_default(),
                author: string_at(node, &["author", "username"]).unwrap_or_default(),
                branch: string_at(node, &["sourceBranch"])?,
                fork: is_fork
                    .then(|| string_at(node, &["sourceProject", "fullPath"]))
                    .flatten(),
            })
        })
        .collect())
}
