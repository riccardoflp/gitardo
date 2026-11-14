//! The pull requests of GitHub in the window (`github.read-pull-requests`, `github.load-pull-requests`): which
//! repository on GitHub the main remote names, the pull requests read last, whether a token is kept, and when they are
//! read again. The reads run on a thread of their own ([`Effect::GitHub`]); the store only decides when.

use std::path::PathBuf;

use model::Timestamp;
use model::contract::{FeatureId, FollowUp};
use model::github::{GitHubRepo, Notice, PullRequests, PushOutcome, Token};
use views::text::github as words;

use super::feature::Said;
use super::message::{Effect, JobId};
use super::state::State;

/// How old, in seconds, the last request is before a read of the repository asks for the pull requests again.
pub const REREAD_AFTER: i64 = 120;

/// What the window knows of the pull requests of the repository shown.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct GitHub {
    /// The repository they are known for.
    pub folder: Option<PathBuf>,
    /// The names of the remotes when the repository on GitHub was last looked for; none before it was.
    pub detected_from: Option<Vec<String>>,
    /// The repository on GitHub of the main remote; none when it is on another host, or not known yet.
    pub repository: Option<GitHubRepo>,
    /// The open pull requests, as last read; none before they were.
    pub pull_requests: Option<PullRequests>,
    /// Whether a token is kept in the credential store, as far as the window knows.
    pub token: TokenKept,
    /// The token `github.load-pull-requests` just found or was given, for the read it asks for at once.
    pub given: Option<Token>,
    /// When the last request was sent, in seconds since 1970.
    pub asked_at: Option<i64>,
    /// When the repository was last read, in seconds since 1970: the store reads no clock.
    pub now: i64,
    /// A read asked for, whatever was read last, and who asked.
    pub wanted: Option<Origin>,
    /// Whether the repository on GitHub is looked for again at the next read of the repository: a remote may have
    /// changed.
    pub detect_again: bool,
    /// The job that looks for the repository on GitHub.
    pub detecting: Option<JobId>,
    /// The job that reads the pull requests, and who asked for it.
    pub reading: Option<(JobId, Origin)>,
}

/// Whether a token is kept for `github.com`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum TokenKept {
    /// Not known yet: the first read after Gitardo starts finds out, reading only the credential store.
    #[default]
    Unknown,
    /// One is.
    Kept,
    /// None is: nothing is read on its own until **Load pull requests from GitHub** keeps one.
    Absent,
}

/// Who asked for a read of the pull requests.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Origin {
    /// `github.load-pull-requests`: what goes wrong is said.
    Loaded,
    /// The window, on its own: what goes wrong is kept quiet, with a warning line in the Command log.
    OnItsOwn,
}

/// What a thread of the pull requests is to do.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GitHubJob {
    /// Its number, which its result carries back.
    pub id: JobId,
    /// The repository.
    pub folder: PathBuf,
    /// What it does.
    pub work: GitHubWork,
}

/// What a job of the pull requests does.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GitHubWork {
    /// Reads the remotes and finds the repository on GitHub of the main one (`github.detect-remote`).
    Detect,
    /// Reads the open pull requests of a repository on GitHub, with the token given, or the one kept; with neither,
    /// nothing is sent.
    Read {
        /// The repository on GitHub.
        repository: GitHubRepo,
        /// The token just found or given.
        token: Option<Token>,
    },
}

/// What a job of the pull requests answers.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GitHubMessage {
    /// The repository on GitHub of the main remote was looked for.
    Detected {
        /// The job.
        job: JobId,
        /// The repository; none when the main remote is on another host, or the remotes could not be read.
        repository: Option<GitHubRepo>,
    },
    /// The pull requests were read, or not.
    Read {
        /// The job.
        job: JobId,
        /// What came of it.
        result: PullsRead,
    },
}

/// What came of a read of the pull requests.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PullsRead {
    /// They were read.
    Read(PullRequests),
    /// No token is kept, and none was given: nothing was sent.
    NoToken,
    /// The request failed: what was said, and whether GitHub refused the token, which was then taken out of the
    /// credential store.
    Failed {
        /// What was said: `GitHub answered 401 Unauthorized`.
        said: String,
        /// Whether GitHub answered 401.
        is_unauthorized: bool,
    },
}

/// The repository shown was read: the pull requests of another repository are forgotten, the repository on GitHub is
/// looked for when the remotes changed, and the pull requests are read when they are wanted or old.
pub fn after_read(state: &mut State, read_at: i64) -> Vec<Effect> {
    let Some(read) = state.shown_read() else {
        return Vec::new();
    };
    let folder = read.folder.clone();
    let remotes = read.state.remotes.clone();
    state.github.now = read_at;
    if state.github.folder.as_ref() != Some(&folder) {
        forget(state, folder.clone());
    }
    let github = &state.github;
    if github.detected_from.as_ref() != Some(&remotes) || github.detect_again {
        if github.detecting.is_some() {
            return Vec::new();
        }
        let id = next_job(state);
        state.github.detect_again = false;
        state.github.detected_from = Some(remotes);
        state.github.detecting = Some(id);
        return vec![Effect::GitHub(GitHubJob {
            id,
            folder,
            work: GitHubWork::Detect,
        })];
    }
    schedule(state)
}

/// What a job of the pull requests answered.
pub fn reduce(state: &mut State, message: GitHubMessage) -> Vec<Effect> {
    match message {
        GitHubMessage::Detected { job, repository } => detected(state, job, repository),
        GitHubMessage::Read { job, result } => pulls_read(state, job, result),
    }
}

/// What an operation that ran asks of the pull requests: the token `github.load-pull-requests` kept is used at once;
/// after a fetch, a pull, a push or a change of the remotes, they are read again once the repository is read, whatever
/// was read last.
pub fn after_operation(state: &mut State, after: &[FollowUp]) -> Vec<Effect> {
    let kept = after.iter().find_map(|follow_up| match follow_up {
        FollowUp::KeepGitHubToken(token) => Some(token.clone()),
        _ => None,
    });
    if let Some(token) = kept {
        state.github.given = Some(token);
        state.github.token = TokenKept::Kept;
        state.github.wanted = Some(Origin::Loaded);
        return schedule(state);
    }
    if after.contains(&FollowUp::ReadPullRequests) {
        state.github.detect_again = true;
        if state.github.wanted.is_none() {
            state.github.wanted = Some(Origin::OnItsOwn);
        }
    }
    Vec::new()
}

/// The browser of the system could not open a pull request: what the system said is said.
pub fn not_opened(state: &mut State, said: String) -> Vec<Effect> {
    state.operations.said = Some(Said::error(said));
    Vec::new()
}

/// What the views are told of GitHub: the repository on GitHub of the main remote and the pull requests known.
#[must_use]
pub fn shown(state: &State) -> views::github::GitHubShown<'_> {
    views::github::GitHubShown {
        repository: state.github.repository.as_ref(),
        pull_requests: state.github.pull_requests.as_ref(),
    }
}

/// The offer to create a pull request after a push that ran, worked out from the repository as it was before the push
/// (`github.offer-after-push`): its branch checked out and whether it had an upstream.
#[must_use]
pub fn offer_after_push(state: &State, feature: FeatureId) -> Option<Notice> {
    let pushes = ["sync.push", "sync.push-force-with-lease", "sync.push-force"];
    if !pushes.contains(&feature.0) {
        return None;
    }
    let repository = &state.shown_read()?.state;
    let branch = repository.branches.iter().find(|branch| branch.is_checked_out)?;
    let none = PullRequests::default();
    model::github::offer_after_push(&PushOutcome {
        succeeded: true,
        branch: Some(&branch.name),
        had_upstream: branch.upstream.is_some(),
        repository: state.github.repository.as_ref(),
        pull_requests: state.github.pull_requests.as_ref().unwrap_or(&none),
    })
}

/// The pull requests of another repository are forgotten, and its repository on GitHub is looked for again.
fn forget(state: &mut State, folder: PathBuf) {
    state.github = GitHub {
        folder: Some(folder),
        token: state.github.token,
        given: state.github.given.take(),
        now: state.github.now,
        ..GitHub::default()
    };
}

/// The repository on GitHub was found, or not: when it changed, the pull requests known are forgotten, and read again.
fn detected(state: &mut State, job: JobId, repository: Option<GitHubRepo>) -> Vec<Effect> {
    if state.github.detecting != Some(job) {
        return Vec::new();
    }
    state.github.detecting = None;
    if state.github.repository != repository {
        state.github.repository = repository;
        state.github.pull_requests = None;
        state.github.asked_at = None;
    }
    schedule(state)
}

/// Reads the pull requests when they are wanted, or when a token is kept and the last request is more than
/// [`REREAD_AFTER`] seconds old; never twice at once, and never on its own with no token kept.
fn schedule(state: &mut State) -> Vec<Effect> {
    let github = &state.github;
    if github.detecting.is_some() || github.reading.is_some() {
        return Vec::new();
    }
    let (Some(repository), Some(folder)) = (github.repository.clone(), github.folder.clone()) else {
        return Vec::new();
    };
    let is_old = github
        .asked_at
        .is_none_or(|asked_at| github.now - asked_at > REREAD_AFTER);
    let origin = match github.wanted {
        Some(Origin::Loaded) => Origin::Loaded,
        _ if github.token == TokenKept::Absent => {
            state.github.wanted = None;
            return Vec::new();
        }
        Some(Origin::OnItsOwn) => Origin::OnItsOwn,
        None if is_old => Origin::OnItsOwn,
        None => return Vec::new(),
    };
    start_read(state, (folder, repository), origin)
}

/// Starts a read of the pull requests.
fn start_read(state: &mut State, (folder, repository): (PathBuf, GitHubRepo), origin: Origin) -> Vec<Effect> {
    let id = next_job(state);
    let github = &mut state.github;
    github.wanted = None;
    github.asked_at = Some(github.now);
    github.reading = Some((id, origin));
    let token = github.given.take();
    vec![Effect::GitHub(GitHubJob {
        id,
        folder,
        work: GitHubWork::Read { repository, token },
    })]
}

/// The pull requests were read, or not: a token refused is known not to be kept any more; what went wrong is said when
/// **Load pull requests from GitHub** asked, and logged otherwise. A read wanted meanwhile runs next.
fn pulls_read(state: &mut State, job: JobId, result: PullsRead) -> Vec<Effect> {
    let Some((reading, origin)) = state.github.reading else {
        return Vec::new();
    };
    if reading != job {
        return Vec::new();
    }
    state.github.reading = None;
    match result {
        PullsRead::Read(pull_requests) => {
            state.github.token = TokenKept::Kept;
            if state.github.pull_requests.as_ref() != Some(&pull_requests) {
                state.github.pull_requests = Some(pull_requests);
            }
        }
        PullsRead::NoToken => state.github.token = TokenKept::Absent,
        PullsRead::Failed { said, is_unauthorized } => {
            if is_unauthorized {
                state.github.token = TokenKept::Absent;
                state.github.pull_requests = None;
            }
            let text = words::could_not_read(&said);
            match origin {
                Origin::Loaded => state.operations.said = Some(Said::error(text)),
                Origin::OnItsOwn => state.log.record_warning(Timestamp(state.github.now), text),
            }
        }
    }
    schedule(state)
}

/// The number of the next job.
fn next_job(state: &mut State) -> JobId {
    let id = JobId(state.next_job);
    state.next_job += 1;
    id
}
