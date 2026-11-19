//! What the engine knows while it speaks to one extension: the repositories opened, the requests that run and their
//! tokens, how much the log says, and what runs; and the answer of each method.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::thread;
use std::time::{Instant, SystemTime};

use engine::{Activities, LogEntry, LogKind, LogLevel};
use git::read::repo::{StateRequest, read_state_counting};
use git::read::worktree::CountedWorktrees;
use git::{CancelToken, Git, GitError};
use model::Timestamp;
use model::contract::FeatureSettings;
use model::contract::Text;
use model::state::RepositoryState;
use protocol::feature::{FeatureRunParams, SettingsParams, ValidateParams};
use protocol::messages::{
    ActivityJson, ActivityParams, CancelParams, ChangedParams, EngineInfo, HostAbilities, InitializeParams,
    InitializeResult, LogEntryJson, LogLineParams, OpenResult, ReadStateParams, RepositoryParams, StateParams,
    StateReason, VERSION, is_compatible, methods,
};
use protocol::rpc::{ErrorData, RequestId, ResponseError, codes, error_message, notification_message, result_message};
use protocol::state::StateJson;
use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::{Map, Value};

use crate::outgoing::Outgoing;
use crate::repository::{Next, Repository};

mod summaries;

/// The engine speaking to one extension.
#[derive(Debug)]
pub struct Session {
    /// The messages for the thread that writes the standard output.
    outgoing: Sender<Value>,
    /// The repositories opened, by the folder the extension named.
    repositories: Mutex<HashMap<String, Arc<Repository>>>,
    /// The requests that run, with the token that cancels each.
    running: Mutex<HashMap<RequestId, CancelToken>>,
    /// Whether the log says the reads too.
    level: Mutex<LogLevel>,
    /// What runs, in every repository.
    activities: Activities,
    /// Whether `shutdown` was asked.
    is_shut_down: AtomicBool,
    /// The requests sent to the extension and waited for.
    asked: Outgoing,
    /// The settings, by the desktop's names, from `initialize` and `settings/changed`.
    settings: Mutex<Map<String, Value>>,
    /// Whether the extension runs the setup commands of new worktrees itself, in VS Code's terminals (`host.terminal`
    /// of `initialize`).
    runs_tasks: AtomicBool,
    /// The host questions the extension said at `initialize` that it answers.
    host: Mutex<HostAbilities>,
    /// What the folders the extension's Repositories view lists are on (`repo.worktree-summary`).
    summaries: summaries::Summaries,
}

/// Why the state of a repository is read, and whether every other worktree is counted again (**Refresh**).
struct Counting<'a> {
    /// What the activity says it is read for.
    cause: &'a str,
    /// Whether the read is a **Refresh**, which counts what is not committed in every other worktree again.
    refresh: bool,
}

/// Why a state is read, and what the read before counted in the other worktrees.
struct Counted<'a> {
    /// What the activity says it is read for.
    cause: &'a str,
    /// The counts of the read before, kept for each other worktree whose git folder did not change since.
    before: &'a CountedWorktrees,
}

/// The value behind a lock, even when a thread that held it panicked: what it guards is plain data, still whole.
pub fn locked<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

impl Session {
    /// A session that writes its messages to the given sender.
    pub fn new(outgoing: Sender<Value>) -> Session {
        Session {
            outgoing,
            repositories: Mutex::new(HashMap::new()),
            running: Mutex::new(HashMap::new()),
            level: Mutex::new(LogLevel::Info),
            activities: Activities::new(),
            is_shut_down: AtomicBool::new(false),
            asked: Outgoing::default(),
            settings: Mutex::new(Map::new()),
            runs_tasks: AtomicBool::new(false),
            host: Mutex::new(HostAbilities::default()),
            summaries: summaries::Summaries::default(),
        }
    }

    /// The requests sent to the extension and waited for.
    pub fn asked(&self) -> &Outgoing {
        &self.asked
    }

    /// The settings of the features, as the extension sent them.
    pub fn feature_settings(&self) -> FeatureSettings {
        let mut settings = crate::settings::feature_settings(&locked(&self.settings));
        // Area 6 of stage 4: the models are VS Code's when the extension answers `host/languageModel`, and what is
        // chosen of them is written in VS Code's settings.
        settings.uses_host_models = self.host().language_model;
        settings
    }

    /// Whether the extension runs the setup commands of new worktrees itself: they are then handed over in `waiting`
    /// instead of run by the engine.
    pub fn runs_tasks(&self) -> bool {
        self.runs_tasks.load(Ordering::SeqCst)
    }

    /// The host questions the extension answers.
    pub fn host(&self) -> HostAbilities {
        *locked(&self.host)
    }

    /// How much the log says.
    pub fn log_level(&self) -> LogLevel {
        *locked(&self.level)
    }

    /// Sends a message; nothing happens once the output is closed.
    pub fn send(&self, message: Value) {
        let _sent = self.outgoing.send(message);
    }

    /// Whether `shutdown` was asked.
    pub fn is_shut_down(&self) -> bool {
        self.is_shut_down.load(Ordering::SeqCst)
    }

    /// Cancels every request that runs, and ends the waits for the extension's answers.
    pub fn cancel_all(&self) {
        for token in locked(&self.running).values() {
            token.cancel();
        }
        self.asked.end();
    }

    /// Answers a request on a thread of its own, with a token that `$/cancel` cancels.
    pub fn request(self: &Arc<Self>, id: RequestId, method: String, params: Value) {
        let cancel = CancelToken::new();
        let _replaced = locked(&self.running).insert(id.clone(), cancel.clone());
        let session = Arc::clone(self);
        let _answering = thread::spawn(move || {
            let outcome = session.answer(&id, &method, params, &cancel);
            let _done = locked(&session.running).remove(&id);
            session.send(match outcome {
                Ok(result) => result_message(&id, result),
                Err(error) => error_message(Some(&id), &error),
            });
        });
    }

    /// Takes a notification of the extension; one this version does not know is left, as JSON-RPC has it.
    pub fn notification(self: &Arc<Self>, method: &str, params: Value) {
        match method {
            methods::CANCEL => {
                if let Ok(CancelParams { id }) = parameters::<CancelParams>(params)
                    && let Some(token) = locked(&self.running).get(&id)
                {
                    token.cancel();
                }
            }
            methods::CHANGED => {
                if let Ok(changed) = parameters::<ChangedParams>(params) {
                    self.changed(changed);
                }
            }
            methods::SETTINGS_CHANGED => {
                if let Ok(SettingsParams { settings }) = parameters::<SettingsParams>(params) {
                    *locked(&self.settings) = settings;
                }
            }
            other => summaries::notification(self, other, params),
        }
    }

    /// The answer of a request.
    fn answer(
        self: &Arc<Self>,
        id: &RequestId,
        method: &str,
        params: Value,
        cancel: &CancelToken,
    ) -> Result<Value, ResponseError> {
        match method {
            methods::FEATURE_RUN => crate::feature::run(self, id, parameters::<FeatureRunParams>(params)?, cancel),
            methods::QUESTION_VALIDATE => crate::feature::validate(parameters::<ValidateParams>(params)?),
            methods::JOURNAL_LAST => crate::journal::last(self, &parameters(params)?),
            methods::DISCOVER => crate::discover::discover(&parameters(params)?, cancel),
            methods::INITIALIZE => self.initialize(&parameters(params)?),
            protocol::mermaid::methods::FILE_VIEW_MERMAID => crate::mermaid::draw(&parameters(params)?),
            protocol::help::methods::ASK => crate::help::ask(self, id, &parameters(params)?, cancel),
            protocol::summaries::methods::WORKTREE_SUMMARY => summaries::follow(self, &parameters(params)?, cancel),
            read if crate::reads::is_read(read) => crate::reads::answer(self, id, read, params, cancel),
            // Area 6 of stage 4: the reads that ask a language model of VS Code.
            read if crate::ai::is_ai_read(read) => crate::ai::answer(self, id, read, params, cancel),
            methods::SHUTDOWN => {
                self.is_shut_down.store(true, Ordering::SeqCst);
                Ok(Value::Null)
            }
            methods::OPEN => self.open(parameters(params)?, cancel),
            methods::CLOSE => {
                let RepositoryParams { repository } = parameters(params)?;
                let closed = locked(&self.repositories)
                    .remove(&repository)
                    .ok_or_else(|| not_open(&repository))?;
                closed.close();
                Ok(Value::Null)
            }
            methods::READ_STATE => {
                let ReadStateParams { repository, refresh } = parameters(params)?;
                let opened = self.opened(&repository)?;
                let reading = Counting {
                    cause: "The extension asked for it",
                    refresh,
                };
                json(&self.read_kept(&opened, &repository, &reading, cancel)?)
            }
            _ => Err(ResponseError::new(
                codes::METHOD_NOT_FOUND,
                format!("Gitardo's engine has no method named {method}."),
            )),
        }
    }

    /// The answer of `repository/open`: the repository read, opened unless it was, and its state kept. Every other
    /// worktree is counted, as on the first read of a repository chosen.
    fn open(
        &self,
        RepositoryParams { repository }: RepositoryParams,
        cancel: &CancelToken,
    ) -> Result<Value, ResponseError> {
        let folder = PathBuf::from(&repository);
        let started = self.opened(&repository).ok().map(|opened| opened.read_starts());
        let counted = Counted {
            cause: "The extension opened it",
            before: &CountedWorktrees::default(),
        };
        let (state, counted) = self.read(&repository, &folder, &counted, cancel)?;
        let state = Arc::new(state);
        let opened = Arc::clone(
            locked(&self.repositories)
                .entry(repository)
                .or_insert_with(|| Arc::new(Repository::new(folder))),
        );
        let answer = OpenResult {
            state: StateJson::from(&*state),
        };
        opened.keep_counted(counted);
        opened.keep_state(started.unwrap_or_else(|| opened.read_starts()), state);
        json(&answer)
    }

    /// The answer of `initialize`: the engine's version, or the error of a major version it does not speak.
    fn initialize(&self, params: &InitializeParams) -> Result<Value, ResponseError> {
        let answer = InitializeResult {
            protocol: String::from(VERSION),
            engine: EngineInfo {
                version: String::from(env!("CARGO_PKG_VERSION")),
                git: git_version(),
            },
        };
        if !is_compatible(&params.protocol) {
            let message = format!(
                "Gitardo's engine speaks version {VERSION} of the protocol, and the extension version {}.",
                params.protocol
            );
            return Err(ResponseError::new(codes::INVALID_PARAMS, message).with_data(json(&answer)?));
        }
        *locked(&self.level) = params.log_level.into();
        locked(&self.settings).clone_from(&params.settings);
        self.runs_tasks.store(params.host.terminal, Ordering::SeqCst);
        *locked(&self.host) = params.host;
        json(&answer)
    }

    /// A repository opened, or the error of one that is not.
    pub fn opened(&self, repository: &str) -> Result<Arc<Repository>, ResponseError> {
        locked(&self.repositories)
            .get(repository)
            .cloned()
            .ok_or_else(|| not_open(repository))
    }

    /// Takes the paths the watchers saw change in a repository opened, and starts waiting for the burst to end.
    fn changed(self: &Arc<Self>, changed: ChangedParams) {
        let Ok(repository) = self.opened(&changed.repository) else {
            return;
        };
        if !repository.changed(&changed.paths) {
            return;
        }
        let session = Arc::clone(self);
        let _waiting = thread::spawn(move || {
            loop {
                match repository.next() {
                    Next::Wait(wait) => thread::sleep(wait),
                    Next::End => break,
                    Next::Read => session.read_again(&changed.repository, &repository),
                }
            }
        });
    }

    /// Reads a repository again after its files changed, and sends its state.
    fn read_again(&self, repository: &str, opened: &Repository) {
        self.read_and_send(repository, opened, "Files changed", StateReason::Changed);
    }

    /// Reads a repository again after a plan ran there, and sends its state. The state kept from before the plan is
    /// dropped first, and so is any read that started before it ended.
    pub fn read_after_plan(&self, repository: &str, opened: &Repository) {
        opened.operation_ran();
        self.read_and_send(repository, opened, "An operation ran", StateReason::Ran);
    }

    /// Reads a repository and sends its state, saying why.
    fn read_and_send(&self, repository: &str, opened: &Repository, cause: &str, reason: StateReason) {
        let reading = Counting { cause, refresh: false };
        if let Ok(state) = self.read_kept(opened, repository, &reading, &CancelToken::new()) {
            let params = StateParams {
                repository: repository.to_owned(),
                state,
                reason,
            };
            if let Ok(params) = json(&params) {
                self.send(notification_message(methods::STATE, params));
            }
        }
    }

    /// Reads the state of a repository opened, keeps it for `journal/last` (`undo.last`) and the reads of the graph
    /// (`graph.read-commits`), and gives its JSON. Each other worktree keeps the count of the read before when nothing
    /// changed in its own git folder since, unless the read is a **Refresh** (M15, Q462).
    fn read_kept(
        &self,
        opened: &Repository,
        repository: &str,
        reading: &Counting<'_>,
        cancel: &CancelToken,
    ) -> Result<StateJson, ResponseError> {
        let started = opened.read_starts();
        let before = if reading.refresh {
            CountedWorktrees::default()
        } else {
            opened.counted()
        };
        let counted = Counted {
            cause: reading.cause,
            before: &before,
        };
        let (state, counted) = self.read(repository, opened.folder(), &counted, cancel)?;
        let state = Arc::new(state);
        let answer = StateJson::from(&*state);
        opened.keep_counted(counted);
        opened.keep_state(started, state);
        Ok(answer)
    }

    /// Reads the state of a repository (`repo.read-state`), shown as running while it does, and logged as a read; each
    /// other worktree keeps the count the read before has of it when nothing changed in its git folder since. Gives
    /// what was counted, for the read after it.
    fn read(
        &self,
        repository: &str,
        folder: &Path,
        counted: &Counted<'_>,
        cancel: &CancelToken,
    ) -> Result<(RepositoryState, CountedWorktrees), ResponseError> {
        let activity = self.activities.track(
            Text::from(format!("Reading {repository}")),
            Text::from(counted.cause),
            now(),
        );
        self.send_activity();
        let started = Instant::now();
        let git = Git::new(folder).with_cancel(cancel.clone());
        let read = read_state_counting(
            &git,
            &StateRequest {
                folder: folder.to_path_buf(),
            },
            counted.before,
        );
        drop(activity);
        self.send_activity();
        self.log_read(repository, format!("Read the state of {repository}"), started.elapsed());
        read.map_err(|error| git_failure(&error))
    }

    /// Sends what runs now.
    fn send_activity(&self) {
        let running = self.activities.running().iter().map(ActivityJson::from).collect();
        if let Ok(params) = json(&ActivityParams { running }) {
            self.send(notification_message(methods::ACTIVITY, params));
        }
    }

    /// Sends the line of a read with how long it took, when the log says the reads.
    pub fn log_read(&self, repository: &str, line: String, took: std::time::Duration) {
        if *locked(&self.level) != LogLevel::Debug {
            return;
        }
        let entry = LogEntry {
            time: now(),
            kind: LogKind::Read(took),
            line,
        };
        self.send_log(repository, vec![entry]);
    }

    /// Sends lines of the log of a repository, in their order.
    pub fn send_log(&self, repository: &str, entries: Vec<LogEntry>) {
        for entry in entries {
            let params = LogLineParams {
                repository: Some(repository.to_owned()),
                entry: LogEntryJson::from(&entry),
            };
            if let Ok(params) = json(&params) {
                self.send(notification_message(methods::LOG_LINE, params));
            }
        }
    }

    /// Gives an answer of the extension to the request of the engine waiting for it.
    pub fn response(&self, id: &RequestId, outcome: Result<Value, ResponseError>) {
        self.asked.answer(id, outcome);
    }
}

/// The parameters of a message, or the error of parameters that are not the method's.
pub(crate) fn parameters<T: DeserializeOwned>(params: Value) -> Result<T, ResponseError> {
    serde_json::from_value(params)
        .map_err(|error| ResponseError::new(codes::INVALID_PARAMS, format!("The parameters are not right: {error}")))
}

/// The JSON of a value of the protocol.
pub fn json(value: &impl Serialize) -> Result<Value, ResponseError> {
    serde_json::to_value(value).map_err(|error| {
        ResponseError::new(
            codes::INTERNAL_ERROR,
            format!("The answer could not be written: {error}"),
        )
    })
}

/// The error of a repository that is not open.
fn not_open(repository: &str) -> ResponseError {
    ResponseError::new(codes::NOT_OPEN, format!("The repository {repository} is not open."))
}

/// The error of a read of git that failed or was cancelled.
pub fn git_failure(error: &GitError) -> ResponseError {
    if let GitError::Cancelled { .. } = error {
        return ResponseError::new(codes::CANCELLED, "The read was cancelled.");
    }
    let output = match error {
        GitError::Failed { stderr, .. } if !stderr.is_empty() => Some(stderr.clone()),
        _ => None,
    };
    let data = ErrorData {
        output,
        steps: Vec::new(),
        follow_ups: Vec::new(),
    };
    let error = ResponseError::new(codes::GIT_FAILED, error.to_string());
    match serde_json::to_value(&data) {
        Ok(data) => error.with_data(data),
        Err(_) => error,
    }
}

/// What `git --version` says; none when git cannot be run.
fn git_version() -> Option<String> {
    let said = Git::new(".").run_read(&["--version"]).ok()?;
    Some(String::from_utf8_lossy(&said).trim().to_owned())
}

/// The time of day, in seconds since 1970.
pub fn now() -> Timestamp {
    let seconds = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map_or(0, |since| since.as_secs());
    Timestamp(i64::try_from(seconds).unwrap_or(i64::MAX))
}
