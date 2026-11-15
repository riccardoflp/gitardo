//! `repository/discover`: the repositories of the folders of a workspace (`repo.read-repositories`), each with the git
//! folder its worktrees share (`repo.common-folder`), so that the extension tells which repository of the workspace a
//! worktree chosen elsewhere belongs to without opening each.

use std::path::Path;

use git::read::repo::{CommonFolderRequest, RepositoriesRequest, read_common_folder, read_repositories};
use git::{CancelToken, Git};
use protocol::messages::{DiscoverParams, DiscoverResult, DiscoveredRepositoryJson};
use protocol::rpc::ResponseError;
use serde_json::Value;

use crate::session::{git_failure, json};

/// The answer of `repository/discover`: the repositories in the order a window shows them. A repository whose common
/// folder git cannot say is listed without it.
///
/// # Errors
///
/// When git cannot be started, or the read is cancelled.
pub fn discover(params: &DiscoverParams, cancel: &CancelToken) -> Result<Value, ResponseError> {
    let git = Git::new(".").with_cancel(cancel.clone());
    let request = RepositoriesRequest {
        folders: params.folders.iter().map(Into::into).collect(),
    };
    let found = read_repositories(&git, &request).map_err(|error| git_failure(&error))?;
    let text = |folder: &Path| folder.to_string_lossy().into_owned();
    let repositories = found
        .iter()
        .map(|repository| DiscoveredRepositoryJson {
            folder: text(&repository.folder),
            superproject: repository.superproject.as_deref().map(text),
            common_folder: read_common_folder(&git.in_directory(&repository.folder), &CommonFolderRequest)
                .ok()
                .as_deref()
                .map(text),
        })
        .collect();
    json(&DiscoverResult { repositories })
}
