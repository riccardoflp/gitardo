//! What a feature decides to do: the steps the engine runs, how dangerous they are, how Undo takes them back, and
//! what follows.

use std::ffi::OsString;
use std::path::PathBuf;

use super::{FeatureId, Text, UndoRecord};

/// What a feature has decided to do. The steps are exactly what the preview shows and the log records.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Plan {
    /// What it does, in words: "Create branch feature/login at 3f2a1c9".
    pub title: Text,
    /// What runs, in order.
    pub steps: Vec<Step>,
    /// Whether the user is asked first, and why.
    pub danger: Danger,
    /// How Undo takes it back.
    pub undo: Undo,
    /// What happens once it succeeded.
    pub after: Vec<FollowUp>,
    /// What happens when a step fails.
    pub on_failure: Vec<FollowUp>,
}

/// One thing a plan does. Nothing in Gitardo changes the disk or the repository but these, run by the engine.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Step {
    /// Runs git with these arguments, in a folder, with something on its standard input.
    Git {
        /// The arguments after `git`.
        arguments: Vec<OsString>,
        /// The folder git runs in.
        directory: PathBuf,
        /// What git reads on its standard input, if anything.
        input: Option<Vec<u8>>,
    },
    /// Writes a file, making the folders it needs.
    Write {
        /// The file.
        path: PathBuf,
        /// What it holds after.
        contents: Vec<u8>,
    },
    /// Makes `from` a link to the folder `to`.
    Link {
        /// The link made.
        from: PathBuf,
        /// The folder it points at.
        to: PathBuf,
    },
    /// Copies a file or a folder.
    Copy {
        /// What is copied.
        from: PathBuf,
        /// Where the copy goes.
        to: PathBuf,
    },
    /// Removes a link, never what it points at.
    Unlink {
        /// The link.
        path: PathBuf,
    },
    /// Deletes a file or a folder.
    Delete {
        /// The file or the folder.
        path: PathBuf,
    },
    /// Runs a command of the user's, through the system's shell.
    Run {
        /// The command, as the user wrote it.
        command: String,
        /// The folder it runs in.
        directory: PathBuf,
    },
    /// Runs a command of the user's in a shell as a task of the source **New worktrees** of the Tasks tab
    /// (`worktree-setup.command-steps`): what it prints is shown in its terminal as it comes, what it asks can be typed
    /// there, and what the plan does once it succeeded waits for it.
    Task {
        /// The command, as the user wrote it.
        command: String,
        /// The folder it runs in.
        directory: PathBuf,
        /// The name the Tasks tab lists it under: the worktree it runs in, as the views name it.
        name: String,
    },
    /// Makes a folder and the folders that lead to it, when they are missing.
    MakeFolder {
        /// The folder.
        path: PathBuf,
    },
    /// Deletes a folder only when it is empty by then; a folder with something in it, or none, is left as it is.
    DeleteEmptyFolder {
        /// The folder.
        path: PathBuf,
    },
    /// Writes a warning to the log, and changes nothing: what the steps after it do and why.
    Warn {
        /// The sentence.
        words: String,
    },
}

impl Step {
    /// A git step with no standard input.
    #[must_use]
    pub fn git<Argument: Into<OsString>>(directory: PathBuf, arguments: impl IntoIterator<Item = Argument>) -> Self {
        Step::Git {
            arguments: arguments.into_iter().map(Into::into).collect(),
            directory,
            input: None,
        }
    }

    /// The line of the command log for this step, in the fixed form of `log.record`: the git command as it runs, with
    /// the arguments that hold spaces in quotes; `write <path>`; `symlink <folder>`; `copy <path>`; `unlink <path>`;
    /// `delete <path>`; `mkdir <path>`; `run <command>`, for a task too; and the sentence of a warning.
    #[must_use]
    pub fn log_line(&self) -> String {
        match self {
            Step::Git { arguments, .. } => {
                let mut line = String::from("git");
                for argument in arguments {
                    line.push(' ');
                    line.push_str(&quoted(&argument.to_string_lossy()));
                }
                line
            }
            Step::Write { path, .. } => format!("write {}", path.display()),
            Step::Link { .. } | Step::Copy { .. } => {
                let verb = if matches!(self, Step::Link { .. }) {
                    "symlink"
                } else {
                    "copy"
                };
                let made = self.made_path().unwrap_or_default();
                let words: Vec<String> = made
                    .components()
                    .map(|component| component.as_os_str().to_string_lossy().into_owned())
                    .collect();
                format!("{verb} {}", words.join("/"))
            }
            Step::Unlink { path } => format!("unlink {}", path.display()),
            Step::Delete { path } | Step::DeleteEmptyFolder { path } => format!("delete {}", path.display()),
            Step::MakeFolder { path } => format!("mkdir {}", path.display()),
            Step::Run { command, .. } | Step::Task { command, .. } => format!("run {command}"),
            Step::Warn { words } => words.clone(),
        }
    }
}

impl Step {
    /// The path a link or a copy makes, from the top of the worktree it makes it in: what the folder it comes from and
    /// the one it goes to end with alike, `node_modules` for a link from `repository/node_modules` to
    /// `repository.worktrees/feature/node_modules` (`worktree-setup.link-and-copy-steps`). None for other steps.
    #[must_use]
    pub fn made_path(&self) -> Option<PathBuf> {
        let (made, source) = match self {
            Step::Link { from, to } => (from, to),
            Step::Copy { from, to } => (to, from),
            _ => return None, // Only a link and a copy make a path from another place.
        };
        let made_parts: Vec<_> = made.components().rev().collect();
        let source_parts: Vec<_> = source.components().rev().collect();
        let alike = made_parts
            .iter()
            .zip(&source_parts)
            .take_while(|(made, source)| made == source)
            .count();
        let mut path: Vec<_> = made_parts.into_iter().take(alike).collect();
        path.reverse();
        Some(path.into_iter().collect())
    }
}

/// An argument as it can be typed again in a terminal: in double quotes when it is empty or holds a space, a quote
/// or a backslash.
fn quoted(argument: &str) -> String {
    let needs_quotes = argument.is_empty()
        || argument.contains(|character: char| character.is_whitespace() || "\"\\".contains(character));
    if !needs_quotes {
        return argument.to_owned();
    }
    let escaped = argument.replace('\\', "\\\\").replace('"', "\\\"");
    format!("\"{escaped}\"")
}

/// Whether the user is asked before a plan runs, and why. The engine decides from this and `commandPreview`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Danger {
    /// Nothing can be lost.
    None,
    /// Nothing can be lost, and the user agreed to it in the feature's own question, which was its confirmation: it is
    /// not shown again, whatever `commandPreview` says.
    Agreed,
    /// The user is asked first, for this reason.
    Confirm(Text),
    /// Work can be lost: the user is asked first, with this reason as a warning.
    Destructive(Text),
}

/// How Undo takes a plan back.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Undo {
    /// By the reflog of `HEAD` and of the branch, which git keeps (`undo.last`).
    ByReflog,
    /// It cannot, for this reason.
    Impossible(Text),
    /// By what the engine's journal keeps of it, since the reflog does not see it (`undo.journal`).
    Journal(UndoRecord),
    /// It is the Undo of the entry of the journal with this number: once it ran, that entry is gone, and Undo again
    /// goes by the reflog.
    TakesBack(u64),
}

/// What happens after a plan ran, or after one of its steps failed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FollowUp {
    /// Say something to the user.
    Say(Text),
    /// Remember the folder that held a clone, which the dialog of the next clone starts in (`cloneFolder`).
    RememberCloneFolder(PathBuf),
    /// Open a repository made from the window in a tab and show it: its folder added to the workspace, or made the
    /// workspace when none is open (`repo.clone`, `repo.init`).
    OpenRepository(PathBuf),
    /// Select the commit `HEAD` is at.
    SelectHead,
    /// Select a commit in the timeline, going to it as `graph.show-in-graph` does.
    SelectCommit(crate::CommitId),
    /// Open the page of a commit (`commit-page.view`), or show the one it has.
    OpenCommitPage(crate::CommitId),
    /// Empty the commit message box and untick **Amend last commit**.
    EmptyCommitMessage,
    /// Read the pull requests of GitHub again, when the token is known.
    ReadPullRequests,
    /// Keep this GitHub token in the credential store of the system, for `github.com`, before the pull requests are
    /// read with it (`github.load-pull-requests`).
    KeepGitHubToken(crate::github::Token),
    /// Keep this token of a GitLab in the credential store of the system, for its host name, before the merge requests
    /// are read with it (`hosting.load-merge-requests`).
    KeepHostingToken(String, crate::github::Token),
    /// Keep this key of the AI provider in the credential store of the system, under the provider's name
    /// (`ai.choose-model`).
    KeepAiKey(crate::ai::AiProvider, crate::ai::Key),
    /// Offer another feature, with the words of its button.
    Offer(FeatureId, Text),
    /// Put this text on the clipboard.
    CopyToClipboard(String),
    /// Open this address in the browser of the system (`compare.open-on-github`).
    OpenUrl(String),
    /// Once the repository is read again, say whether the pull left the changes it set aside in a stash
    /// (`stash.autostash-left`).
    CheckAutostash,
    /// When a step failed: run this feature at once, on the same target, which gets past the stop when it can
    /// (`cherry-pick.skip-empty`); when it does nothing, the failure is shown.
    Recover(FeatureId),
    /// Select the commit as many first parents before `HEAD` as this, once the plan ran: 0 is `HEAD`.
    SelectBeforeHead(usize),
    /// Put the message git prepared for the next commit (`SQUASH_MSG` in the git folder) in the commit box, when the
    /// box is empty.
    PreparedMessage,
    /// Put this message in the commit box, as the message of the next commit.
    FillCommitMessage(String),
    /// Delete this folder of the files of an interactive rebase once the plan ran, unless the rebase stopped, whose
    /// steps still to run read them: the engine looks for `rebase-merge` in the git folder of the worktree.
    DeleteWhenRebaseEnded(std::path::PathBuf),
    /// A step the engine runs once the steps of the plan ended, whether they succeeded or not, logged as they are:
    /// the temporary file of a patch deleted (`patch.apply`).
    Cleanup(Step),
    /// Open the comparison page of two sides, or show the one they have (`compare.page`).
    OpenComparison(Box<crate::compare::ComparisonTarget>),
    /// Open the page of what a rewrite changed, commit by commit, in place of the one shown before (`range-diff.page`).
    OpenRangeDiff(Box<crate::range_diff::RangeDiffTarget>),
    /// Offered in the confirmation of the plan as a button of its own, which closes it, runs nothing and runs this in
    /// its place: **Compare first** in the warning of a force push (`preview.confirm`, Q1140).
    OfferBeforeRun(Box<OfferedRun>),
    /// Once the repository is read again, say which upstream of a local branch was force pushed, as a fetch found it,
    /// with **See what changed** (`range-diff.compare`).
    CheckForcePushed,
    /// Show the comparison of two commits in the inspector, in place of the commit selected (`compare.in-place`).
    CompareInPlace(Box<crate::compare::InPlaceSides>),
    /// Once the repository is read again, say which commit is the first bad one when a bisect found it
    /// (`bisect.mark`).
    CheckBisect,
    /// Open the review of the resolutions of these files of the working tree (`review.page`).
    OpenReview(Vec<String>),
    /// Close the Merge page of a file, by its path from the top of the working tree, showing nothing in its place
    /// (`merge-view.mark-resolved`).
    CloseMergePage(String),
    /// Open a folder in a new window of Gitardo Desktop, as a repository of its own
    /// (`submodule.open-in-new-window`, `worktree.open-in-new-window`).
    OpenWindow(PathBuf),
    /// Point the views of this window at another folder: a worktree of the repository (`worktree.show-in-gitardo`) or
    /// a submodule (`submodule.show-in-gitardo`).
    ShowFolder(PathBuf),
    /// Offer another operation beside what is said, on a target of its own and with answers given.
    OfferRun(Box<OfferedRun>),
    /// When a step failed because git does not clone submodules from a folder on disk, offer the plan again from that
    /// step, allowed for those commands only (`submodule.allow-file-protocol`).
    AllowFileProtocol,
    /// When a step failed and git said these words, offer another plan in its place, asked first as its danger says:
    /// `git submodule deinit --force` once git refused for changes not committed (`submodule.deinitialize`).
    OfferWhenFailed(Box<FailureOffer>),
    /// Show the timeline narrowed to the history of a file, of some lines or of a folder; none shows it with no
    /// narrowing (`file-history.file`, `file-history.lines`, `file-history.folder`).
    ShowHistory(Option<Box<crate::file_history::HistoryOf>>),
    /// Show this blame beside the lines of the File page of a file (`blame.toggle-file`).
    ShowBlame(Box<crate::blame::BlameOfFile>),
    /// Hide the blame of the File page of a file, by its path and its revision, until the page is closed
    /// (`blame.toggle-file`).
    HideBlame(String, Option<crate::CommitId>),
    /// Open a file in a File page at a line, with its blame shown (`blame.previous`).
    OpenFile(Box<crate::blame::FileToOpen>),
    /// Remember the values given for the inputs of `branches.format`, by their ids, as the ones used last in this
    /// workspace (`branch.name-from-format`).
    RememberInputs(std::collections::BTreeMap<String, String>),
    /// Open a file in a File page to edit it, at a line from 1: by its path from the top of the working tree, or by
    /// its whole path for a file outside it, such as the user's settings file (`worktree-setup.edit-settings-file`).
    EditFile(String, usize),
    /// Open the Diff page of a file between two sides (`details.compare-file-with`).
    OpenFileDiff(Box<crate::compare::FileSides>),
    /// Trust this folder from now on, at once, whether the plan succeeded or not (`trust.ask`).
    TrustFolder(PathBuf),
    /// Stop trusting these folders (`trust.manage`).
    ForgetTrustedFolders(Vec<PathBuf>),
    /// Write a setting in the settings of the host, in the place it names, in place of a step that writes a settings
    /// file: under the extension, VS Code's (`ai.choose-model`, `worktree-setup.save`, Q399). Never in Gitardo Desktop.
    WriteSetting(Box<SettingWritten>),
    /// Run tasks in the Tasks tab, each in a terminal of its own, after those it depends on (`tasks.run`,
    /// `tasks.run-group`).
    RunTasks(Box<crate::tasks::TasksToRun>),
    /// Stop tasks that run in the Tasks tab (`tasks.stop`).
    StopTasks(Vec<crate::tasks::TaskReference>),
    /// Change the marks of the tags a remote lacks as the operation changed the tags on it, without asking it again
    /// (`tag.remote-marks`).
    TagsOnRemote(Box<crate::tag::TagsOnRemote>),
}

/// A setting the host writes in its settings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SettingWritten {
    /// Its name, as the desktop's settings file names it: `ai.model`; VS Code's own settings with their `git.` prefix,
    /// `git.worktreeSymlinkFolders`.
    pub key: String,
    /// Its value as JSON text, `"claude-test"`; none to remove the setting from that place, so that what is set
    /// elsewhere, or its default, applies.
    pub value: Option<String>,
    /// The place of the host's settings it is written in.
    pub target: SettingTarget,
}

/// A place of the host's settings a setting is written in (Q399).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SettingTarget {
    /// The settings of the workspace folder that has the repository: its `.vscode/settings.json`.
    #[default]
    WorkspaceFolder,
    /// The settings of the workspace: its `.code-workspace` file, or the folder's when the workspace is one folder.
    Workspace,
    /// The user's settings, for every workspace.
    User,
}

/// A plan offered in place of one whose step failed, when git said some words.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FailureOffer {
    /// Words of what git said that call for the offer: `contains local modifications`.
    pub when: String,
    /// The plan offered, which runs in place of what was left of the one that failed.
    pub plan: Plan,
}

/// An operation offered beside what is said once a plan ran, or once it failed: **Open in new window** after a worktree
/// is created, **Remove anyway** after git refused to remove one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OfferedRun {
    /// The words of its button.
    pub label: Text,
    /// The operation.
    pub feature: FeatureId,
    /// What it runs on.
    pub target: super::Target,
    /// The answers it is given before it asks.
    pub answers: super::Answers,
    /// Among the follow-ups of a failure: offered only when what git said holds one of these; any failure when empty.
    pub when_said: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::Step;
    use std::path::PathBuf;

    #[test]
    fn writes_each_step_as_the_log_shows_it() {
        let folder = PathBuf::from("repository");
        let cases = [
            (
                Step::git(folder.clone(), ["checkout", "-b", "fix/login", "HEAD"]),
                "git checkout -b fix/login HEAD",
            ),
            (
                Step::git(folder.clone(), ["commit", "-m", "Fix the parser"]),
                "git commit -m \"Fix the parser\"",
            ),
            (
                Step::git(folder.clone(), ["commit", "-m", "say \"hi\""]),
                "git commit -m \"say \\\"hi\\\"\"",
            ),
            (Step::git(folder.clone(), ["config", "a", ""]), "git config a \"\""),
            (
                Step::Run {
                    command: String::from("npm ci"),
                    directory: folder,
                },
                "run npm ci",
            ),
            (
                Step::Delete {
                    path: PathBuf::from("old"),
                },
                "delete old",
            ),
            (
                Step::Link {
                    from: PathBuf::from("trees/feature/packages/api/node_modules"),
                    to: PathBuf::from("repository/packages/api/node_modules"),
                },
                "symlink packages/api/node_modules",
            ),
            (
                Step::Copy {
                    from: PathBuf::from("repository/.env"),
                    to: PathBuf::from("trees/feature/.env"),
                },
                "copy .env",
            ),
        ];
        for (step, expected) in cases {
            assert_eq!(step.log_line(), expected);
        }
    }
}
