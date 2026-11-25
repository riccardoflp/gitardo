//! A repository opened, and its state answered in two steps (`repo.read-first`, version 1.14 of the protocol): the
//! first step is the answer, and the rest, read behind it from what the first step found, is sent as
//! `repository/state` with the reason `rest`.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::thread;
use std::time::Instant;

use git::read::repo::{FirstState, StateRequest, read_first, read_rest};
use git::read::worktree::CountedWorktrees;
use git::{CancelToken, Git};
use model::contract::Text;
use protocol::messages::{OpenParams, OpenResult, StateParams, StateReason, methods};
use protocol::rpc::{ResponseError, notification_message};
use protocol::state::StateJson;
use serde_json::Value;

use super::{Counted, Session, git_failure, json, locked, now};
use crate::repository::{ReadStarted, Repository};

/// The first step of a state, with when its read started and what the read before counted in the other worktrees.
struct FirstStep {
    /// What the first step found.
    found: FirstState,
    /// When its read started.
    started: ReadStarted,
    /// What the read before counted, for the rest; empty to count every other worktree again.
    before: CountedWorktrees,
}

impl Session {
    /// The answer of `repository/open`: the repository read, opened unless it was, and its state kept. Every other
    /// worktree is counted, as on the first read of a repository chosen. Asked with `first`, the answer is the first
    /// step of its state, and the rest follows in `repository/state`.
    pub(super) fn open(
        self: &Arc<Self>,
        OpenParams { repository, first }: OpenParams,
        cancel: &CancelToken,
    ) -> Result<Value, ResponseError> {
        let folder = PathBuf::from(&repository);
        let started = self.opened(&repository).ok().map(|opened| opened.read_starts());
        let before = CountedWorktrees::default();
        if first {
            let found = self.read_first(&repository, &folder, "The extension opened it", cancel)?;
            let opened = self.opened_or_new(&repository, folder);
            let step = FirstStep {
                found,
                started: started.unwrap_or_else(|| opened.read_starts()),
                before,
            };
            let state = self.rest_behind(&opened, &repository, step);
            return json(&OpenResult { state, first: true });
        }
        let counted = Counted {
            cause: "The extension opened it",
            before: &before,
        };
        let (state, counted) = self.read(&repository, &folder, &counted, cancel)?;
        let state = Arc::new(state);
        let opened = self.opened_or_new(&repository, folder);
        let answer = OpenResult {
            state: StateJson::from(&*state),
            first: false,
        };
        opened.keep_counted(counted);
        let _kept = opened.keep_state(started.unwrap_or_else(|| opened.read_starts()), state);
        json(&answer)
    }

    /// The answer of `read/repo.read-first`: the first step of the state of a repository opened, its rest following
    /// in `repository/state`. Each other worktree keeps the count of the read before when nothing changed in its own
    /// git folder since, unless a **Refresh** is asked.
    pub(super) fn read_first_of(
        self: &Arc<Self>,
        repository: &str,
        refresh: bool,
        cancel: &CancelToken,
    ) -> Result<StateJson, ResponseError> {
        let opened = self.opened(repository)?;
        let started = opened.read_starts();
        let before = if refresh {
            CountedWorktrees::default()
        } else {
            opened.counted()
        };
        let found = self.read_first(repository, opened.folder(), "The extension asked for it", cancel)?;
        let step = FirstStep { found, started, before };
        Ok(self.rest_behind(&opened, repository, step))
    }

    /// The repository of a folder the extension named, opened now unless it was.
    fn opened_or_new(&self, repository: &str, folder: PathBuf) -> Arc<Repository> {
        Arc::clone(
            locked(&self.repositories)
                .entry(repository.to_owned())
                .or_insert_with(|| Arc::new(Repository::new(folder))),
        )
    }

    /// Reads the first step of the state of a repository (`repo.read-first`), shown as running while it does, and
    /// logged as a read.
    fn read_first(
        &self,
        repository: &str,
        folder: &Path,
        cause: &str,
        cancel: &CancelToken,
    ) -> Result<FirstState, ResponseError> {
        let activity = self
            .activities
            .track(Text::from(format!("Reading {repository}")), Text::from(cause), now());
        self.send_activity();
        let started = Instant::now();
        let git = Git::new(folder).with_cancel(cancel.clone());
        let request = StateRequest {
            folder: folder.to_path_buf(),
        };
        let read = read_first(&git, &request);
        drop(activity);
        self.send_activity();
        let line = format!("Read the first step of the state of {repository}");
        self.log_read(repository, line, started.elapsed());
        read.map_err(|error| git_failure(&error))
    }

    /// Keeps the first step of a state, which the reads of the graph and the journal take at once, gives its JSON,
    /// and starts the read of the rest on a thread of its own. A read of the state asked meanwhile waits for it.
    fn rest_behind(self: &Arc<Self>, opened: &Arc<Repository>, repository: &str, step: FirstStep) -> StateJson {
        let answer = StateJson::from(&step.found.state);
        let _kept = opened.keep_first(step.started, Arc::new(step.found.state.clone()));
        let under_way = opened.rest_starts(step.before == CountedWorktrees::default());
        let session = Arc::clone(self);
        let (opened, repository) = (Arc::clone(opened), repository.to_owned());
        let _reading = thread::spawn(move || {
            session.read_rest(&opened, &repository, step);
            drop(under_way);
        });
        answer
    }

    /// Reads the rest of a state from its first step (`repo.read-state`), keeps the whole state and what was counted,
    /// and sends it as `repository/state` with the reason `rest`. It is not cancelled with the request of the first
    /// step, which is answered already. Nothing is sent when it fails, or when an operation ran meanwhile, whose own
    /// read sends the state: a read of the state asked then reads it whole.
    fn read_rest(&self, opened: &Repository, repository: &str, step: FirstStep) {
        let cause = Text::from("The rest of its state");
        let activity = self
            .activities
            .track(Text::from(format!("Reading {repository}")), cause, now());
        self.send_activity();
        let started = Instant::now();
        let read = read_rest(&Git::new(opened.folder()), step.found, &step.before);
        drop(activity);
        self.send_activity();
        let line = match &read {
            Ok(_) => format!("Read the rest of the state of {repository}"),
            Err(error) => format!("The rest of the state of {repository} was not read: {error}"),
        };
        self.log_read(repository, line, started.elapsed());
        let Ok((state, counted)) = read else {
            return;
        };
        let state = Arc::new(state);
        if opened.is_closed() || !opened.keep_state(step.started, Arc::clone(&state)) {
            return;
        }
        opened.keep_counted(counted);
        let params = StateParams {
            repository: repository.to_owned(),
            state: StateJson::from(&*state),
            reason: StateReason::Rest,
        };
        if let Ok(params) = json(&params) {
            self.send(notification_message(methods::STATE, params));
        }
    }
}
