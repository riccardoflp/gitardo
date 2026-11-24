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
    /// The repositories of the folders of a workspace (`repo.read-repositories`, 1.7).
    pub const DISCOVER: &str = "repository/discover";
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
}

/// The version of the protocol the engine speaks: 1.1 added the messages of the features (stage 2), 1.2 those of the
/// journal (stage 3), 1.3 those of stage 4: the operations offered with their target and answers, such as **Delete
/// anyway** and **Remove anyway**, the targets of the conflicts, and whether the host runs the setup commands of new
/// worktrees in its terminals; 1.4 the reads but the graph's (stage 5, [`crate::reads`]); 1.5 the reads of the graph
/// (stage 7); 1.6 what area 6 of stage 4 needs, AI and GitHub ([`crate::ai`]): the host questions to VS Code's language
/// models, the reads of the draft of a commit message and of the question about a conflict, the follow-up that writes a
/// setting, and the target of a comparison; 1.7 what stage 10 needs once the extension has no git of its own:
/// `repository/discover` with the common git folder of each repository, a change of a diff and a file of a comparison
/// as targets, and `journal/record` gone, which no client sends any more; 1.8 what milestone M15 needs: the pages of
/// `read/graph.read-commits` sent as they are read (`stream`, `read/graph.rows`), `refresh` in
/// `read/repo.read-state`, and `read/repo.watch-changes`, which says which files of a burst ask for a read; 1.9 what
/// milestone M17 needs: `read/file-view.mermaid`, a diagram of Mermaid as an SVG for VS Code's preview of Markdown
/// ([`crate::mermaid`]); 1.10 what milestone M22 needs: `read/repo.worktree-summary`, what each repository of the
/// workspace, each of its worktrees and each of its submodules is on, with `repository/summaries` for those read again
/// and `repository/folderChanged` for what the extension's watchers saw ([`crate::summaries`]); 1.11 what milestones
/// M31, M32, M34, M35, M36, M37, M38 and M40 need: `read/help.ask`, a question of VS Code's Chat to Ask Gitardo, and
/// the titles of the features its answer may name in `host/languageModel` ([`crate::help`]); the question `openFolder`,
/// a folder chosen in the system's dialog, the validations `cloneFolder` and `count` ([`crate::feature`]), and
/// `shallow` in the state, where the history of a shallow clone ends; how git signs commits in the state,
/// `read/details.read-signature` and `read/commit.read-authors`; `read/tag.read-remote-tags` and the follow-up
/// `tagsOnRemote`; `read/range-diff.read-pairs`, the follow-up that opens the tab of what a rewrite changed, and the one
/// that offers an operation in the warning of a plan in its place ([`crate::range_diff`]); the validation of the number
/// of a pull request, and the host question of the token of a GitLab; `skipped` in the state and `locked` in a
/// worktree, the files hidden from git and the reason a worktree is locked; whether git's maintenance looks after the
/// repository, `notes` in the state, the commits with a note, and `note` in the details of a commit; 1.12 what
/// milestone M44 needs: `commitGraph` in the state, whether git keeps its commit-graph file of the repository and reads
/// it; 1.13 what milestone M48 needs: `target` in the follow-up `writeSetting`, where VS Code writes the setting, the
/// target `setupSettings` and `setup` in `feature/run`, what new worktrees get in the repository's folder, for the
/// settings of new worktrees saved by the engine's plans.
pub const VERSION: &str = "1.13";

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

/// The parameters of `read/repo.read-state` (1.8): the repository, and whether the read is a **Refresh**, or the read
/// of a repository chosen, which counts what is not committed in every other worktree again; any other read keeps the
/// count of a worktree whose own git folder did not change since the read before (M15).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReadStateParams {
    /// The folder of the top of its working tree.
    pub repository: String,
    /// Whether every other worktree is counted again.
    #[serde(default)]
    pub refresh: bool,
}

/// The parameters of `repository/discover`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscoverParams {
    /// The folders of the workspace, in their order.
    pub folders: Vec<String>,
}

/// The answer of `repository/discover`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscoverResult {
    /// The repositories, in the order a window shows them.
    pub repositories: Vec<DiscoveredRepositoryJson>,
}

/// A repository found in the folders of a workspace.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscoveredRepositoryJson {
    /// The top folder of its working tree, as git names it.
    pub folder: String,
    /// The top folder of the repository it is a submodule of; none for a repository of its own.
    pub superproject: Option<String>,
    /// The git folder its worktrees share (`repo.common-folder`), which tells the worktrees of one repository; none
    /// when git could not say it.
    pub common_folder: Option<String>,
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
