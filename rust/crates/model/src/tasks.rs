//! The tasks of the Activity drawer, the commands a new worktree runs (`worktree-setup.command-steps`): how each
//! stands, what it printed as the drawer shows it, and what Gitardo says when one did not succeed.
//!
//! And the tasks of a repository (`docs/features/19-tasks.md`): those its worktree defines, the order they run in,
//! and when one that runs in the background is ready.

use std::path::{Path, PathBuf};

mod defined;
mod files;
mod order;
mod ready;
mod run;
#[cfg(test)]
mod testing;

pub use defined::{
    Background, DependsOrder, NotRunnable, PackageManager, PackageScripts, Quoting, Task, TaskCategory,
    TaskFileProblem, TaskKind, TaskReference, TaskShell, TaskSource, Tasks, Word,
};
pub use files::{
    LOCK_FILES, MODULES_FOLDER, PACKAGE_FILE, PackageFile, TASKS_FILE, TaskFiles, TaskSystem, TasksContext,
    package_manager,
};
pub use order::{OrderRefusal, RunOrder, RunStep, run_order};
pub use ready::{QUIET, Readiness};
pub use run::{Blocked, Run, StepState};

/// The most lines the drawer keeps of what a task printed; older ones are dropped.
const KEPT_LINES: usize = 5_000;

/// How a task ended when it did not succeed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TaskEnd {
    /// It exited with this code, other than 0.
    Code(i32),
    /// It was stopped, or ended with no exit code.
    Stopped,
    /// The shell of the system could not be started: what the system said.
    NotStarted(String),
}

/// How a task stands.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TaskStatus {
    /// It waits for the commands before it.
    Waiting,
    /// It runs.
    Running,
    /// It exited with 0.
    Succeeded,
    /// It did not succeed.
    Failed(TaskEnd),
    /// A command before it did not succeed, so it did not run.
    DidNotRun,
}

/// A command that did not succeed, with what it leaves undone.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskFailure {
    /// The command.
    pub command: String,
    /// The worktree it ran in.
    pub worktree: PathBuf,
    /// How it ended.
    pub end: TaskEnd,
    /// How many commands after it did not run.
    pub after: usize,
}

/// What Gitardo says when a command of a new worktree did not succeed: **`npm ci` failed with exit code 1 in
/// `<worktree>`, so the 2 commands after it did not run.**, or **did not finish** for one stopped.
#[must_use]
pub fn failure_said(failure: &TaskFailure) -> String {
    let command = &failure.command;
    let worktree = shown(&failure.worktree);
    let what = match &failure.end {
        TaskEnd::Code(code) => format!("`{command}` failed with exit code {code} in `{worktree}`"),
        TaskEnd::Stopped => format!("`{command}` did not finish in `{worktree}`"),
        TaskEnd::NotStarted(said) => format!("`{command}` could not be started in `{worktree}`: {}", said.trim()),
    };
    match failure.after {
        0 => format!("{what}."),
        1 => format!("{what}, so the command after it did not run."),
        after => format!("{what}, so the {after} commands after it did not run."),
    }
}

/// A folder as the messages show it.
fn shown(folder: &Path) -> String {
    folder.display().to_string()
}

/// What a task printed, as the drawer shows it: in lines, without the colors and the moves of the cursor a terminal
/// would take, a line written again from its start after a carriage return, as progress bars do, and the last 5,000
/// lines only.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TaskOutput {
    /// The lines, the last one being written; empty before anything was printed.
    lines: Vec<String>,
    /// How many older lines were dropped.
    dropped: usize,
    /// Where it is in what a terminal would read as an escape sequence.
    escape: Escape,
    /// Whether the last character was a carriage return, which the next one writes over, unless it ends the line.
    is_returned: bool,
}

/// Where the output is in an escape sequence of a terminal, which can come in two pieces.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum Escape {
    /// In plain text.
    #[default]
    None,
    /// Just after `ESC`.
    Started,
    /// In a control sequence, `ESC [`, until its final character.
    Control,
    /// In an operating system command, `ESC ]`, until `BEL` or `ESC \`.
    Command,
    /// After `ESC` inside an operating system command.
    CommandEnding,
}

impl TaskOutput {
    /// Adds what the task printed next.
    pub fn push(&mut self, text: &str) {
        for character in text.chars() {
            self.push_character(character);
        }
        // The empty line being written at the end is not one of those kept.
        let shown = self.lines().len();
        if shown > KEPT_LINES {
            let extra = shown - KEPT_LINES;
            let _dropped = self.lines.drain(..extra);
            self.dropped += extra;
        }
    }

    /// The lines to show, oldest first, without an empty line being written at the end.
    #[must_use]
    pub fn lines(&self) -> &[String] {
        match self.lines.split_last() {
            Some((last, before)) if last.is_empty() => before,
            _ => &self.lines,
        }
    }

    /// How many older lines were dropped to keep the last 5,000.
    #[must_use]
    pub fn dropped(&self) -> usize {
        self.dropped
    }

    fn push_character(&mut self, character: char) {
        self.escape = match self.escape {
            Escape::None if character == '\u{1b}' => Escape::Started,
            Escape::None => {
                self.push_text(character);
                Escape::None
            }
            Escape::Started => match character {
                '[' => Escape::Control,
                ']' => Escape::Command,
                // Any other sequence of two characters, such as `ESC 7`, ends with its second one.
                _ => Escape::None,
            },
            Escape::Control if ('\u{40}'..='\u{7e}').contains(&character) => Escape::None,
            Escape::Control => Escape::Control,
            Escape::Command => match character {
                '\u{7}' => Escape::None,
                '\u{1b}' => Escape::CommandEnding,
                _ => Escape::Command,
            },
            Escape::CommandEnding if character == '\\' => Escape::None,
            Escape::CommandEnding => Escape::Command,
        };
    }

    fn push_text(&mut self, character: char) {
        if character == '\r' {
            self.is_returned = true;
            return;
        }
        let is_returned = std::mem::take(&mut self.is_returned);
        if self.lines.is_empty() {
            self.lines.push(String::new());
        }
        if character == '\n' {
            self.lines.push(String::new());
            return;
        }
        let Some(line) = self.lines.last_mut() else {
            return;
        };
        if is_returned {
            line.clear();
        }
        match character {
            '\t' => line.push_str("    "),
            // The other control characters, such as a bell, show nothing.
            control if control.is_control() => {}
            shown => line.push(shown),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::{TaskEnd, TaskFailure, TaskOutput, failure_said};

    fn output(pieces: &[&str]) -> Vec<String> {
        let mut output = TaskOutput::default();
        for piece in pieces {
            output.push(piece);
        }
        output.lines().to_vec()
    }

    #[test]
    fn splits_what_is_printed_in_lines_across_pieces() {
        assert_eq!(output(&["added 12 pack", "ages\r\nok\n"]), ["added 12 packages", "ok"]);
        assert_eq!(output(&["Name? "]), ["Name? "]);
        assert_eq!(output(&[]), Vec::<String>::new());
    }

    #[test]
    fn writes_a_line_again_after_a_carriage_return() {
        assert_eq!(output(&["10 %\r50 %\r", "100 %\ndone"]), ["100 %", "done"]);
    }

    #[test]
    fn drops_the_colors_and_the_titles_of_a_terminal_even_cut_in_two() {
        assert_eq!(output(&["\u{1b}[1;3", "2mred\u{1b}[0m plain"]), ["red plain"]);
        assert_eq!(output(&["\u{1b}]0;npm\u{7}title gone"]), ["title gone"]);
        assert_eq!(output(&["\u{1b}]0;npm\u{1b}\\after"]), ["after"]);
    }

    #[test]
    fn keeps_the_last_lines_and_counts_those_dropped() {
        let mut output = TaskOutput::default();
        for line in 0..5_010 {
            output.push(&format!("{line}\n"));
        }
        assert_eq!(output.lines().len(), 5_000);
        assert_eq!(output.dropped(), 10);
        assert_eq!(output.lines().first().map(String::as_str), Some("10"));
    }

    #[test]
    fn says_which_command_did_not_succeed_and_how_many_did_not_run_after_it() {
        let failure = |end, after| TaskFailure {
            command: String::from("npm ci"),
            worktree: PathBuf::from("trees/fix"),
            end,
            after,
        };
        let tree = PathBuf::from("trees/fix").display().to_string();
        assert_eq!(
            failure_said(&failure(TaskEnd::Code(1), 2)),
            format!("`npm ci` failed with exit code 1 in `{tree}`, so the 2 commands after it did not run.")
        );
        assert_eq!(
            failure_said(&failure(TaskEnd::Stopped, 1)),
            format!("`npm ci` did not finish in `{tree}`, so the command after it did not run.")
        );
        assert_eq!(
            failure_said(&failure(TaskEnd::Code(3), 0)),
            format!("`npm ci` failed with exit code 3 in `{tree}`.")
        );
    }
}
