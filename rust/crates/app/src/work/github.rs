//! The thread of the pull requests of a window: it looks for the repository on GitHub of the main remote
//! (`github.detect-remote`) and reads the open pull requests (`github.read-pull-requests`), with the token given or the
//! one kept in the credential store, never asking git's credential helper. A token GitHub refuses is taken out of the
//! store.

use std::ffi::OsString;
use std::sync::Arc;
use std::thread;

use engine::{Activities, GITHUB_TOKEN, SecretStore, SystemSecrets};
use git::Git;
use git::read::github::read_http_proxy;
use git::read::remote::{RemotesRequest, read_remotes};
use http::{Cancel, Http, Proxies, ProxySettings, Trust};
use model::contract::Text;
use model::github::{GitHubRepo, Token, detect_remote};
use model::remote::Remote;
use views::text::github as words;

use super::Deliver;
use crate::store::Message;
use crate::store::github::{GitHubJob, GitHubMessage, GitHubWork, PullsRead};

/// What makes the store the secrets are kept in, on each thread that needs it.
pub type MakeSecrets = Arc<dyn Fn() -> Box<dyn SecretStore> + Send + Sync>;

/// How the pull requests are reached.
#[derive(Clone)]
pub enum Network {
    /// Through the network of the system: the certificates it trusts and the proxies of the environment or of git.
    System,
    /// Through this client: a test's, which reaches a server on 127.0.0.1 in place of `api.github.com`.
    Given(Http),
}

/// Where the pull requests are read from, and where the token is kept: the system's, or a test's.
#[derive(Clone)]
pub struct GitHubAccess {
    /// The network.
    pub network: Network,
    /// The credential store.
    pub secrets: MakeSecrets,
    /// The variables git runs with: a test's keep it from the user's configuration.
    pub git_variables: Vec<(OsString, OsString)>,
}

impl GitHubAccess {
    /// The network and the credential store of the system.
    #[must_use]
    pub fn system() -> GitHubAccess {
        GitHubAccess {
            network: Network::System,
            secrets: Arc::new(|| Box::new(SystemSecrets)),
            git_variables: Vec::new(),
        }
    }

    /// Git in a folder, with the variables of this access.
    #[must_use]
    pub fn git(&self, folder: &std::path::Path) -> Git {
        self.git_variables.iter().fold(Git::new(folder), |git, (name, value)| {
            git.with_variable(name.clone(), value.clone())
        })
    }
}

impl std::fmt::Debug for GitHubAccess {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.debug_struct("GitHubAccess").finish_non_exhaustive()
    }
}

/// Starts the jobs of the pull requests, each on a thread of its own.
pub struct GitHubReader {
    /// Where they are read from.
    access: GitHubAccess,
    /// What runs, where a read is shown.
    activities: Activities,
    /// Where the answers go.
    deliver: Deliver,
}

impl GitHubReader {
    /// A reader that gives its answers to `deliver`.
    #[must_use]
    pub fn new(access: GitHubAccess, activities: &Activities, deliver: &Deliver) -> GitHubReader {
        GitHubReader {
            access,
            activities: activities.clone(),
            deliver: Arc::clone(deliver),
        }
    }

    /// Runs a job on a thread of its own; a read of the pull requests is among the activities while it runs.
    pub fn start(&self, job: GitHubJob) {
        let access = self.access.clone();
        let activities = self.activities.clone();
        let deliver = Arc::clone(&self.deliver);
        let _started = thread::Builder::new().name(String::from("github")).spawn(move || {
            let message = {
                let _activity = matches!(job.work, GitHubWork::Read { .. }).then(|| {
                    let description = Text::from(words::READING_PULL_REQUESTS);
                    activities.track_read(description, Text::from(""), super::now())
                });
                run(&access, &job)
            };
            deliver(message);
        });
    }
}

/// Runs a job of the pull requests, and gives its answer.
#[must_use]
pub fn run(access: &GitHubAccess, job: &GitHubJob) -> Message {
    let message = match &job.work {
        GitHubWork::Detect => GitHubMessage::Detected {
            job: job.id,
            repository: detect(&access.git(&job.folder)),
        },
        GitHubWork::Read { repository, token } => GitHubMessage::Read {
            job: job.id,
            result: read(access, job, (repository, token.clone())),
        },
    };
    Message::GitHub(message)
}

/// The repository on GitHub of the main remote; none when the remotes cannot be read.
fn detect(git: &Git) -> Option<GitHubRepo> {
    let remotes = read_remotes(git, &RemotesRequest).ok()?;
    let remotes: Vec<Remote> = remotes
        .into_iter()
        .map(|remote| Remote {
            name: remote.name,
            url: remote.url,
        })
        .collect();
    detect_remote(&remotes)
}

/// Reads the pull requests with the token given, or the one kept; a 401 takes the kept token out of the store.
fn read(access: &GitHubAccess, job: &GitHubJob, (repository, given): (&GitHubRepo, Option<Token>)) -> PullsRead {
    let secrets = (access.secrets)();
    // A store that cannot be read keeps no token Gitardo can use.
    let token = given.or_else(|| secrets.read(GITHUB_TOKEN).ok().flatten().map(Token));
    let Some(token) = token else {
        return PullsRead::NoToken;
    };
    let http = match &access.network {
        Network::Given(http) => http.clone(),
        Network::System => match system_http(&access.git(&job.folder)) {
            Ok(http) => http,
            Err(said) => {
                return PullsRead::Failed {
                    said,
                    is_unauthorized: false,
                };
            }
        },
    };
    let request = engine::github::PullsRequest {
        repository: repository.clone(),
        token,
        cancel: Cancel::new(),
    };
    match engine::github::read_pull_requests(&http, &request) {
        Ok(pull_requests) => PullsRead::Read(pull_requests),
        Err(error) => {
            let is_unauthorized = error.is_unauthorized();
            if is_unauthorized {
                // A store that refuses keeps the token, which GitHub will refuse again: nothing else can be done.
                let _taken_out = secrets.delete(GITHUB_TOKEN);
            }
            PullsRead::Failed {
                said: error.to_string(),
                is_unauthorized,
            }
        }
    }
}

/// The client of the system: the certificates it trusts, and the proxies of the environment, or git's `http.proxy`.
fn system_http(git: &Git) -> Result<Http, String> {
    let git_proxy = read_http_proxy(git).ok().flatten();
    let proxies =
        Proxies::from_settings(&ProxySettings::from_environment(git_proxy)).map_err(|error| error.to_string())?;
    let trust = Trust::system().map_err(|error| error.to_string())?;
    Ok(Http::new(trust, proxies))
}
