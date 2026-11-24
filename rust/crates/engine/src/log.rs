//! The command log: every step of every plan, written before it starts with what it is given on its standard input,
//! then what git printed under a step that succeeded or what went wrong under a step that failed; and, at the debug
//! level, the reads with how long each took.

use std::collections::VecDeque;
use std::time::Duration;

use model::Timestamp;
use model::contract::Step;

/// How many lines the log keeps; older lines are dropped and counted (decision D12).
pub const MOST_LINES: usize = 10_000;

/// `log.level`: what the log records.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LogLevel {
    /// What changes something.
    #[default]
    Info,
    /// The reads too, with how long each took.
    Debug,
}

/// What a line of the log is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LogKind {
    /// A step of a plan, written as it runs.
    Step,
    /// What git or the system said when the step above failed.
    Failure,
    /// A line of what the step above was given on its standard input, indented as the preview shows it.
    Input,
    /// A line of what git printed under the step above, which succeeded: a hint of a push, what a pull did.
    Output,
    /// A warning a plan writes about the steps after it.
    Warning,
    /// A read, at the debug level, with how long it took.
    Read(Duration),
}

/// A line of the log.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogEntry {
    /// When it started.
    pub time: Timestamp,
    /// What it is.
    pub kind: LogKind,
    /// The words: `git checkout -b fix/login HEAD`.
    pub line: String,
}

/// The lines of the log of a window, oldest first.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CommandLog {
    entries: VecDeque<LogEntry>,
    dropped: usize,
    level: LogLevel,
}

impl CommandLog {
    /// An empty log that records what `level` says.
    #[must_use]
    pub fn new(level: LogLevel) -> Self {
        CommandLog {
            entries: VecDeque::new(),
            dropped: 0,
            level,
        }
    }

    /// Records from now on what another level says, as when `log.level` changed.
    pub fn set_level(&mut self, level: LogLevel) {
        self.level = level;
    }

    /// What the log records.
    #[must_use]
    pub fn level(&self) -> LogLevel {
        self.level
    }

    /// Writes a step, before it starts, and under it what it is given on its standard input, as the preview shows it.
    ///
    /// Feature: log.record
    pub fn record(&mut self, time: Timestamp, step: &Step) {
        let kind = match step {
            Step::Warn { .. } => LogKind::Warning,
            _ => LogKind::Step, // Every other step changes something, and is written as it runs.
        };
        self.push(LogEntry {
            time,
            kind,
            line: step.log_line(),
        });
        for line in step.input_lines() {
            self.push(LogEntry {
                time,
                kind: LogKind::Input,
                line,
            });
        }
    }

    /// Writes what git printed under the step that succeeded, a line each, as a terminal would leave it: of a line
    /// that git wrote again and again, such as its progress, the last of it; lines with nothing on them are left out.
    pub fn record_output(&mut self, time: Timestamp, printed: &str) {
        for line in printed_lines(printed) {
            self.push(LogEntry {
                time,
                kind: LogKind::Output,
                line,
            });
        }
    }

    /// Writes what git or the system said, under the step that failed.
    pub fn record_failure(&mut self, time: Timestamp, said: &str) {
        let said = said.trim_end();
        let line = if said.is_empty() {
            String::from("(nothing was said)")
        } else {
            said.to_owned()
        };
        self.push(LogEntry {
            time,
            kind: LogKind::Failure,
            line,
        });
    }

    /// Makes what git said under the newest step that failed a warning: the plan got past that step, as a cherry-pick
    /// that changes nothing is skipped, and nothing failed in the end.
    pub fn got_past_failure(&mut self) {
        if let Some(entry) = self
            .entries
            .iter_mut()
            .rev()
            .find(|entry| entry.kind == LogKind::Failure)
        {
            entry.kind = LogKind::Warning;
        }
    }

    /// Writes a warning, whatever the level: what went wrong beside a plan that still ran, such as a token that could
    /// not be kept.
    pub fn record_warning(&mut self, time: Timestamp, line: String) {
        self.push(LogEntry {
            time,
            kind: LogKind::Warning,
            line,
        });
    }

    /// Writes a read, when the level is debug.
    pub fn record_read(&mut self, time: Timestamp, line: String, took: Duration) {
        if self.level == LogLevel::Debug {
            self.push(LogEntry {
                time,
                kind: LogKind::Read(took),
                line,
            });
        }
    }

    /// Writes lines another log recorded, as those of an operation that ran on a thread of its own: a read only when
    /// the level is debug.
    pub fn append(&mut self, entries: impl IntoIterator<Item = LogEntry>) {
        for entry in entries {
            if self.level == LogLevel::Debug || !matches!(entry.kind, LogKind::Read(_)) {
                self.push(entry);
            }
        }
    }

    /// Takes the lines kept, oldest first, and leaves the log empty.
    pub fn take(&mut self) -> Vec<LogEntry> {
        self.dropped = 0;
        self.entries.drain(..).collect()
    }

    /// The lines kept, oldest first.
    pub fn entries(&self) -> impl Iterator<Item = &LogEntry> {
        self.entries.iter()
    }

    /// How many lines were dropped to keep at most [`MOST_LINES`].
    #[must_use]
    pub fn dropped(&self) -> usize {
        self.dropped
    }

    fn push(&mut self, entry: LogEntry) {
        if self.entries.len() == MOST_LINES {
            let _oldest = self.entries.pop_front();
            self.dropped += 1;
        }
        self.entries.push_back(entry);
    }
}

/// The lines git printed as a terminal would leave them: what follows the last carriage return of each line, without
/// the spaces at its end, and none that is empty.
fn printed_lines(printed: &str) -> Vec<String> {
    printed
        .split('\n')
        .filter_map(|line| {
            let shown = line.trim_end().rsplit('\r').next().unwrap_or_default().trim_end();
            (!shown.is_empty()).then(|| shown.to_owned())
        })
        .collect()
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests {
    use super::{CommandLog, LogKind, LogLevel, MOST_LINES};
    use model::Timestamp;
    use model::contract::Step;
    use std::path::PathBuf;
    use std::time::Duration;

    #[test]
    fn writes_steps_and_failures_and_reads_only_at_debug() {
        let step = Step::git(PathBuf::from("."), ["commit", "-m", "Fix the parser"]);
        let mut info = CommandLog::new(LogLevel::Info);
        info.record(Timestamp(5), &step);
        info.record_failure(Timestamp(6), "fatal: nothing to commit\n");
        info.record_read(Timestamp(7), String::from("git status"), Duration::from_millis(3));
        let lines: Vec<(&LogKind, &str)> = info.entries().map(|entry| (&entry.kind, entry.line.as_str())).collect();
        assert_eq!(
            lines,
            vec![
                (&LogKind::Step, "git commit -m \"Fix the parser\""),
                (&LogKind::Failure, "fatal: nothing to commit")
            ]
        );
        let mut debug = CommandLog::new(LogLevel::Debug);
        debug.record_read(Timestamp(7), String::from("git status"), Duration::from_millis(3));
        assert_eq!(debug.entries().count(), 1);
    }

    #[test]
    fn log_record__a_failure_the_plan_got_past_is_written_as_a_warning() {
        let mut log = CommandLog::new(LogLevel::Info);
        log.record(
            Timestamp(1),
            &Step::git(PathBuf::from("."), ["cherry-pick", "--continue"]),
        );
        log.record_failure(Timestamp(2), "The previous cherry-pick is now empty");
        log.record(Timestamp(3), &Step::git(PathBuf::from("."), ["cherry-pick", "--skip"]));
        log.got_past_failure();
        let kinds: Vec<&LogKind> = log.entries().map(|entry| &entry.kind).collect();
        assert_eq!(kinds, vec![&LogKind::Step, &LogKind::Warning, &LogKind::Step]);
    }

    #[test]
    fn log_record__a_step_has_its_standard_input_under_it_as_the_preview_shows_it() {
        let step = Step::Git {
            arguments: ["apply", "--cached", "-"].into_iter().map(Into::into).collect(),
            directory: PathBuf::from("."),
            input: Some(b"--- a/a.rs\n+++ b/a.rs\n".to_vec()),
        };
        let mut log = CommandLog::new(LogLevel::Info);
        log.record(Timestamp(1), &step);
        let lines: Vec<(&LogKind, &str)> = log.entries().map(|entry| (&entry.kind, entry.line.as_str())).collect();
        assert_eq!(
            lines,
            vec![
                (&LogKind::Step, "git apply --cached -"),
                (&LogKind::Input, "  --- a/a.rs"),
                (&LogKind::Input, "  +++ b/a.rs"),
            ]
        );
    }

    #[test]
    fn log_record__what_git_printed_is_written_a_line_each_as_a_terminal_leaves_it() {
        let mut log = CommandLog::new(LogLevel::Info);
        log.record_output(
            Timestamp(1),
            "To /remote.git\r\n   1a2b3c4..5d6e7f8  main -> main\n\nWriting objects:  50% (1/2)\rWriting objects: 100% (2/2), done.\n",
        );
        let lines: Vec<(&LogKind, &str)> = log.entries().map(|entry| (&entry.kind, entry.line.as_str())).collect();
        assert_eq!(
            lines,
            vec![
                (&LogKind::Output, "To /remote.git"),
                (&LogKind::Output, "   1a2b3c4..5d6e7f8  main -> main"),
                (&LogKind::Output, "Writing objects: 100% (2/2), done."),
            ]
        );
    }

    #[test]
    fn keeps_the_last_lines_and_counts_the_others() {
        let mut log = CommandLog::new(LogLevel::Info);
        for index in 0..MOST_LINES + 3 {
            log.record(
                Timestamp(0),
                &Step::Delete {
                    path: PathBuf::from(index.to_string()),
                },
            );
        }
        assert_eq!(log.entries().count(), MOST_LINES);
        assert_eq!(log.dropped(), 3);
        assert_eq!(log.entries().next().map(|entry| entry.line.as_str()), Some("delete 3"));
    }
}
