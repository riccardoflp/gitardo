//! The tasks a worktree defines, as the Tasks tab and **Run Task…** list them (`tasks.read`): those of
//! `.vscode/tasks.json` and the scripts of each `package.json`, by source.

use std::fmt;
use std::path::PathBuf;

/// Where a task comes from.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum TaskSource {
    /// `.vscode/tasks.json` of the worktree shown: **Workspace**.
    Workspace,
    /// The scripts of a `package.json`: **npm**, with the folder of the file from the top of the worktree, with `/`,
    /// `extension`; empty for the file at the top.
    Npm(String),
    /// The commands of a new worktree (`worktree-setup.command-steps`): **New worktrees**, with the name of the
    /// worktree. They run once, when the worktree is made: no group has them and no file of the worktree defines them.
    NewWorktree(String),
}

/// How a task is named by a group, by the task that depends on it and by what runs it: its source and its name,
/// `Workspace` `build` or `npm` `extension` `test`. The same in every worktree of the repository.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TaskReference {
    /// Where it comes from.
    pub source: TaskSource,
    /// Its `label` in `tasks.json`, or the name of the script.
    pub name: String,
}

impl TaskReference {
    /// A task of `.vscode/tasks.json`, by its label.
    #[must_use]
    pub fn workspace(name: &str) -> Self {
        TaskReference {
            source: TaskSource::Workspace,
            name: name.to_owned(),
        }
    }

    /// A script of the `package.json` of a folder, from the top of the worktree with `/`; empty for the top.
    #[must_use]
    pub fn npm(folder: &str, script: &str) -> Self {
        TaskReference {
            source: TaskSource::Npm(folder.to_owned()),
            name: script.to_owned(),
        }
    }

    /// A command of a new worktree, by the name of the worktree and the name the command is listed by.
    #[must_use]
    pub fn new_worktree(worktree: &str, name: &str) -> Self {
        TaskReference {
            source: TaskSource::NewWorktree(worktree.to_owned()),
            name: name.to_owned(),
        }
    }

    /// Whether it is a command of a new worktree, which runs once and is neither run again nor put in a group.
    #[must_use]
    pub fn is_of_new_worktree(&self) -> bool {
        matches!(self.source, TaskSource::NewWorktree(_))
    }

    /// The name VS Code gives the task, which is how Run My Tasks names it in a group: the label of a task of
    /// `tasks.json`, `build` for a script of the `package.json` at the top, `build - extension` for one of a folder.
    #[must_use]
    pub fn name_in_vs_code(&self) -> String {
        match &self.source {
            TaskSource::Npm(folder) if !folder.is_empty() => format!("{} - {folder}", self.name),
            TaskSource::Workspace | TaskSource::Npm(_) | TaskSource::NewWorktree(_) => self.name.clone(),
        }
    }
}

impl fmt::Display for TaskReference {
    /// The task as a sentence names it: `build` for a task of `tasks.json`, `npm: build` and `npm: build - extension`
    /// for a script, as VS Code names them.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.source {
            TaskSource::Workspace | TaskSource::NewWorktree(_) => formatter.write_str(&self.name),
            TaskSource::Npm(_) => write!(formatter, "npm: {}", self.name_in_vs_code()),
        }
    }
}

/// How a task runs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TaskKind {
    /// `shell`: its command line runs in a shell.
    Shell,
    /// `process`: its command runs with its arguments, with no shell.
    Process,
    /// Another `type`, of VS Code or of one of its extensions, which Gitardo lists and does not run: `cargo`.
    Other(String),
}

/// How a word of a command is quoted when it goes to a shell (`quoting` in `tasks.json`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Quoting {
    /// Written as a plain string: quoted as the shell needs only when it holds a space.
    #[default]
    Plain,
    /// `escape`: its spaces and special characters escaped with the escape character of the shell.
    Escape,
    /// `strong`: in the quotes of the shell that leave everything as written.
    Strong,
    /// `weak`: in the quotes of the shell that still expand its variables.
    Weak,
}

/// A word of what a task runs: its command or one of its arguments.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Word {
    /// The word, its variables resolved.
    pub value: String,
    /// How it is quoted for a shell.
    pub quoting: Quoting,
}

impl Word {
    /// A word written as a plain string.
    #[must_use]
    pub fn plain(value: &str) -> Self {
        Word {
            value: value.to_owned(),
            quoting: Quoting::Plain,
        }
    }
}

/// The shell a task asks for (`options.shell`), in place of the one of `terminal.shellPath`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskShell {
    /// Its program.
    pub executable: String,
    /// The arguments it is given before the command line; none takes those the shell needs to run a command.
    pub arguments: Option<Vec<String>>,
}

/// The `group` of a task in `tasks.json`, which VS Code runs its build and its tests by; not a group of the Tasks tab.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskCategory {
    /// `build`, `test`, `none` or another name.
    pub kind: String,
    /// Whether it is the default task of its kind.
    pub is_default: bool,
}

/// Whether the tasks a task depends on run one after the other (`dependsOrder`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DependsOrder {
    /// All at once.
    #[default]
    Parallel,
    /// `sequence`: each once the one before it ended with exit code 0, or is ready.
    Sequence,
}

/// What says a background task is ready, from the `background` of its problem matcher.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Background {
    /// The regular expression of a line that says it started working, when the matcher has one.
    pub begins_pattern: Option<String>,
    /// The regular expression of a line that says it is ready, when the matcher has one.
    pub ends_pattern: Option<String>,
}

/// A task of the worktree shown.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Task {
    /// How groups and the tasks that depend on it name it.
    pub reference: TaskReference,
    /// How it runs.
    pub kind: TaskKind,
    /// What it runs, for the system Gitardo runs on; none for a task that only runs those it depends on.
    pub command: Option<Word>,
    /// The arguments of the command.
    pub arguments: Vec<Word>,
    /// The text of the script, for a script of a `package.json`, shown under its name.
    pub script: Option<String>,
    /// The folder it runs in: `options.cwd`, the folder of the `package.json`, or the worktree shown.
    pub folder: PathBuf,
    /// `options.env`, in the order written, set over the environment of Gitardo.
    pub environment: Vec<(String, String)>,
    /// `options.shell`, when the task asks for a shell of its own.
    pub shell: Option<TaskShell>,
    /// Its `group` in `tasks.json`.
    pub category: Option<TaskCategory>,
    /// The tasks that run before it (`dependsOn`).
    pub depends_on: Vec<TaskReference>,
    /// Whether those run one after the other.
    pub depends_order: DependsOrder,
    /// `hide`: it is not listed, but runs as one another depends on.
    pub is_hidden: bool,
    /// What says it is ready, for a task with `isBackground`; none for a task others wait for the end of.
    pub background: Option<Background>,
    /// The variables it uses that Gitardo cannot resolve, each once, as written: `${input:name}`.
    pub unresolved: Vec<String>,
    /// The line of its label in `.vscode/tasks.json`, from 1; none for a script.
    pub line: Option<usize>,
}

/// Why Gitardo lists a task and does not run it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NotRunnable {
    /// It is of a `type` only VS Code or one of its extensions runs.
    OtherKind(String),
    /// It uses variables Gitardo cannot resolve.
    Unresolved(Vec<String>),
}

impl NotRunnable {
    /// What the row of the task says: **This kind of task runs in VS Code only.**, or the variables it cannot be run
    /// with.
    #[must_use]
    pub fn said(&self) -> String {
        match self {
            NotRunnable::OtherKind(_) => String::from("This kind of task runs in VS Code only."),
            NotRunnable::Unresolved(variables) => {
                let named: Vec<String> = variables.iter().map(|variable| format!("`{variable}`")).collect();
                format!(
                    "It cannot be run with {}, which only VS Code resolves.",
                    named.join(", ")
                )
            }
        }
    }
}

impl Task {
    /// A task of `.vscode/tasks.json` with a label and nothing else, of the kind `process`, to fill in.
    #[must_use]
    pub fn named(reference: TaskReference, folder: PathBuf) -> Self {
        Task {
            reference,
            kind: TaskKind::Process,
            command: None,
            arguments: Vec::new(),
            script: None,
            folder,
            environment: Vec::new(),
            shell: None,
            category: None,
            depends_on: Vec::new(),
            depends_order: DependsOrder::Parallel,
            is_hidden: false,
            background: None,
            unresolved: Vec::new(),
            line: None,
        }
    }

    /// Why the task is listed dimmed and not run; none for a task Gitardo runs.
    #[must_use]
    pub fn not_runnable(&self) -> Option<NotRunnable> {
        match &self.kind {
            TaskKind::Other(kind) => Some(NotRunnable::OtherKind(kind.clone())),
            TaskKind::Shell | TaskKind::Process if !self.unresolved.is_empty() => {
                Some(NotRunnable::Unresolved(self.unresolved.clone()))
            }
            TaskKind::Shell | TaskKind::Process => None,
        }
    }

    /// The command with its arguments on one line, as the row of the task shows it after its name and **Copy
    /// command** copies it; empty for a task that only runs those it depends on.
    #[must_use]
    pub fn command_line(&self) -> String {
        let words: Vec<&str> = self
            .command
            .iter()
            .chain(&self.arguments)
            .map(|word| word.value.as_str())
            .collect();
        words.join(" ")
    }
}

/// The program that runs the scripts of a `package.json`, as the lock file beside it names it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PackageManager {
    /// `npm`, with `package-lock.json` or no lock file.
    #[default]
    Npm,
    /// `pnpm`, with `pnpm-lock.yaml`.
    Pnpm,
    /// `yarn`, with `yarn.lock`.
    Yarn,
    /// `bun`, with `bun.lockb` or `bun.lock`.
    Bun,
}

impl PackageManager {
    /// The program, as it is run: `pnpm`.
    #[must_use]
    pub fn program(self) -> &'static str {
        match self {
            PackageManager::Npm => "npm",
            PackageManager::Pnpm => "pnpm",
            PackageManager::Yarn => "yarn",
            PackageManager::Bun => "bun",
        }
    }
}

/// The scripts of one `package.json`, a group of their own under **npm**.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageScripts {
    /// The folder of the file from the top of the worktree, with `/`; empty for the file at the top.
    pub folder: String,
    /// The name of the group: the folder, or the name of the folder of the worktree for the file at the top.
    pub name: String,
    /// The program that runs them.
    pub manager: PackageManager,
    /// The scripts, in the order of the file.
    pub scripts: Vec<Task>,
}

/// A file of tasks that could not be read, said at the top of the list with **Open the file**.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskFileProblem {
    /// The file from the top of the worktree, with `/`: `.vscode/tasks.json`.
    pub file: String,
    /// Why, as the reader of JSON says it, without the capital and the full stop of a sentence: `at line 4, column
    /// 5, a comma or a closing brace was expected`.
    pub why: String,
}

impl TaskFileProblem {
    /// **`.vscode/tasks.json` could not be read: `<why>`.**
    #[must_use]
    pub fn said(&self) -> String {
        format!("`{}` could not be read: {}.", self.file, self.why)
    }
}

/// The tasks the worktree shown defines, by source.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Tasks {
    /// **Workspace**: the tasks of `.vscode/tasks.json`, in the order of the file, those with `hide` included.
    pub workspace: Vec<Task>,
    /// **npm**: the scripts of each `package.json`, the file at the top first, then by folder.
    pub packages: Vec<PackageScripts>,
    /// The files that could not be read, which list nothing.
    pub problems: Vec<TaskFileProblem>,
}

impl Tasks {
    /// Every task, those with `hide` included: **Workspace** first, then the scripts.
    pub fn all(&self) -> impl Iterator<Item = &Task> {
        let scripts = self.packages.iter().flat_map(|package| &package.scripts);
        self.workspace.iter().chain(scripts)
    }

    /// The task a group or another task names, when the worktree has it.
    #[must_use]
    pub fn find(&self, reference: &TaskReference) -> Option<&Task> {
        self.all().find(|task| task.reference == *reference)
    }

    /// The task Run My Tasks means by a name in a group: among the tasks VS Code names so, the one of `tasks.json`
    /// first, then the first script.
    #[must_use]
    pub fn named_in_vs_code(&self, name: &str) -> Option<&Task> {
        let of_workspace = self.workspace.iter().find(|task| task.reference.name == name);
        of_workspace.or_else(|| self.all().find(|task| task.reference.name_in_vs_code() == name))
    }
}
