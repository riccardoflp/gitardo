//! Running git: reads that never take a lock and never ask, and the steps of plans, both cancellable.

use std::ffi::{OsStr, OsString};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, ExitStatus, Stdio};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;

use crate::error::GitError;

/// A way to stop a command that is no longer wanted: the git process it runs is killed.
#[derive(Debug, Clone, Default)]
pub struct CancelToken(Arc<AtomicBool>);

impl CancelToken {
    /// A token not cancelled yet.
    #[must_use]
    pub fn new() -> Self {
        CancelToken::default()
    }

    /// Cancels every command run with this token, and the ones run with it later.
    pub fn cancel(&self) {
        self.0.store(true, Ordering::SeqCst);
    }

    /// Whether it was cancelled.
    #[must_use]
    pub fn is_cancelled(&self) -> bool {
        self.0.load(Ordering::SeqCst)
    }
}

/// What a git command printed, and how it ended.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Output {
    /// Its exit code; none when it was ended by a signal.
    pub code: Option<i32>,
    /// What it printed on its standard output.
    pub stdout: Vec<u8>,
    /// What it printed on its standard error.
    pub stderr: Vec<u8>,
}

impl Output {
    /// Whether it ended with the exit code 0.
    #[must_use]
    pub fn is_success(&self) -> bool {
        self.code == Some(0)
    }
}

/// The git of the `PATH`, run in one folder.
#[derive(Debug, Clone)]
pub struct Git {
    executable: PathBuf,
    directory: PathBuf,
    cancel: CancelToken,
}

/// The configuration every command gets, so that what git prints can be read whatever the user's configuration says
/// (rule R9.3): paths as they are, in UTF-8, and no colors.
const FIXED_CONFIGURATION: [&str; 4] = ["-c", "core.quotepath=false", "-c", "color.ui=false"];

impl Git {
    /// The `git` of the `PATH`, run in a folder.
    #[must_use]
    pub fn new(directory: impl Into<PathBuf>) -> Self {
        Git {
            executable: PathBuf::from("git"),
            directory: directory.into(),
            cancel: CancelToken::new(),
        }
    }

    /// The same git, cancelled through a token.
    #[must_use]
    pub fn with_cancel(mut self, cancel: CancelToken) -> Self {
        self.cancel = cancel;
        self
    }

    /// The same git, run in another folder.
    #[must_use]
    pub fn in_directory(&self, directory: impl Into<PathBuf>) -> Self {
        Git {
            executable: self.executable.clone(),
            directory: directory.into(),
            cancel: self.cancel.clone(),
        }
    }

    /// The folder git runs in.
    #[must_use]
    pub fn directory(&self) -> &Path {
        &self.directory
    }

    /// Runs a read and returns what it printed; a read that fails is an error. A read never takes a lock another
    /// program waits for (`GIT_OPTIONAL_LOCKS=0`), never asks for anything, and runs in the C locale.
    ///
    /// # Errors
    ///
    /// When git cannot be started, fails, or is cancelled.
    pub fn run_read<Argument: AsRef<OsStr>>(&self, arguments: &[Argument]) -> Result<Vec<u8>, GitError> {
        let output = self.read_output(arguments)?;
        if output.is_success() {
            Ok(output.stdout)
        } else {
            Err(failure(arguments, &output))
        }
    }

    /// Runs a read whose exit code is part of the answer, such as `rev-parse --verify --quiet` or `diff --quiet`,
    /// and returns how it ended.
    ///
    /// # Errors
    ///
    /// When git cannot be started or is cancelled.
    pub fn read_output<Argument: AsRef<OsStr>>(&self, arguments: &[Argument]) -> Result<Output, GitError> {
        self.run(self.read_command(arguments), arguments, None)
    }

    /// Runs a read that reads what it is given on its standard input, such as `blame --contents -` with the text of a
    /// file not saved, and returns how it ended. It is a read like the others: no lock, no question, the C locale.
    ///
    /// # Errors
    ///
    /// When git cannot be started or is cancelled.
    pub fn read_output_with_input<Argument: AsRef<OsStr>>(
        &self,
        arguments: &[Argument],
        input: &[u8],
    ) -> Result<Output, GitError> {
        self.run(self.read_command(arguments), arguments, Some(input))
    }

    /// The command of a read: one that never takes a lock another program waits for, in the C locale.
    fn read_command<Argument: AsRef<OsStr>>(&self, arguments: &[Argument]) -> Command {
        let mut command = self.command(arguments);
        let _ = command
            .env("GIT_OPTIONAL_LOCKS", "0")
            .env("LC_ALL", "C")
            .env("LANGUAGE", "C");
        command
    }

    /// Runs a step of a plan, with what it reads on its standard input, and returns how it ended. Unlike a read, it
    /// takes the locks it needs and speaks the user's language.
    ///
    /// # Errors
    ///
    /// When git cannot be started or is cancelled. A step that fails is not an error here: its output says so.
    pub fn run_step<Argument: AsRef<OsStr>>(
        &self,
        arguments: &[Argument],
        input: Option<&[u8]>,
    ) -> Result<Output, GitError> {
        self.run(self.command(arguments), arguments, input)
    }

    fn command<Argument: AsRef<OsStr>>(&self, arguments: &[Argument]) -> Command {
        let mut command = Command::new(&self.executable);
        let _ = command
            .args(FIXED_CONFIGURATION)
            .args(arguments)
            .current_dir(&self.directory)
            // Gitardo has no terminal to ask in: git asks the credential helper, or fails.
            .env("GIT_TERMINAL_PROMPT", "0")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        hide_console_window(&mut command);
        command
    }

    fn run<Argument: AsRef<OsStr>>(
        &self,
        mut command: Command,
        arguments: &[Argument],
        input: Option<&[u8]>,
    ) -> Result<Output, GitError> {
        let line = command_line(arguments);
        let mut child = command.spawn().map_err(|source| match source.kind() {
            std::io::ErrorKind::NotFound => GitError::NotFound { source },
            _ => GitError::Process {
                command: line.clone(),
                source,
            },
        })?;
        let feeder = feed(child.stdin.take(), input.map(<[u8]>::to_vec));
        let stdout = drain(child.stdout.take());
        let stderr = drain(child.stderr.take());
        let status = self.wait(&mut child, &line)?;
        let gathered = |reader: thread::JoinHandle<std::io::Result<Vec<u8>>>| {
            reader
                .join()
                .unwrap_or_else(|_| Ok(Vec::new()))
                .map_err(|source| GitError::Process {
                    command: line.clone(),
                    source,
                })
        };
        let output = Output {
            code: status.code(),
            stdout: gathered(stdout)?,
            stderr: gathered(stderr)?,
        };
        // Git may end before it read all of its input, as `git commit` with nothing to commit does: what it printed
        // says what happened, so a failure to write the rest is not the error to report.
        let _ = feeder.join();
        Ok(output)
    }

    /// Waits for the process, and kills it when the token is cancelled.
    fn wait(&self, child: &mut Child, line: &str) -> Result<ExitStatus, GitError> {
        let mut pause = Duration::from_micros(200);
        loop {
            if let Some(status) = child.try_wait().map_err(|source| GitError::Process {
                command: line.to_owned(),
                source,
            })? {
                return Ok(status);
            }
            if self.cancel.is_cancelled() {
                // The process may have ended on its own in the meantime; either way it is no longer wanted.
                let _ = child.kill();
                let _ = child.wait();
                return Err(GitError::Cancelled {
                    command: line.to_owned(),
                });
            }
            thread::sleep(pause);
            pause = (pause * 2).min(Duration::from_millis(5));
        }
    }
}

/// Reads all a pipe gives, on a thread of its own, so that a full pipe never stops the process.
fn drain(pipe: Option<impl Read + Send + 'static>) -> thread::JoinHandle<std::io::Result<Vec<u8>>> {
    thread::spawn(move || {
        let mut bytes = Vec::new();
        if let Some(mut pipe) = pipe {
            let _length = pipe.read_to_end(&mut bytes)?;
        }
        Ok(bytes)
    })
}

/// Writes the input on a thread of its own, then closes the pipe so that git sees its end.
fn feed(pipe: Option<ChildStdin>, input: Option<Vec<u8>>) -> thread::JoinHandle<std::io::Result<()>> {
    thread::spawn(move || {
        if let (Some(mut pipe), Some(input)) = (pipe, input) {
            pipe.write_all(&input)?;
        }
        Ok(())
    })
}

/// The error of a read that failed.
fn failure<Argument: AsRef<OsStr>>(arguments: &[Argument], output: &Output) -> GitError {
    GitError::Failed {
        command: command_line(arguments),
        code: output.code,
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
    }
}

/// A command as the log writes it: `git` and its arguments, with the ones that hold spaces in quotes.
#[must_use]
pub fn command_line<Argument: AsRef<OsStr>>(arguments: &[Argument]) -> String {
    let arguments: Vec<OsString> = arguments.iter().map(|argument| argument.as_ref().to_owned()).collect();
    model::contract::Step::Git {
        arguments,
        directory: PathBuf::new(),
        input: None,
    }
    .log_line()
}

/// On Windows, a program started from a window gets a console window of its own unless told not to.
#[cfg(windows)]
fn hide_console_window(command: &mut Command) {
    use std::os::windows::process::CommandExt;
    /// `CREATE_NO_WINDOW`, from the documentation of the process creation flags of Windows.
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    let _ = command.creation_flags(CREATE_NO_WINDOW);
}

/// Other systems open no window for a process.
#[cfg(not(windows))]
fn hide_console_window(_command: &mut Command) {}
