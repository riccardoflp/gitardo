//! The reads of stage 5, those of the graph from stage 7, and `repo.watch-changes` from milestone M15: every
//! `read/<card>` of the protocol but `repo.read-state`, each answered by the function its card names. A read runs on the thread of its request, never
//! on the thread of the repository's engine, so that reads run at the same time as each other and as an operation;
//! its git is given the token that `$/cancel` cancels, and it is logged as a read with how long it took. Nothing here
//! changes the repository.

mod commit_box;
mod commits;
mod conflicts;
mod files;
mod graph;
mod hosting;
mod repository;
mod tags;

use std::fmt::Display;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;

use git::{CancelToken, Git, GitError};
use model::state::RepositoryState;
use protocol::messages::RepositoryParams;
use protocol::reads::methods;
use protocol::rpc::{RequestId, ResponseError, codes};
use serde::Serialize;
use serde_json::Value;

use crate::session::{Session, git_failure, json, parameters};

/// What a read is given: the repository it reads, the git that reads it, and the token that cancels it.
pub struct Reading<'a> {
    /// The session, for the settings and the host questions.
    pub session: &'a Arc<Session>,
    /// The id of the request, which the pages of a read of the commits sent as they are read name (1.8).
    pub id: &'a RequestId,
    /// The folder of the top of the working tree.
    pub folder: PathBuf,
    /// Git, run in that folder, with the token of the request.
    pub git: Git,
    /// The token of the request.
    pub cancel: &'a CancelToken,
    /// The state the last read of the repository kept (`repo.read-state`), while no operation ran since it: the reads
    /// of the graph take what is checked out, the references and the stashes from it rather than asking git again.
    pub kept_state: Option<Arc<RepositoryState>>,
}

/// Whether a method is one of the reads of this module.
pub fn is_read(method: &str) -> bool {
    methods::ALL.contains(&method)
}

/// The answer of a read of a repository opened, logged as a read with how long it took.
///
/// # Errors
///
/// When the repository is not open, the parameters are not the read's, or git fails or is cancelled.
pub fn answer(
    session: &Arc<Session>,
    id: &RequestId,
    method: &str,
    params: Value,
    cancel: &CancelToken,
) -> Result<Value, ResponseError> {
    let RepositoryParams { repository } = parameters(params.clone())?;
    let opened = session.opened(&repository)?;
    let folder = opened.folder().to_path_buf();
    let reading = Reading {
        session,
        id,
        git: Git::new(&folder).with_cancel(cancel.clone()),
        folder,
        cancel,
        kept_state: opened.kept_state(),
    };
    let started = Instant::now();
    let answer = read(&reading, method, params);
    session.log_read(&repository, format!("Read {method} in {repository}"), started.elapsed());
    answer
}

/// The answer of one read, by its method.
fn read(reading: &Reading<'_>, method: &str, params: Value) -> Result<Value, ResponseError> {
    match method {
        methods::GRAPH_COMMITS => graph::commits(reading, &parameters(params)?),
        methods::GRAPH_POSITION => graph::position(reading, &parameters(params)?),
        methods::SEARCH_MATCHES => graph::matches(reading, &parameters(params)?),
        methods::SEARCH_REVISION => commits::search_revision(reading, parameters(params)?),
        methods::DETAILS_COMMIT => commits::details(reading, parameters(params)?),
        methods::COMMIT_PAGE_PATCH => commits::commit_patch(reading, &parameters(params)?),
        methods::COMPARE_SIDES => commits::compare_sides(reading, &parameters(params)?),
        methods::COMPARE_FILES => commits::compare_files(reading, parameters(params)?),
        methods::COMPARE_PATCH => commits::compare_patch(reading, &parameters(params)?),
        methods::COMPARE_REVISION => commits::compare_revision(reading, parameters(params)?),
        methods::BRANCH_LOG => commits::branch_log(reading, &parameters(params)?),
        methods::CHANGES_WORKING_TREE => files::working_tree(reading),
        methods::CHANGES_IGNORED => files::ignored(reading, &parameters(params)?),
        methods::DIFF_SIDES => files::diff_sides(reading, &parameters(params)?),
        methods::FILE_VIEW_REVISION => files::file_revision(reading, parameters(params)?),
        methods::BLAME_LINE => files::blame_line(reading, &parameters(params)?),
        methods::BLAME_FILE => files::blame_file(reading, parameters(params)?),
        methods::FILE_HISTORY_NAMES => files::names(reading, parameters(params)?),
        methods::FILE_HISTORY_COMMITTED_LINES => files::committed_lines(reading, parameters(params)?),
        methods::PREPARED_MESSAGE => files::prepared_message(reading),
        methods::COMMON_FOLDER => files::common_folder(reading),
        methods::MERGE_VIEW_SIDES => conflicts::merge_sides(reading, parameters(params)?),
        methods::CONFLICTS_FILES => conflicts::files(reading),
        methods::CONFLICTS_SIDE_NAMES => conflicts::side_names(reading),
        methods::CONFLICTS_RULES => conflicts::rules(reading, &parameters(params)?),
        methods::REFLOG => repository::reflog(reading, parameters(params)?),
        methods::REMOTES => repository::remotes(reading),
        methods::IDENTITY => repository::identity(reading),
        methods::SUBMODULES => repository::submodules(reading),
        methods::WORKTREE_FOLDER_LINKS => repository::folder_links(reading, &parameters(params)?),
        methods::SETUP_IGNORED => repository::setup_ignored(reading, &parameters(params)?),
        methods::SETUP_TAKES => repository::setup_takes(reading, &parameters(params)?),
        methods::SETUP_COMMANDS => repository::setup_commands(reading),
        methods::SETUP_PATHS => repository::path_statuses(reading, &parameters(params)?),
        methods::PULL_REQUESTS => repository::pull_requests(reading, &parameters(params)?),
        methods::HOSTING_DETECT => hosting::detect(reading),
        methods::HOSTING_WEB_LINK => hosting::web_link(reading, &parameters(params)?),
        methods::WATCH_CHANGES => files::changes_to_read(reading, parameters(params)?),
        methods::DETAILS_SIGNATURE => commit_box::signature(reading, &parameters(params)?),
        methods::COMMIT_AUTHORS => commit_box::authors(reading),
        methods::TAG_REMOTE_TAGS => tags::remote_tags(reading, parameters(params)?),
        _ => Err(ResponseError::new(
            codes::METHOD_NOT_FOUND,
            format!("Gitardo's engine has no method named {method}."),
        )),
    }
}

/// The JSON of what a read of git gave, or the error of git.
fn answered<T, J: Serialize>(read: Result<T, GitError>, convert: impl FnOnce(&T) -> J) -> Result<Value, ResponseError> {
    let read = read.map_err(|error| git_failure(&error))?;
    json(&convert(&read))
}

/// The error of parameters that do not hold what the read takes: a commit that is not a full id, bytes not in base64.
fn invalid(problem: impl Display) -> ResponseError {
    ResponseError::new(codes::INVALID_PARAMS, problem.to_string())
}

/// A path from the top of a folder, with `/`, as a path of the system.
fn below(top: &Path, path: &str) -> PathBuf {
    path.split('/')
        .fold(top.to_path_buf(), |folder, name| folder.join(name))
}

/// A path as the system gives it, as a string.
fn path_text(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}
