//! The commands of a new worktree, handed over (`worktree-setup.command-steps`): the steps of a plan from its first
//! task on, run apart from the plan once its other steps ran, one after the other. Gitardo Desktop runs the tasks in
//! the terminals of its Tasks tab, and gives this runner the steps after them. A task this runner is given is a command
//! of the user's run through the shell of the system, whose output is told as it comes and whose input is what its
//! caller gives it; one that does not succeed, or is stopped, stops the steps after it.

use std::io::{Read, Write};
use std::path::Path;
use std::process::{Child, ChildStdin, Stdio};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::thread;
use std::time::{Duration, Instant};

use git::{CancelToken, Git};
use model::contract::Step;
use model::tasks::{TaskEnd, TaskFailure};

use crate::clock::Clock;
use crate::log::{CommandLog, LogEntry, LogLevel};
use crate::steps::{StepFailure, StepRunner, shell};

/// How often at most what a task prints is told, so that a task that prints much does not flood the window.
const TELL_EVERY: Duration = Duration::from_millis(50);
/// How long what a task printed is still read once it exited, from what it started and left running.
const LAST_PRINTED_FOR: Duration = Duration::from_millis(300);
/// The longest pause between two looks at a task running.
const LONGEST_PAUSE: Duration = Duration::from_millis(20);

/// How the engine runs the tasks of a plan.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TaskSteps {
    /// In the plan, as any step, with nothing told of their output: `gitardo-cli` and the tests of the engine.
    #[default]
    InPlace,
    /// Handed back with the steps after them, for a [`TaskRunner`] to run apart: the window, whose other operations do
    /// not wait for them.
    HandedOver,
}

/// The steps of a plan to run now, and those handed over from the first task on, when the tasks are handed over.
pub(crate) fn split(steps: &[Step], how: TaskSteps) -> (&[Step], &[Step]) {
    let first_task = steps.iter().position(|step| matches!(step, Step::Task { .. }));
    match (how, first_task) {
        (TaskSteps::HandedOver, Some(first)) => steps.split_at(first),
        _ => (steps, &[]),
    }
}

/// What a task tells as it runs, by its place among the tasks, from 0.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TaskEvent {
    /// It started.
    Started(usize),
    /// It printed this, on its output or its error output.
    Printed(usize, String),
}

/// Where the tasks tell what they do, and where what the user types for them comes from.
pub struct TaskTalk<'a> {
    /// The lines the user typed, each given to the task running with a line feed after it.
    pub input: &'a Receiver<String>,
    /// Told what the tasks do.
    pub tell: &'a mut dyn FnMut(TaskEvent),
}

/// Why the tasks did not all succeed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TasksFailure {
    /// A task did not succeed, or was stopped.
    Task(TaskFailure),
    /// A step after the tasks failed, such as opening the worktree in the external editor.
    Step(StepFailure),
}

/// Runs the steps handed over by a plan, with a log and a token of their own: stopping them cancels the token, which
/// kills the shell of the task running.
pub struct TaskRunner {
    git: Git,
    log: CommandLog,
    clock: Box<dyn Clock>,
    cancel: CancelToken,
}

impl TaskRunner {
    /// A runner that runs git as `git` says, logs what `level` says, tells the time from `clock`, and stops when
    /// `cancel` is cancelled.
    #[must_use]
    pub fn new(git: Git, level: LogLevel, clock: Box<dyn Clock>, cancel: CancelToken) -> Self {
        TaskRunner {
            git: git.with_cancel(cancel.clone()),
            log: CommandLog::new(level),
            clock,
            cancel,
        }
    }

    /// Runs the steps one after the other, each written to the log before it starts, telling what the tasks print as
    /// it comes and giving them what the user types.
    ///
    /// # Errors
    ///
    /// The task that did not succeed, or was stopped, with how many tasks after it did not run; or the step that
    /// failed after the tasks.
    pub fn run(&mut self, steps: &[Step], talk: &mut TaskTalk<'_>) -> Result<(), TasksFailure> {
        let count = steps.iter().filter(|step| matches!(step, Step::Task { .. })).count();
        let mut index = 0;
        for step in steps {
            let Step::Task { command, directory, .. } = step else {
                let mut runner = StepRunner {
                    git: &self.git,
                    log: &mut self.log,
                    clock: self.clock.as_ref(),
                    cancel: &self.cancel,
                    went_on_after: Vec::new(),
                };
                let _ran = runner.run(std::slice::from_ref(step)).map_err(TasksFailure::Step)?;
                continue;
            };
            let after = count - index - 1;
            let failure = |end| {
                TasksFailure::Task(TaskFailure {
                    command: command.clone(),
                    worktree: directory.clone(),
                    end,
                    after,
                })
            };
            if self.cancel.is_cancelled() {
                return Err(failure(TaskEnd::Stopped));
            }
            self.log.record(self.clock.now(), step);
            (talk.tell)(TaskEvent::Started(index));
            if let Err(end) = run_task((command, directory), index, &self.cancel, talk) {
                self.log.record_failure(self.clock.now(), &said_of(&end));
                return Err(failure(end));
            }
            index += 1;
        }
        Ok(())
    }

    /// Takes the lines the log recorded.
    pub fn take_log(&mut self) -> Vec<LogEntry> {
        self.log.take()
    }
}

/// What the log says under a task that did not succeed.
fn said_of(end: &TaskEnd) -> String {
    match end {
        TaskEnd::Code(code) => format!("It exited with {code}."),
        TaskEnd::Stopped => String::from("It was stopped."),
        TaskEnd::NotStarted(said) => said.clone(),
    }
}

/// Runs one command in a folder through the shell of the system, telling what it prints and giving it what is typed,
/// until it exits or is stopped.
fn run_task(
    (command, directory): (&str, &Path),
    index: usize,
    cancel: &CancelToken,
    talk: &mut TaskTalk<'_>,
) -> Result<(), TaskEnd> {
    let mut shell = shell(command);
    hide_console_window(&mut shell);
    let mut child = shell
        .current_dir(directory)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| TaskEnd::NotStarted(error.to_string()))?;
    let printed = read_printed(&mut child);
    let mut input = child.stdin.take();
    let mut told = Told::new(index);
    let mut pause = Duration::from_micros(200);
    loop {
        give_input(&mut input, talk.input);
        let status = child
            .try_wait()
            .map_err(|error| TaskEnd::NotStarted(error.to_string()))?;
        told.take(&printed, talk);
        if let Some(status) = status {
            told.take_last(&printed, talk);
            return match status.code() {
                Some(0) => Ok(()),
                Some(code) => Err(TaskEnd::Code(code)),
                None => Err(TaskEnd::Stopped),
            };
        }
        if cancel.is_cancelled() {
            // What the shell started itself may outlive it, and is not waited for.
            let _ = child.kill();
            let _ = child.wait();
            told.take_last(&printed, talk);
            return Err(TaskEnd::Stopped);
        }
        thread::sleep(pause);
        pause = (pause * 2).min(LONGEST_PAUSE);
    }
}

/// The output and the error output of a process, read on threads of their own so that a full pipe never stops it, in
/// the order they come.
fn read_printed(child: &mut Child) -> Receiver<Vec<u8>> {
    let (sender, printed) = mpsc::channel();
    if let Some(output) = child.stdout.take() {
        read_pipe(output, sender.clone());
    }
    if let Some(errors) = child.stderr.take() {
        read_pipe(errors, sender);
    }
    printed
}

/// Reads a pipe until it closes, sending each piece read.
fn read_pipe(mut pipe: impl Read + Send + 'static, sender: Sender<Vec<u8>>) {
    let reader = thread::Builder::new().name(String::from("task output")).spawn(move || {
        let mut buffer = [0_u8; 4096];
        loop {
            let length = match pipe.read(&mut buffer) {
                Ok(0) | Err(_) => return,
                Ok(length) => length,
            };
            let piece = buffer.get(..length).unwrap_or_default().to_vec();
            if sender.send(piece).is_err() {
                return;
            }
        }
    });
    // A reader that could not start leaves the task's output unread; the task still runs to its end.
    drop(reader);
}

/// Gives the task what the user typed, each line followed by a line feed. A task that closed its input reads nothing
/// more, which it decided itself.
fn give_input(input: &mut Option<ChildStdin>, typed: &Receiver<String>) {
    while let Ok(line) = typed.try_recv() {
        let Some(pipe) = input.as_mut() else {
            continue;
        };
        let written = pipe
            .write_all(line.as_bytes())
            .and_then(|()| pipe.write_all(b"\n"))
            .and_then(|()| pipe.flush());
        if written.is_err() {
            *input = None;
        }
    }
}

/// What a task printed and was not told yet, and when it was last told.
struct Told {
    index: usize,
    bytes: Vec<u8>,
    last: Option<Instant>,
}

impl Told {
    fn new(index: usize) -> Told {
        Told {
            index,
            bytes: Vec::new(),
            last: None,
        }
    }

    /// Takes what was printed since, and tells it unless it was told less than [`TELL_EVERY`] ago.
    fn take(&mut self, printed: &Receiver<Vec<u8>>, talk: &mut TaskTalk<'_>) {
        while let Ok(piece) = printed.try_recv() {
            self.bytes.extend(piece);
        }
        if self.last.is_some_and(|last| last.elapsed() < TELL_EVERY) {
            return;
        }
        self.tell(talk);
    }

    /// Takes what is left once the task exited, for a moment at most, and tells all of it.
    fn take_last(&mut self, printed: &Receiver<Vec<u8>>, talk: &mut TaskTalk<'_>) {
        let until = Instant::now() + LAST_PRINTED_FOR;
        loop {
            let left = until.saturating_duration_since(Instant::now());
            match printed.recv_timeout(left) {
                Ok(piece) => self.bytes.extend(piece),
                Err(RecvTimeoutError::Timeout | RecvTimeoutError::Disconnected) => break,
            }
        }
        self.tell(talk);
        if !self.bytes.is_empty() {
            let rest = String::from_utf8_lossy(&std::mem::take(&mut self.bytes)).into_owned();
            (talk.tell)(TaskEvent::Printed(self.index, rest));
        }
    }

    /// Tells the text of what was printed, keeping a character cut in two for the next piece.
    fn tell(&mut self, talk: &mut TaskTalk<'_>) {
        let text = take_text(&mut self.bytes);
        if !text.is_empty() {
            (talk.tell)(TaskEvent::Printed(self.index, text));
            self.last = Some(Instant::now());
        }
    }
}

/// The text of the bytes printed, taken from them; what is not UTF-8 is replaced, and a character whose last bytes have
/// not come yet is left for the next piece.
fn take_text(bytes: &mut Vec<u8>) -> String {
    let mut text = String::new();
    loop {
        match std::str::from_utf8(bytes) {
            Ok(rest) => {
                text.push_str(rest);
                bytes.clear();
                return text;
            }
            Err(error) => {
                let valid = error.valid_up_to();
                let cut = error.error_len().map_or(valid, |length| valid + length);
                text.push_str(&String::from_utf8_lossy(bytes.get(..cut).unwrap_or_default()));
                let _taken = bytes.drain(..cut.min(bytes.len()));
                if error.error_len().is_none() {
                    return text;
                }
            }
        }
    }
}

/// On Windows, a program started from a window gets a console window of its own unless told not to: what a task prints
/// is told to its caller.
#[cfg(windows)]
fn hide_console_window(command: &mut std::process::Command) {
    use std::os::windows::process::CommandExt;
    /// `CREATE_NO_WINDOW`, from the documentation of the process creation flags of Windows.
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    let _ = command.creation_flags(CREATE_NO_WINDOW);
}

/// Other systems open no window for a process.
#[cfg(not(windows))]
fn hide_console_window(_command: &mut std::process::Command) {}

#[cfg(test)]
mod tests {
    use super::take_text;

    #[test]
    fn keeps_a_character_cut_in_two_for_the_next_piece() {
        let mut bytes = "café".as_bytes().to_vec();
        let last = bytes.pop();
        assert_eq!(take_text(&mut bytes), "caf");
        bytes.extend(last);
        assert_eq!(take_text(&mut bytes), "é");
        let mut broken = vec![b'a', 0xff, b'b'];
        assert_eq!(take_text(&mut broken), "a\u{fffd}b");
    }
}
