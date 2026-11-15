//! The reads of what a repository holds besides its commits: the reflog, the remotes, the identity, the submodules,
//! the worktrees and what new ones get, and the open pull requests on GitHub.

use std::path::{Path, PathBuf};

use engine::github::{PullsRequest, read_pull_requests};
use git::read::github::read_http_proxy;
use git::read::identity::{IdentityRequest, read_identity};
use git::read::reflog::{self, ReflogRequest};
use git::read::remote::{RemotesRequest, read_remotes};
use git::read::submodule::{SubmoduleListRequest, read_list};
use git::read::worktree::{FolderLinksRequest, read_folder_links};
use git::read::worktree_setup::{
    IgnoredRequest, PatternsRequest, read_committed_at_top, read_ignored, read_path_status, read_what_patterns_take,
};
use http::{Cancel, Http, Proxies, ProxySettings, Trust};
use model::github::{Token, detect_remote};
use model::remote::Remote;
use model::worktree_setup::{CommandsInput, LOCK_FILES, PathStatus, TakeRequest, propose_commands};
use protocol::reads::{
    CommandProposalJson, CommandsResult, GitHubTokenParams, GitHubTokenResult, IdentityKeysJson, IgnoredEntryJson,
    PathStatusJson, PathsParams, PathsResult, PullRequestsJson, PullRequestsParams, ReflogParams, ReflogPositionJson,
    RemoteJson, SetupPreviewJson, SetupTakesParams, WorktreeParams, methods,
};
use protocol::rpc::{ResponseError, codes};
use protocol::state::SubmoduleJson;
use serde_json::Value;

use super::{Reading, answered, below, path_text};
use crate::outgoing::NoAnswer;
use crate::session::{git_failure, json};

/// The places a reference was at, the last first (`reflog.read`).
pub fn reflog(reading: &Reading<'_>, params: ReflogParams) -> Result<Value, ResponseError> {
    let request = ReflogRequest {
        reference: params.reference,
        count: params.count,
    };
    answered(reflog::read(&reading.git, &request), |positions| {
        positions.iter().map(ReflogPositionJson::from).collect::<Vec<_>>()
    })
}

/// The remotes, with their URLs (`remote.read-remotes`).
pub fn remotes(reading: &Reading<'_>) -> Result<Value, ResponseError> {
    answered(read_remotes(&reading.git, &RemotesRequest), |remotes| {
        remotes.iter().map(RemoteJson::from).collect::<Vec<_>>()
    })
}

/// The identity commits are made with, and the keys of it this repository sets (`identity.read`).
pub fn identity(reading: &Reading<'_>) -> Result<Value, ResponseError> {
    answered(read_identity(&reading.git, &IdentityRequest), |read| {
        IdentityKeysJson::from(read)
    })
}

/// The submodules, and theirs (`submodule.read-list`).
pub fn submodules(reading: &Reading<'_>) -> Result<Value, ResponseError> {
    let request = SubmoduleListRequest {
        folder: reading.folder.clone(),
    };
    answered(read_list(&reading.git, &request), |submodules| {
        submodules.iter().map(SubmoduleJson::from).collect::<Vec<_>>()
    })
}

/// The links to folders that removing a worktree would follow (`worktree.read-folder-links`).
pub fn folder_links(reading: &Reading<'_>, params: &WorktreeParams) -> Result<Value, ResponseError> {
    let request = FolderLinksRequest {
        worktree: PathBuf::from(&params.worktree),
    };
    answered(read_folder_links(&reading.git, &request), |links| PathsResult {
        paths: links.iter().map(|link| path_text(link)).collect(),
    })
}

/// What git ignores in a checkout, with its size on disk (`worktree-setup.read-ignored`).
pub fn setup_ignored(reading: &Reading<'_>, params: &WorktreeParams) -> Result<Value, ResponseError> {
    let request = IgnoredRequest {
        folder: PathBuf::from(&params.worktree),
    };
    answered(read_ignored(&reading.git, &request), |entries| {
        entries.iter().map(IgnoredEntryJson::from).collect::<Vec<_>>()
    })
}

/// What link and copy patterns take from each checkout asked (`worktree-setup.read-what-patterns-take`).
pub fn setup_takes(reading: &Reading<'_>, params: &SetupTakesParams) -> Result<Value, ResponseError> {
    let mut previews = Vec::with_capacity(params.requests.len());
    for asked in &params.requests {
        let take = TakeRequest::from(asked);
        let request = PatternsRequest {
            folder: take.folder,
            link_patterns: take.patterns.link,
            copy_patterns: take.patterns.copy,
            commit: take.commit,
        };
        let preview = read_what_patterns_take(&reading.git, &request).map_err(|error| git_failure(&error))?;
        previews.push(SetupPreviewJson::from(&preview));
    }
    json(&previews)
}

/// The commands proposed for new worktrees, from the lock files committed at the top (`worktree-setup.propose-commands`).
pub fn setup_commands(reading: &Reading<'_>) -> Result<Value, ResponseError> {
    let committed = read_committed_at_top(&reading.git, &LOCK_FILES).map_err(|error| git_failure(&error))?;
    let package_manager = committed
        .iter()
        .any(|file| file == "package.json")
        .then(|| package_manager(&reading.folder))
        .flatten();
    let proposals = propose_commands(&CommandsInput {
        committed: &committed,
        package_manager: package_manager.as_deref(),
    });
    json(&CommandsResult {
        commands: proposals.iter().map(CommandProposalJson::from).collect(),
    })
}

/// The `packageManager` of the `package.json` at the top of the checkout.
fn package_manager(folder: &Path) -> Option<String> {
    let text = std::fs::read_to_string(folder.join("package.json")).ok()?;
    let value = ::json::parse_with_comments(&text).ok()?;
    value.get("packageManager")?.as_str().map(str::to_owned)
}

/// What each path is for git, and whether it is a folder on disk (`worktree-setup.explain-change`).
pub fn path_statuses(reading: &Reading<'_>, params: &PathsParams) -> Result<Value, ResponseError> {
    let mut statuses = Vec::with_capacity(params.paths.len());
    for path in &params.paths {
        let (tracked, ignored) = read_path_status(&reading.git, path).map_err(|error| git_failure(&error))?;
        let status = PathStatus {
            is_folder: below(&reading.folder, path).is_dir(),
            tracked,
            ignored,
        };
        statuses.push(PathStatusJson::from(&status));
    }
    json(&statuses)
}

/// The open pull requests of the repository on GitHub of a remote (`github.read-pull-requests`), read with the token
/// VS Code keeps, which the extension is asked for without asking the user to sign in.
pub fn pull_requests(reading: &Reading<'_>, params: &PullRequestsParams) -> Result<Value, ResponseError> {
    let remotes: Vec<Remote> = read_remotes(&reading.git, &RemotesRequest)
        .map_err(|error| git_failure(&error))?
        .into_iter()
        .map(|remote| Remote {
            name: remote.name,
            url: remote.url,
        })
        .filter(|remote| params.remote.as_ref().is_none_or(|asked| *asked == remote.name))
        .collect();
    let Some(repository) = detect_remote(&remotes) else {
        return json(&PullRequestsJson::NotOnGitHub);
    };
    let Some(token) = github_token(reading)? else {
        return json(&PullRequestsJson::NoToken);
    };
    let http = match system_http(reading) {
        Ok(http) => http,
        Err(message) => {
            return json(&PullRequestsJson::Failed {
                message,
                is_unauthorized: false,
            });
        }
    };
    let cancel = reading.cancel.clone();
    let request = PullsRequest {
        repository,
        token,
        cancel: Cancel::following(move || cancel.is_cancelled()),
    };
    match read_pull_requests(&http, &request) {
        Ok(pulls) => json(&PullRequestsJson::from(&pulls)),
        Err(_) if reading.cancel.is_cancelled() => Err(ResponseError::new(codes::CANCELLED, "The read was cancelled.")),
        Err(error) => json(&PullRequestsJson::Failed {
            message: error.to_string(),
            is_unauthorized: error.is_unauthorized(),
        }),
    }
}

/// The token of GitHub the extension gives, when it said at `initialize` that it answers `host/githubToken`.
fn github_token(reading: &Reading<'_>) -> Result<Option<Token>, ResponseError> {
    if !reading.session.host().github_token {
        return Ok(None);
    }
    let params = json(&GitHubTokenParams {
        host: String::from("github.com"),
        ask: false,
    })?;
    let session = reading.session;
    match session.asked().ask(
        |message| session.send(message),
        methods::HOST_GITHUB_TOKEN,
        params,
        reading.cancel,
    ) {
        Ok(answer) => Ok(serde_json::from_value::<GitHubTokenResult>(answer)
            .ok()
            .and_then(|result| result.token)
            .filter(|token| !token.is_empty())
            .map(Token)),
        Err(NoAnswer::Cancelled) => Err(ResponseError::new(codes::CANCELLED, "The read was cancelled.")),
        // An extension that cannot give the token gives none.
        Err(NoAnswer::Failed(_)) => Ok(None),
    }
}

/// The client of the system: the certificates it trusts, and the proxies of the environment, or git's `http.proxy`.
fn system_http(reading: &Reading<'_>) -> Result<Http, String> {
    let git_proxy = read_http_proxy(&reading.git).ok().flatten();
    let proxies =
        Proxies::from_settings(&ProxySettings::from_environment(git_proxy)).map_err(|error| error.to_string())?;
    let trust = Trust::system().map_err(|error| error.to_string())?;
    Ok(Http::new(trust, proxies))
}
