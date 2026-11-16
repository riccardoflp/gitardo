//! The tabs not shown (`repo.watch-hidden`): a change in one reads nothing and marks it out of date, with the last
//! change; a tab never read is read when the window gives it its turn, one at a time, never beside an operation.
//!
//! The window decides whose turn it is, and stops the read of a tab not shown when the tab shown is read: this says what
//! each tab not shown is, and what one event does to it.

use super::schedule::{ReadAction, ReadEvent, StartedRead};
use super::watch::ReadCause;

/// What the window knows of a tab not shown.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct HiddenTab {
    /// How what it keeps stands against its repository.
    kept: Kept,
    /// The number of its next read.
    next_number: u64,
    /// Whether an operation runs: no tab not shown is read meanwhile.
    is_operation_running: bool,
}

/// How what a tab not shown keeps stands against its repository.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
enum Kept {
    /// It was never read: it waits for its turn.
    #[default]
    NotRead,
    /// It is read for the first time.
    Reading {
        /// The number of the read.
        number: u64,
        /// The last change seen while it ran, which it may have missed.
        changed: Option<ReadCause>,
    },
    /// What it keeps is the repository as it is.
    UpToDate,
    /// Something changed since it was last read: the last change.
    OutOfDate(ReadCause),
}

impl HiddenTab {
    /// A tab that was never read, which waits for its turn.
    #[must_use]
    pub fn new() -> HiddenTab {
        HiddenTab::default()
    }

    /// A tab just left: out of date when a read of it was under way or waiting, for that read's reason; up to date
    /// otherwise.
    #[must_use]
    pub fn left(pending: Option<ReadCause>) -> HiddenTab {
        let kept = pending.map_or(Kept::UpToDate, Kept::OutOfDate);
        HiddenTab {
            kept,
            ..HiddenTab::default()
        }
    }

    /// Whether it was never read and waits for its turn.
    #[must_use]
    pub fn is_waiting(&self) -> bool {
        self.kept == Kept::NotRead
    }

    /// Whether it is being read.
    #[must_use]
    pub fn is_reading(&self) -> bool {
        matches!(self.kept, Kept::Reading { .. })
    }

    /// The last change since it was read, when it is out of date: it is read again for it once shown.
    #[must_use]
    pub fn out_of_date(&self) -> Option<&ReadCause> {
        match &self.kept {
            Kept::OutOfDate(cause) => Some(cause),
            Kept::NotRead | Kept::Reading { .. } | Kept::UpToDate => None,
        }
    }

    /// The read under way stopped, thrown away: the tab waits for its turn again.
    fn stop(&mut self, actions: &mut Vec<ReadAction>) {
        if let Kept::Reading { number, .. } = self.kept {
            actions.push(ReadAction::Discard(number));
            self.kept = Kept::NotRead;
        }
    }

    /// Its turn: a tab never read is read, unless an operation runs.
    fn read(&mut self, cause: ReadCause, actions: &mut Vec<ReadAction>) {
        if self.is_operation_running || self.kept != Kept::NotRead {
            return;
        }
        let number = self.next_number;
        self.next_number += 1;
        self.kept = Kept::Reading { number, changed: None };
        actions.push(ReadAction::Start(StartedRead {
            number,
            cause,
            in_a_row: 1,
        }));
    }
}

/// A tab not shown after an event, and what the window is to do. A change reads nothing: it marks the tab out of date,
/// with the last change, or is noted when the tab is being read. The tab's turn starts the read of a tab never read; an
/// operation, or the window, stops it, and the tab waits for its turn again. A read that ended shows what it found, and
/// leaves the tab out of date when something changed while it ran.
///
/// Feature: repo.watch-hidden
#[must_use]
pub fn schedule_hidden(tab: &HiddenTab, event: &ReadEvent) -> (HiddenTab, Vec<ReadAction>) {
    let mut next = tab.clone();
    let mut actions = Vec::new();
    match event {
        ReadEvent::Changed { request, .. } => match &mut next.kept {
            Kept::NotRead => {}
            Kept::Reading { changed, .. } => *changed = Some(request.cause.clone()),
            kept @ (Kept::UpToDate | Kept::OutOfDate(_)) => *kept = Kept::OutOfDate(request.cause.clone()),
        },
        ReadEvent::Asked(cause) => next.read(cause.clone(), &mut actions),
        ReadEvent::Restarted(cause) => {
            next.stop(&mut actions);
            next.read(cause.clone(), &mut actions);
        }
        ReadEvent::Finished(finished) => {
            if let Kept::Reading { number, changed } = &next.kept
                && number == finished
            {
                actions.push(ReadAction::Show(*number));
                next.kept = changed.clone().map_or(Kept::UpToDate, Kept::OutOfDate);
            }
        }
        ReadEvent::Stopped => next.stop(&mut actions),
        ReadEvent::OperationStarted => {
            next.is_operation_running = true;
            next.stop(&mut actions);
        }
        ReadEvent::OperationEnded => next.is_operation_running = false,
        ReadEvent::Tick { .. } | ReadEvent::Minimized(_) => {}
    }
    (next, actions)
}

#[cfg(test)]
mod tests;
