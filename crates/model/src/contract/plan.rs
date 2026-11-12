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
    /// `delete <path>`; `run <command>`.
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
            Step::Link { to, .. } => format!("symlink {}", to.display()),
            Step::Copy { to, .. } => format!("copy {}", to.display()),
            Step::Unlink { path } => format!("unlink {}", path.display()),
            Step::Delete { path } => format!("delete {}", path.display()),
            Step::Run { command, .. } => format!("run {command}"),
        }
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
    /// Offer another feature, with the words of its button.
    Offer(FeatureId, Text),
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
        ];
        for (step, expected) in cases {
            assert_eq!(step.log_line(), expected);
        }
    }
}
