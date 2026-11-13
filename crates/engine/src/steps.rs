//! Running the steps of a plan, one by one, each written to the log before it starts. A step that fails stops the
//! plan (rule R4.5), and so does cancelling it: the process of the step running is killed, and no step after it starts.

use std::fmt;
use std::fs;
use std::io::Read;
use std::path::Path;
use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;

use git::{CancelToken, Git, GitError};
use model::contract::Step;

use crate::clock::Clock;
use crate::log::CommandLog;

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
    /// start because the plan was cancelled before it is not logged.
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
                });
            }
            self.log.record(self.clock.now(), step);
            let (said, cancelled) = match run_step(self.git, step, self.cancel) {
                Ok(()) => {
                    ran.push(step.log_line());
                    continue;
                }
                Err(StepError::Failed(said)) => (said, false),
                Err(StepError::Cancelled) => (String::from("Cancelled."), true),
            };
            self.log.record_failure(self.clock.now(), &said);
            return Err(StepFailure {
                ran,
                failed: step.log_line(),
                said,
                cancelled,
                plan: String::new(),
            });
        }
        Ok(ran)
    }
}

/// Runs one step; what went wrong, in the words of git or of the system, when it fails.
fn run_step(git: &Git, step: &Step, cancel: &CancelToken) -> Result<(), StepError> {
    match step {
        Step::Git {
            arguments,
            directory,
            input,
        } => {
            let output = match git.in_directory(directory).run_step(arguments, input.as_deref()) {
                Ok(output) => output,
                Err(GitError::Cancelled { .. }) => return Err(StepError::Cancelled),
                Err(error) => return Err(StepError::failed(&error)),
            };
            if output.is_success() {
                return Ok(());
            }
            // Git says most failures on its standard error, but some, such as "nothing to commit", on its output.
            let said = if output.stderr.iter().all(u8::is_ascii_whitespace) {
                &output.stdout
            } else {
                &output.stderr
            };
            Err(StepError::Failed(String::from_utf8_lossy(said).into_owned()))
        }
        Step::Write { path, contents } => write(path, contents).map_err(|error| StepError::failed(&error)),
        Step::Link { from, to } => link(from, to).map_err(|error| StepError::failed(&error)),
        Step::Copy { from, to } => copy(from, to).map_err(|error| StepError::failed(&error)),
        Step::Unlink { path } => unlink(path).map_err(StepError::Failed),
        Step::Delete { path } => delete(path).map_err(|error| StepError::failed(&error)),
        Step::Run { command, directory } => run_command(command, directory, cancel),
        // A warning is written to the log before it "runs", and does nothing.
        Step::Warn { .. } => Ok(()),
    }
}

fn write(path: &Path, contents: &[u8]) -> std::io::Result<()> {
    if let Some(folder) = path.parent() {
        fs::create_dir_all(folder)?;
    }
    fs::write(path, contents)
}

/// Makes a link to a folder. Linking with a junction on Windows, which needs no privilege, comes with the worktree
/// setup and the platform layer (milestone M8).
fn link(from: &Path, to: &Path) -> std::io::Result<()> {
    #[cfg(windows)]
    return std::os::windows::fs::symlink_dir(to, from);
    #[cfg(not(windows))]
    return std::os::unix::fs::symlink(to, from);
}

fn copy(from: &Path, to: &Path) -> std::io::Result<()> {
    if !fs::symlink_metadata(from)?.is_dir() {
        if let Some(folder) = to.parent() {
            fs::create_dir_all(folder)?;
        }
        return fs::copy(from, to).map(|_bytes| ());
    }
    fs::create_dir_all(to)?;
    for entry in fs::read_dir(from)? {
        let entry = entry?;
        copy(&entry.path(), &to.join(entry.file_name()))?;
    }
    Ok(())
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

fn delete(path: &Path) -> std::io::Result<()> {
    if fs::symlink_metadata(path)?.is_dir() {
        fs::remove_dir_all(path)
    } else {
        fs::remove_file(path)
    }
}

/// Runs a command of the user's through the system's shell, and kills the shell when the plan is cancelled. What the
/// shell started itself may outlive it.
fn run_command(command: &str, directory: &Path, cancel: &CancelToken) -> Result<(), StepError> {
    #[cfg(windows)]
    let mut shell = {
        let mut shell = Command::new("cmd");
        let _ = shell.arg("/C").arg(command);
        shell
    };
    #[cfg(not(windows))]
    let mut shell = {
        let mut shell = Command::new("sh");
        let _ = shell.arg("-c").arg(command);
        shell
    };
    let mut child = shell
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
