//! The lines of the Command log as the bottom panel shows them (`log.show`): each with the time it started, in UTC as the
//! timeline says dates (open question Q58), and a read with how long it took; and the line it was opened at.

use engine::{CommandLog, LogKind};
use views::log::{LogAt, LogLine};

use super::state::State;

/// The lines kept, oldest first.
#[must_use]
pub fn log_lines(log: &CommandLog) -> Vec<LogLine> {
    log.entries()
        .map(|entry| {
            let text = match &entry.kind {
                LogKind::Read(took) => format!("{} ({} ms)", entry.line, took.as_millis()),
                LogKind::Step | LogKind::Failure | LogKind::Warning | LogKind::Input | LogKind::Output => {
                    entry.line.clone()
                }
            };
            LogLine {
                time: views::text::activity::time_of_day(entry.time.0),
                text,
                is_failure: entry.kind == LogKind::Failure,
                is_printed: matches!(entry.kind, LogKind::Input | LogKind::Output),
            }
        })
        .collect()
}

/// The line the Command log was opened at, among the lines kept, while it is still kept (`activity.said`).
#[must_use]
pub fn log_at(state: &State, kept: usize) -> Option<LogAt> {
    let (line, serial) = state.panel.log_at?;
    let line = line.checked_sub(state.log.dropped()).filter(|line| *line < kept)?;
    Some(LogAt { line, serial })
}
