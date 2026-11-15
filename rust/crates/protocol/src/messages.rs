//! The messages of version 1.0 that stage 1 of the transition builds: starting and stopping, the repositories, the log
//! and the activity (`docs/transition/02-protocol.md`).

use engine::{ActivityEntry, LogEntry, LogKind};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::rpc::RequestId;
use crate::state::StateJson;

/// The names of the methods.
pub mod methods {
    /// The extension's first request.
    pub const INITIALIZE: &str = "initialize";
    /// The request that ends what runs, before `exit`.
    pub const SHUTDOWN: &str = "shutdown";
    /// The notification that ends the process.
    pub const EXIT: &str = "exit";
    /// The notification that cancels a request, either way.
    pub const CANCEL: &str = "$/cancel";
    /// Makes the engine of a repository and reads its state.
    pub const OPEN: &str = "repository/open";
    /// Drops the engine of a repository.
    pub const CLOSE: &str = "repository/close";
    /// The extension's watchers saw files change.
    pub const CHANGED: &str = "repository/changed";
    /// The engine read a repository again.
    pub const STATE: &str = "repository/state";
    /// The state of a repository, read now (`repo.read-state`).
    pub const READ_STATE: &str = "read/repo.read-state";
    /// A line of the log, as it is written.
    pub const LOG_LINE: &str = "log/line";
    /// What runs, whenever it changes.
    pub const ACTIVITY: &str = "activity/changed";
    /// Runs a feature of the catalog (stage 2).
    pub const FEATURE_RUN: &str = "feature/run";
    /// The engine asks a question of a feature that runs.
    pub const FEATURE_ASK: &str = "feature/ask";
    /// The engine asks whether to run a plan.
    pub const FEATURE_CONFIRM: &str = "feature/confirm";
    /// What the extension carries out once a feature ran.
    pub const FEATURE_FOLLOW_UPS: &str = "feature/followUps";
    /// Whether the text typed so far is an answer a question takes.
    pub const QUESTION_VALIDATE: &str = "question/validate";
    /// Every setting, whenever one changes.
    pub const SETTINGS_CHANGED: &str = "settings/changed";
    /// The newest entry of the journal, for the label of Undo.
    pub const JOURNAL_LAST: &str = "journal/last";
    /// An operation the extension ran, recorded for Undo.
    pub const JOURNAL_RECORD: &str = "journal/record";
}

/// The version of the protocol the engine speaks: 1.1 added the messages of the features (stage 2), 1.2 those of the
/// journal (stage 3), 1.3 those of stage 4: the operations offered with their target and answers, such as **Delete
/// anyway** and **Remove anyway**, the targets of the conflicts, and whether the host runs the setup commands of new
/// worktrees in its terminals; 1.4 the reads but the graph's (stage 5, [`crate::reads`]).
pub const VERSION: &str = "1.4";

/// Whether a version of the protocol is one the engine speaks: the same major number, whatever the minor one.
#[must_use]
pub fn is_compatible(asked: &str) -> bool {
    let major = |version: &str| version.split('.').next().map(str::to_owned);
    major(asked).is_some_and(|asked| Some(asked) == major(VERSION))
}

/// The parameters of `initialize`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InitializeParams {
    /// The version of the protocol the extension speaks.
    pub protocol: String,
    /// Who the extension is.
    pub client: ClientInfo,
    /// The settings, by their names without `gitardo.`.
    #[serde(default)]
    pub settings: Map<String, Value>,
    /// What the host can answer.
    #[serde(default)]
    pub host: HostAbilities,
    /// Whether the reads are logged too.
    #[serde(default)]
    pub log_level: LogLevelName,
}

/// Who the extension is.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClientInfo {
    /// `gitardo-vscode`.
    pub name: String,
    /// The version of the extension.
    pub version: String,
}

/// What the host can answer of the host questions.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
#[allow(
    clippy::struct_excessive_bools,
    reason = "each is a host question of its own that the extension says it answers, as the protocol names them"
)]
pub struct HostAbilities {
    /// It answers `host/clipboard`.
    pub clipboard: bool,
    /// It answers `host/githubToken`.
    pub github_token: bool,
    /// It answers `host/languageModel`.
    pub language_model: bool,
    /// It answers `host/openFile`, `host/saveFile` and `host/chooseFolder`.
    pub dialogs: bool,
    /// It answers `host/secret` and `host/storeSecret`.
    pub secrets: bool,
    /// It runs the setup commands of a worktree itself.
    pub terminal: bool,
}

/// How much the log records.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum LogLevelName {
    /// The steps, the failures and the warnings.
    #[default]
    Info,
    /// The reads too.
    Debug,
}

impl From<LogLevelName> for engine::LogLevel {
    fn from(level: LogLevelName) -> engine::LogLevel {
        match level {
            LogLevelName::Info => engine::LogLevel::Info,
            LogLevelName::Debug => engine::LogLevel::Debug,
        }
    }
}

/// The answer of `initialize`, and the data of its error when the major versions differ.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InitializeResult {
    /// The version of the protocol the engine speaks.
    pub protocol: String,
    /// Who the engine is.
    pub engine: EngineInfo,
}

/// Who the engine is.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EngineInfo {
    /// The version of `gitardo-engine`.
    pub version: String,
    /// What `git --version` says; none when git is not found.
    pub git: Option<String>,
}

/// The parameters of a request about one repository and nothing else.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RepositoryParams {
    /// The folder of the top of its working tree.
    pub repository: String,
}

/// The answer of `repository/open`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenResult {
    /// The state read.
    pub state: StateJson,
}

/// The parameters of `repository/changed`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChangedParams {
    /// The repository.
    pub repository: String,
    /// The paths the watchers saw change, from the top of the working tree, or in the git folder with `.git/`.
    #[serde(default)]
    pub paths: Vec<String>,
}

/// The parameters of `repository/state`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StateParams {
    /// The repository.
    pub repository: String,
    /// Its state, read again.
    pub state: StateJson,
    /// Why it was read again.
    pub reason: StateReason,
}

/// Why a repository was read again.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum StateReason {
    /// Files changed.
    Changed,
    /// A plan ran.
    Ran,
    /// It was opened.
    Opened,
}

/// The parameters of `$/cancel`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CancelParams {
    /// The id of the request cancelled.
    pub id: RequestId,
}

/// The parameters of `log/line`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LogLineParams {
    /// The repository it is about; none for a line about no repository.
    pub repository: Option<String>,
    /// The line.
    pub entry: LogEntryJson,
}

/// A line of the log.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LogEntryJson {
    /// When it started, in seconds since 1970.
    pub time: i64,
    /// What it is.
    pub kind: LogKindJson,
    /// The words: `git checkout -b fix/login HEAD`.
    pub line: String,
}

/// What a line of the log is: `"step"`, `"failure"`, `"warning"`, or `{ "kind": "read", "milliseconds": 12 }`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum LogKindJson {
    /// A step, a failure or a warning.
    Plain(PlainKind),
    /// A read, with how long it took.
    Read(ReadKind),
}

/// The kinds of line that carry nothing more.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PlainKind {
    /// A step of a plan.
    Step,
    /// What failed.
    Failure,
    /// What went wrong without stopping anything.
    Warning,
}

/// A read, with how long it took.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReadKind {
    /// Always `read`.
    pub kind: ReadWord,
    /// How long it took.
    pub milliseconds: u64,
}

/// The word `read`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ReadWord {
    /// A read.
    Read,
}

impl From<&LogEntry> for LogEntryJson {
    fn from(entry: &LogEntry) -> LogEntryJson {
        let kind = match entry.kind {
            LogKind::Step => LogKindJson::Plain(PlainKind::Step),
            LogKind::Failure => LogKindJson::Plain(PlainKind::Failure),
            LogKind::Warning => LogKindJson::Plain(PlainKind::Warning),
            LogKind::Read(took) => LogKindJson::Read(ReadKind {
                kind: ReadWord::Read,
                milliseconds: u64::try_from(took.as_millis()).unwrap_or(u64::MAX),
            }),
        };
        LogEntryJson {
            time: entry.time.0,
            kind,
            line: entry.line.clone(),
        }
    }
}

/// The parameters of `activity/changed`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActivityParams {
    /// All that runs, in every repository.
    pub running: Vec<ActivityJson>,
}

/// Something that runs.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActivityJson {
    /// Its number, unique while the engine runs.
    pub number: u64,
    /// What it does, in words.
    pub description: String,
    /// Why it was asked, in words.
    pub cause: String,
    /// When it started, in seconds since 1970.
    pub started: i64,
}

impl From<&ActivityEntry> for ActivityJson {
    fn from(entry: &ActivityEntry) -> ActivityJson {
        ActivityJson {
            number: entry.number,
            description: entry.description.as_str().to_owned(),
            cause: entry.cause.as_str().to_owned(),
            started: entry.started.0,
        }
    }
}
