//! The JSON of the reads of the hosting service of the main remote, which version 1.11 adds for milestone M37:
//! `read/hosting.detect-remote`, the service and the project of the main remote, and `read/hosting.web-link`, the
//! address of one of its pages.

use model::hosting::{HostingRepo, HostingService};
use serde::{Deserialize, Serialize};

/// A hosting service, as the JSON names it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum HostingServiceJson {
    /// github.com.
    GitHub,
    /// gitlab.com, or a GitLab of one's own.
    GitLab,
    /// bitbucket.org.
    Bitbucket,
    /// Azure DevOps.
    AzureDevOps,
    /// A Gitea of one's own.
    Gitea,
    /// codeberg.org, or a Forgejo of one's own.
    Forgejo,
}

impl From<HostingService> for HostingServiceJson {
    fn from(service: HostingService) -> HostingServiceJson {
        match service {
            HostingService::GitHub => HostingServiceJson::GitHub,
            HostingService::GitLab => HostingServiceJson::GitLab,
            HostingService::Bitbucket => HostingServiceJson::Bitbucket,
            HostingService::AzureDevOps => HostingServiceJson::AzureDevOps,
            HostingService::Gitea => HostingServiceJson::Gitea,
            HostingService::Forgejo => HostingServiceJson::Forgejo,
        }
    }
}

/// The answer of `read/hosting.detect-remote` (`hosting.detect-remote`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum HostingJson {
    /// The main remote is on no service Gitardo knows, or there is no remote.
    None,
    /// The project of the main remote on its service.
    Found {
        /// The service.
        service: HostingServiceJson,
        /// The address of the pages of the project: `https://gitlab.com/group/app`.
        web: String,
        /// The host name of the remote's URL.
        host: String,
        /// The path of the project on the service: `group/app`.
        path: String,
        /// The name of the remote.
        remote: String,
        /// The default branch of the remote, when git knows it.
        default_branch: Option<String>,
        /// Whether the service keeps the commits of each pull request under a reference of its own, so that one can be
        /// checked out (`hosting.checkout-pull-request`).
        can_check_out: bool,
        /// Whether its comparisons show only what one side has since the two diverged.
        compares_since_diverged: bool,
    },
}

impl HostingJson {
    /// The JSON of the project found, or of none.
    #[must_use]
    pub fn of(project: Option<&HostingRepo>) -> HostingJson {
        let Some(project) = project else {
            return HostingJson::None;
        };
        HostingJson::Found {
            service: project.service.into(),
            web: project.web.clone(),
            host: project.host.clone(),
            path: project.path.clone(),
            remote: project.remote.clone(),
            default_branch: project.default_branch.clone(),
            can_check_out: project.service.pull_request_reference(1).is_some(),
            compares_since_diverged: project.service.compares_only_since_diverged(),
        }
    }
}

/// The page of the project whose address `read/hosting.web-link` gives.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum HostingPageJson {
    /// The page that creates a pull request of a branch, by its name on the remote.
    Create {
        /// The branch.
        branch: String,
    },
    /// A pull request, by its number.
    PullRequest {
        /// The number.
        number: u64,
    },
}

/// The parameters of `read/hosting.web-link`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WebLinkParams {
    /// The page.
    pub page: HostingPageJson,
}

/// The answer of `read/hosting.web-link`: the address, none when the main remote is on no service Gitardo knows.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WebLinkResult {
    /// The address.
    pub url: Option<String>,
}
