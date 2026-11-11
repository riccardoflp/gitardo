//! Running the steps of a plan, one by one, each written to the log before it starts. A step that fails stops the
//! plan (rule R4.5).

use std::fmt;
use std::fs;
use std::path::Path;
use std::process::Command;

use git::Git;
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
    /// What git or the system said.
    pub said: String,
}

impl fmt::Display for StepFailure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "`{}` failed: {}", self.failed, self.said.trim())
    }
}

/// Where steps run and what they write to.
pub(crate) struct StepRunner<'a> {
    pub(crate) git: &'a Git,
    pub(crate) log: &'a mut CommandLog,
    pub(crate) clock: &'a dyn Clock,
}

impl StepRunner<'_> {
    /// Runs the steps in order, and stops at the first that fails.
    pub(crate) fn run(&mut self, steps: &[Step]) -> Result<Vec<String>, StepFailure> {
        let mut ran = Vec::new();
        for step in steps {
            self.log.record(self.clock.now(), step);
            if let Err(said) = run_step(self.git, step) {
                self.log.record_failure(self.clock.now(), &said);
                return Err(StepFailure {
                    ran,
                    failed: step.log_line(),
                    said,
                });
            }
            ran.push(step.log_line());
        }
        Ok(ran)
    }
}

/// Runs one step; what went wrong, in the words of git or of the system, when it fails.
fn run_step(git: &Git, step: &Step) -> Result<(), String> {
    match step {
        Step::Git {
            arguments,
            directory,
            input,
        } => {
            let output = git
                .in_directory(directory)
                .run_step(arguments, input.as_deref())
                .map_err(|error| error.to_string())?;
            if output.is_success() {
                return Ok(());
            }
            // Git says most failures on its standard error, but some, such as "nothing to commit", on its output.
            let said = if output.stderr.iter().all(u8::is_ascii_whitespace) {
                &output.stdout
            } else {
                &output.stderr
            };
            Err(String::from_utf8_lossy(said).into_owned())
        }
        Step::Write { path, contents } => write(path, contents).map_err(|error| error.to_string()),
        Step::Link { from, to } => link(from, to).map_err(|error| error.to_string()),
        Step::Copy { from, to } => copy(from, to).map_err(|error| error.to_string()),
        Step::Unlink { path } => unlink(path),
        Step::Delete { path } => delete(path).map_err(|error| error.to_string()),
        Step::Run { command, directory } => run_command(command, directory),
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

/// Runs a command of the user's through the system's shell.
fn run_command(command: &str, directory: &Path) -> Result<(), String> {
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
    let output = shell
        .current_dir(directory)
        .output()
        .map_err(|error| error.to_string())?;
    if output.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).into_owned())
    }
}
