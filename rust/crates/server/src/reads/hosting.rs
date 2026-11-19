//! The reads of the hosting service of the main remote (version 1.11): which service and project it is
//! (`hosting.detect-remote`), the address of one of its pages (`hosting.web-links`), and the merge requests of a GitLab
//! (`hosting.read-merge-requests`), with the services of one's own of `hosting.selfHosted`.

use engine::hosting::{MergeRequestsRequest, read_merge_requests};
use git::read::hosting::read_default_branch;
use git::read::remote::{RemotesRequest, read_remotes};
use http::Cancel;
use model::hosting::{self, HostingPage, HostingRepo, HostingService, detect_remote};
use model::remote::Remote;
use protocol::reads::{HostingJson, HostingPageJson, PullRequestsJson, WebLinkParams, WebLinkResult};
use protocol::rpc::{ResponseError, codes};
use serde_json::Value;

use super::Reading;
use crate::session::{git_failure, json};

/// The project of the main remote on its hosting service, with the default branch of the remote; none when it is on
/// none Gitardo knows.
pub(super) fn project(reading: &Reading<'_>, remotes: &[Remote]) -> Option<HostingRepo> {
    let known = reading.session.feature_settings().hosting;
    detect_remote(remotes, &known).map(|mut project| {
        // Without the remote's `HEAD`, a new pull request on Gitea and Forgejo goes into `main`.
        project.default_branch = read_default_branch(&reading.git, &project.remote).ok().flatten();
        project
    })
}

/// The remotes of the repository, each with its URL.
pub(super) fn remotes(reading: &Reading<'_>) -> Result<Vec<Remote>, ResponseError> {
    Ok(read_remotes(&reading.git, &RemotesRequest)
        .map_err(|error| git_failure(&error))?
        .into_iter()
        .map(|remote| Remote {
            name: remote.name,
            url: remote.url,
        })
        .collect())
}

/// `read/hosting.detect-remote`: the hosting service and the project of the main remote.
pub fn detect(reading: &Reading<'_>) -> Result<Value, ResponseError> {
    let remotes = remotes(reading)?;
    json(&HostingJson::of(project(reading, &remotes).as_ref()))
}

/// `read/hosting.web-link`: the address of a page of the project of the main remote.
pub fn web_link(reading: &Reading<'_>, params: &WebLinkParams) -> Result<Value, ResponseError> {
    let remotes = remotes(reading)?;
    let url = project(reading, &remotes).map(|project| match &params.page {
        HostingPageJson::Create { branch } => hosting::web_link(&project, &HostingPage::Create(branch)),
        HostingPageJson::PullRequest { number } => hosting::web_link(&project, &HostingPage::PullRequest(*number)),
    });
    json(&WebLinkResult { url })
}

/// The open merge requests of the project of the main remote on a GitLab (`hosting.read-merge-requests`), read with the
/// token the extension keeps for its host name; not on GitHub when the main remote is on no GitLab.
pub(super) fn merge_requests(reading: &Reading<'_>, remotes: &[Remote]) -> Result<Value, ResponseError> {
    let Some(project) = project(reading, remotes).filter(|project| project.service == HostingService::GitLab) else {
        return json(&PullRequestsJson::NotOnGitHub);
    };
    let Some(token) = super::repository::token_of(reading, &project.host)? else {
        return json(&PullRequestsJson::NoToken);
    };
    let http = match super::repository::system_http(reading) {
        Ok(http) => http,
        Err(message) => {
            return json(&PullRequestsJson::Failed {
                message,
                is_unauthorized: false,
            });
        }
    };
    let cancel = reading.cancel.clone();
    let request = MergeRequestsRequest {
        repository: project,
        token,
        cancel: Cancel::following(move || cancel.is_cancelled()),
    };
    match read_merge_requests(&http, &request) {
        Ok(pulls) => json(&PullRequestsJson::from(&pulls)),
        Err(_) if reading.cancel.is_cancelled() => Err(ResponseError::new(codes::CANCELLED, "The read was cancelled.")),
        Err(error) => json(&PullRequestsJson::Failed {
            message: error.to_string(),
            is_unauthorized: error.is_unauthorized(),
        }),
    }
}
