//! Running the steps of a plan, one by one, each written to the log before it starts, and what git printed under it once
//! it succeeded. A step that fails stops the plan (rule R4.5), and so does cancelling it: the process of the step
//! running is killed, and no step after it starts.

use std::fmt;
use std::fs;
use std::io::Read;
use std::path::Path;
use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;

use git::{CancelToken, Git, GitError};
use model::contract::{FollowUp, Step};

use crate::clock::Clock;
use crate::log::CommandLog;
use crate::setup::{self, Room, room};

/// A step that failed, with the steps that ran before it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StepFailure {
    /// The lines of the steps that ran, in order.
    pub ran: Vec<String>,
    /// The line of the step that failed.
    pub failed: String,
    /// What git or the system said, or that it was cancelled.
    pub said: String,
    /// Whether it stopped because the plan was cancelled, not because the step failed.
    pub cancelled: bool,
    /// The title of the plan it is a step of, as the engine ran it: *Merge feature into main*.
    pub plan: String,
    /// What the plan has the window do when it fails, past what the engine does itself: the operations offered for
    /// what git said, such as **Remove anyway**, and a new window opened when the external editor did not start.
    pub on_failure: Box<[FollowUp]>,
}

impl fmt::Display for StepFailure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.cancelled {
            return write!(formatter, "`{}` was cancelled", self.failed);
        }
        write!(formatter, "`{}` failed: {}", self.failed, self.said.trim())
    }
}

/// Where steps run, what they write to, and what cancels them.
pub(crate) struct StepRunner<'a> {
    pub(crate) git: &'a Git,
    pub(crate) log: &'a mut CommandLog,
    pub(crate) clock: &'a dyn Clock,
    pub(crate) cancel: &'a CancelToken,
    /// The links and copies that failed, which the steps after them went on from.
    pub(crate) went_on_after: Vec<StepFailure>,
    /// What says the progress of the plan, for a git step that asks git for it with `--progress`.
    pub(crate) progress: Option<&'a crate::ProgressReporter>,
}

/// Why a step did not run to its end.
enum StepError {
    /// It failed, and git or the system said this.
    Failed(String),
    /// The plan was cancelled while it ran: its process was killed.
    Cancelled,
}

impl StepError {
    fn failed(error: &dyn fmt::Display) -> Self {
        StepError::Failed(error.to_string())
    }
}

impl StepRunner<'_> {
    /// Runs the steps in order, and stops at the first that fails or when the plan is cancelled. A step that does not
    /// start because the plan was cancelled before it is not logged. A link or a copy is the exception
    /// (`worktree-setup.link-and-copy-steps`): one that has no room in the worktree is not made and the log says why,
    /// and one that fails is kept in [`StepRunner::went_on_after`] while the steps after it go on.
    pub(crate) fn run(&mut self, steps: &[Step]) -> Result<Vec<String>, StepFailure> {
        let mut ran = Vec::new();
        for step in steps {
            if self.cancel.is_cancelled() {
                return Err(StepFailure {
                    ran,
                    failed: step.log_line(),
                    said: String::from("Cancelled before it started."),
                    cancelled: true,
                    plan: String::new(),
                    on_failure: Box::default(),
                });
            }
            let is_setup = matches!(step, Step::Link { .. } | Step::Copy { .. });
            if is_setup && let Room::Taken(words) = room(step) {
                self.log.record(self.clock.now(), &Step::Warn { words });
                continue;
            }
            self.log.record(self.clock.now(), step);
            let (said, cancelled) = match run_step(self.git, step, (self.cancel, self.progress)) {
                Ok(printed) => {
                    self.log.record_output(self.clock.now(), &printed);
                    ran.push(step.log_line());
                    continue;
                }
                Err(StepError::Failed(said)) => match self.held(step, said) {
                    Ok(()) => {
                        ran.push(step.log_line());
                        continue;
                    }
                    Err(said) => (said, false),
                },
                Err(StepError::Cancelled) => (String::from("Cancelled."), true),
            };
            self.log.record_failure(self.clock.now(), &said);
            let failure = StepFailure {
                ran: ran.clone(),
                failed: step.log_line(),
                said,
                cancelled,
                plan: String::new(),
                on_failure: Box::default(),
            };
            if is_setup && !cancelled {
                self.went_on_after.push(failure);
                continue;
            }
            return Err(failure);
        }
        Ok(ran)
    }
}

impl StepRunner<'_> {
    /// Gets past a git step on the folder of a worktree that failed because Windows holds the folder, as
    /// `worktree.recover-held-folder` says: what git said when it cannot.
    fn held(&mut self, step: &Step, said: String) -> Result<(), String> {
        let Step::Git {
            arguments, directory, ..
        } = step
        else {
            return Err(said);
        };
        let words: Vec<String> = arguments
            .iter()
            .map(|argument| argument.to_string_lossy().into_owned())
            .collect();
        let git = self.git.in_directory(directory);
        let cancel = self.cancel;
        let (log, clock) = (&mut *self.log, self.clock);
        crate::held::recover(
            &words,
            said,
            &mut crate::held::Attempts {
                is_windows: cfg!(windows),
                again: &mut || match run_step(&git, step, (cancel, None)) {
                    Ok(_printed) => Ok(()),
                    Err(StepError::Failed(said)) => Err(said),
                    Err(StepError::Cancelled) => Err(String::from("Cancelled.")),
                },
                is_listed: &mut |folder| crate::held::is_listed(&git, folder),
                delete: &mut |folder| delete(folder).map_err(|error| error.to_string()),
                wait: &mut |milliseconds| thread::sleep(Duration::from_millis(milliseconds)),
                note: &mut |logged| log.record(clock.now(), logged),
            },
        )
    }
}

/// Runs one step: what git printed, on its output then on its error output, when it succeeds (nothing for a step that
/// is not git's); what went wrong, in the words of git or of the system, when it fails. A git step that asks git for its
/// progress says the last line of it as git prints it (`activity.describe-task`).
fn run_step(
    git: &Git,
    step: &Step,
    (cancel, progress): (&CancelToken, Option<&crate::ProgressReporter>),
) -> Result<String, StepError> {
    match step {
        Step::Git {
            arguments,
            directory,
            input,
        } => {
            let git = git.in_directory(directory);
            let reporter = progress.filter(|_| arguments.iter().any(|argument| argument == "--progress"));
            let ran = match reporter {
                Some(reporter) => git.run_step_reporting(arguments, input.as_deref(), &mut |printed| {
                    reporter.report(model::activity::progress_line(printed));
                }),
                None => git.run_step(arguments, input.as_deref()),
            };
            let output = match ran {
                Ok(output) => output,
                Err(GitError::Cancelled { .. }) => return Err(StepError::Cancelled),
                Err(error) => return Err(StepError::failed(&error)),
            };
            if output.is_success() {
                let mut printed = String::from_utf8_lossy(&output.stdout).into_owned();
                if !printed.is_empty() && !printed.ends_with('\n') {
                    printed.push('\n');
                }
                printed.push_str(&String::from_utf8_lossy(&output.stderr));
                return Ok(printed);
            }
            // Git says most failures on its standard error, but some, such as "nothing to commit", on its output.
            let said = if output.stderr.iter().all(u8::is_ascii_whitespace) {
                &output.stdout
            } else {
                &output.stderr
            };
            Err(StepError::Failed(String::from_utf8_lossy(said).into_owned()))
        }
        _ => run_other(step, cancel).map(|()| String::new()), // The others print nothing the log keeps.
    }
}

/// Runs a step that is not git's.
fn run_other(step: &Step, cancel: &CancelToken) -> Result<(), StepError> {
    match step {
        Step::Write { path, contents } => write(path, contents).map_err(|error| StepError::failed(&error)),
        Step::Link { from, to } => setup::link(from, to).map_err(|error| StepError::failed(&error)),
        Step::Copy { from, to } => setup::copy(from, to).map_err(|error| StepError::failed(&error)),
        Step::Unlink { path } => unlink(path).map_err(StepError::Failed),
        Step::Delete { path } => delete(path).map_err(|error| StepError::failed(&error)),
        Step::MakeFolder { path } => fs::create_dir_all(path).map_err(|error| StepError::failed(&error)),
        Step::DeleteEmptyFolder { path } => delete_empty_folder(path).map_err(|error| StepError::failed(&error)),
        // A task run in place, as `gitardo-cli` runs it, is a command like any other.
        Step::Run { command, directory } | Step::Task { command, directory, .. } => {
            run_command(command, directory, cancel)
        }
        // A warning is written to the log before it "runs", and does nothing; a git step is run by `run_step`.
        Step::Warn { .. } | Step::Git { .. } => Ok(()),
    }
}

fn write(path: &Path, contents: &[u8]) -> std::io::Result<()> {
    if let Some(folder) = path.parent() {
        fs::create_dir_all(folder)?;
    }
    fs::write(path, contents)
}

/// Removes a link and never what it points at.
fn unlink(path: &Path) -> Result<(), String> {
    let metadata = fs::symlink_metadata(path).map_err(|error| error.to_string())?;
    if !metadata.is_symlink() {
        return Err(format!("{} is not a link.", path.display()));
    }
    // On Windows a link to a folder is removed as a folder, and a link to a file as a file.
    fs::remove_file(path)
        .or_else(|_| fs::remove_dir(path))
        .map_err(|error| error.to_string())
}

/// Deletes a file or a folder; what is not there is deleted already (`submodule.remove`, whose clone may be gone).
pub(crate) fn delete(path: &Path) -> std::io::Result<()> {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(error),
    };
    if metadata.is_dir() {
        fs::remove_dir_all(path)
    } else {
        fs::remove_file(path)
    }
}

/// How many times a folder that is empty is deleted when Windows holds it for a moment, and how long apart.
const EMPTY_FOLDER_TRIES: (usize, u64) = (20, 100);

/// Deletes a folder when it is empty, and leaves it when it has something in it or is not there
/// (`worktree.remove-empty-folders`). On Windows an empty folder held for a moment is tried again.
fn delete_empty_folder(path: &Path) -> std::io::Result<()> {
    let (tries, wait) = EMPTY_FOLDER_TRIES;
    for _try in 1..tries {
        let is_empty = match fs::read_dir(path) {
            Ok(mut entries) => entries.next().is_none(),
            // A folder that is not there, or is no folder, has nothing to delete.
            Err(_) => return Ok(()),
        };
        if !is_empty {
            return Ok(());
        }
        match fs::remove_dir(path) {
            Ok(()) => return Ok(()),
            Err(_) if cfg!(windows) => thread::sleep(Duration::from_millis(wait)),
            Err(error) => return Err(error),
        }
    }
    fs::remove_dir(path)
}

/// The shell of the system, given a command of the user's to run: `cmd /S /C "<command>"` on Windows, which runs the
/// command as it was written, its own quotes included, and `sh -c` elsewhere.
pub(crate) fn shell(command: &str) -> Command {
    #[cfg(windows)]
    let shell = {
        use std::os::windows::process::CommandExt;
        let mut shell = Command::new("cmd");
        // Given as it is: the quoting of an argument for a program would put a backslash before each quote of the
        // command, which `cmd` does not take away.
        let _ = shell.raw_arg(format!("/S /C \"{command}\""));
        shell
    };
    #[cfg(not(windows))]
    let shell = {
        let mut shell = Command::new("sh");
        let _ = shell.arg("-c").arg(command);
        shell
    };
    shell
}

/// Runs a command of the user's through the system's shell, and kills the shell when the plan is cancelled. What the
/// shell started itself may outlive it.
fn run_command(command: &str, directory: &Path, cancel: &CancelToken) -> Result<(), StepError> {
    let mut child = shell(command)
        .current_dir(directory)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| StepError::failed(&error))?;
    let stderr = child.stderr.take();
    // What the command says on its error output is read on a thread of its own, so that a full pipe never stops it.
    let said = thread::spawn(move || {
        let mut bytes = Vec::new();
        if let Some(mut pipe) = stderr {
            let _length = pipe.read_to_end(&mut bytes);
        }
        bytes
    });
    let mut pause = Duration::from_micros(200);
    let status = loop {
        if let Some(status) = child.try_wait().map_err(|error| StepError::failed(&error))? {
            break status;
        }
        if cancel.is_cancelled() {
            let _ = child.kill();
            let _ = child.wait();
            return Err(StepError::Cancelled);
        }
        thread::sleep(pause);
        pause = (pause * 2).min(Duration::from_millis(5));
    };
    if status.success() {
        return Ok(());
    }
    let said = said.join().unwrap_or_default();
    Err(StepError::Failed(String::from_utf8_lossy(&said).into_owned()))
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests {
    use std::path::PathBuf;

    use git::{CancelToken, Git};
    use model::contract::Step;

    use super::run_step;

    #[test]
    fn log_record__a_git_step_that_succeeded_gives_what_git_printed_on_both_outputs() {
        let repository = testkit::Repository::new();
        let folder = repository.path().to_path_buf();
        let git = Git::new(&folder);
        let cancel = CancelToken::new();
        // `git hash-object --stdin` prints the id of what it reads on its output.
        let hashed = Step::Git {
            arguments: ["hash-object", "--stdin"].into_iter().map(Into::into).collect(),
            directory: folder.clone(),
            input: Some(b"hello\n".to_vec()),
        };
        let printed = run_step(&git, &hashed, (&cancel, None)).ok().unwrap();
        assert_eq!(printed, "ce013625030ba8dba906f756967f9e9ca394464a\n");
        // `git checkout -b` says what it did on its error output, in the user's language.
        let branched = Step::git(folder.clone(), ["checkout", "-b", "topic"]);
        let printed = run_step(&git, &branched, (&cancel, None)).ok().unwrap();
        assert!(printed.contains("'topic'") && printed.ends_with('\n'), "{printed:?}");
        let written = Step::Write {
            path: PathBuf::from(&folder).join("a.txt"),
            contents: b"a".to_vec(),
        };
        assert_eq!(run_step(&git, &written, (&cancel, None)).ok().unwrap(), "");
    }
}
