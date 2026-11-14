//! When the repository is read (`repo.schedule-reads`): once after a burst of changes, never two reads at once, again
//! while something keeps changing during each read, never during an operation, at once when asked.
//!
//! Time is a value here: the moments are durations from any fixed instant the window chooses, such as when it opened.

use std::time::Duration;

use super::watch::{ReadCause, ReadRequest};

/// What the schedule knows: the read waiting for its moment, the read under way, the one noted to follow it.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ReadSchedule {
    /// The read a burst of changes asked for, waiting for its moment.
    due: Option<Due>,
    /// The read under way.
    running: Option<RunningRead>,
    /// The read asked for while one runs: one more follows it, however many were asked for.
    again: Option<ReadCause>,
    /// Whether something changed while the read under way ran.
    is_changed_during_read: bool,
    /// How many reads in a row the last one was; zero once a read ended with nothing changed during it.
    in_a_row: u32,
    /// The number of the next read.
    next_number: u64,
    /// The first read whose end stops the bar along the top of the list of commits.
    bar_until: Option<u64>,
    /// Whether the window is minimized: nothing of the repository is in sight.
    is_minimized: bool,
    /// Whether an operation runs: it is followed by a read of its own.
    is_operation_running: bool,
}

/// A read waiting for its moment.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Due {
    /// When it starts.
    at: Duration,
    /// Why: the last change of the burst.
    cause: ReadCause,
}

/// The read under way.
#[derive(Debug, Clone, PartialEq, Eq)]
struct RunningRead {
    /// Its number.
    number: u64,
    /// Why it runs.
    cause: ReadCause,
    /// Whether an operation started while it ran: what it read may be half from before and half from after.
    is_stale: bool,
}

/// What the schedule is told.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReadEvent {
    /// A change asked for a read, at a moment (`repo.watch-changes`).
    Changed {
        /// The read it asks for.
        request: ReadRequest,
        /// When it changed.
        at: Duration,
    },
    /// A read is asked for at once: **Refresh**, the repository chosen, the branches shown, a search, a filter, the
    /// next page.
    Asked(ReadCause),
    /// The moment a [`ReadAction::WakeAt`] asked for, or later.
    Tick {
        /// The moment.
        at: Duration,
    },
    /// A read ended, by its number.
    Finished(u64),
    /// The read under way was stopped by the window, which wants none of it, as when another repository is chosen.
    Stopped,
    /// The window stopped the read under way, if any, and wants one at once: the repository chosen, the history read
    /// again from its first page for other switches.
    Restarted(ReadCause),
    /// An operation started.
    OperationStarted,
    /// The operation ended.
    OperationEnded,
    /// The window was minimized, or shown again.
    Minimized(bool),
}

/// What the schedule asks of the window.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReadAction {
    /// Start a read.
    Start(StartedRead),
    /// Tell the schedule the moment it is, at this moment, with [`ReadEvent::Tick`].
    WakeAt(Duration),
    /// Show what the read of this number found.
    Show(u64),
    /// Throw away what the read of this number found, or stop it: it is read again.
    Discard(u64),
    /// Stop the bar along the top of the list of commits: what was waited for is there.
    StopBar,
}

/// A read to start.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StartedRead {
    /// Its number, which [`ReadEvent::Finished`] gives back.
    pub number: u64,
    /// Why it runs.
    pub cause: ReadCause,
    /// How many reads in a row it is: 2 and more when something changed during each read before it.
    pub in_a_row: u32,
}

impl ReadSchedule {
    /// Whether a read is under way.
    #[must_use]
    pub fn is_reading(&self) -> bool {
        self.running.is_some()
    }

    /// Whether the bar along the top of the list of commits runs.
    #[must_use]
    pub fn is_bar_running(&self) -> bool {
        self.bar_until.is_some()
    }

    /// The moment the read waiting starts, if one waits.
    #[must_use]
    pub fn due_at(&self) -> Option<Duration> {
        self.due.as_ref().map(|due| due.at)
    }

    /// Starts a read now, the next of the reads in a row or the first.
    fn start(&mut self, cause: ReadCause, actions: &mut Vec<ReadAction>) -> u64 {
        let number = self.next_number;
        self.next_number += 1;
        self.in_a_row += 1;
        self.due = None;
        self.is_changed_during_read = false;
        self.running = Some(RunningRead {
            number,
            cause: cause.clone(),
            is_stale: false,
        });
        actions.push(ReadAction::Start(StartedRead {
            number,
            cause,
            in_a_row: self.in_a_row,
        }));
        number
    }
}

/// The schedule after an event, and what the window is to do. A change waits for its delay from the first change of
/// its burst, brought forward by a change with a shorter delay; a read asked for while one runs is noted, and one more
/// follows; an operation stops the changes, throws away the read under way and is followed by a read of its own.
///
/// Feature: repo.schedule-reads
#[must_use]
pub fn schedule_reads(schedule: &ReadSchedule, event: &ReadEvent) -> (ReadSchedule, Vec<ReadAction>) {
    let mut next = schedule.clone();
    let mut actions = Vec::new();
    match event {
        ReadEvent::Changed { request, at } => changed(&mut next, request, *at, &mut actions),
        ReadEvent::Asked(cause) => asked(&mut next, cause.clone(), &mut actions),
        ReadEvent::Tick { at } => tick(&mut next, *at, &mut actions),
        ReadEvent::Finished(number) => finished(&mut next, *number, &mut actions),
        ReadEvent::Stopped => stopped(&mut next, &mut actions),
        ReadEvent::Restarted(cause) => {
            stopped(&mut next, &mut actions);
            next.in_a_row = 0;
            let number = next.start(cause.clone(), &mut actions);
            if cause.shows_bar() {
                next.bar_until = Some(number);
            }
        }
        ReadEvent::OperationStarted => {
            next.is_operation_running = true;
            next.due = None;
            next.again = None;
            if let Some(running) = next.running.as_mut() {
                running.is_stale = true;
            }
        }
        ReadEvent::OperationEnded => {
            next.is_operation_running = false;
            asked(&mut next, ReadCause::OperationEnded, &mut actions);
        }
        ReadEvent::Minimized(is_minimized) => {
            next.is_minimized = *is_minimized;
            if *is_minimized {
                next.due = None;
            }
        }
    }
    (next, actions)
}

/// The read under way stopped by the window: nothing follows it, and the bar waits for no read any more.
fn stopped(schedule: &mut ReadSchedule, actions: &mut Vec<ReadAction>) {
    if let Some(running) = schedule.running.take() {
        actions.push(ReadAction::Discard(running.number));
    }
    schedule.again = None;
    if schedule.bar_until.take().is_some() {
        actions.push(ReadAction::StopBar);
    }
}

/// A change: it waits for its delay, unless the window is minimized or an operation runs.
fn changed(schedule: &mut ReadSchedule, request: &ReadRequest, at: Duration, actions: &mut Vec<ReadAction>) {
    if schedule.is_minimized || schedule.is_operation_running {
        return;
    }
    if schedule.running.is_some() {
        schedule.is_changed_during_read = true;
    }
    let wanted = at + request.delay;
    let moment = schedule.due.as_ref().map_or(wanted, |due| due.at.min(wanted));
    schedule.due = Some(Due {
        at: moment,
        cause: request.cause.clone(),
    });
    actions.push(ReadAction::WakeAt(moment));
}

/// A read asked for at once: started now, or noted to follow the one under way.
fn asked(schedule: &mut ReadSchedule, cause: ReadCause, actions: &mut Vec<ReadAction>) {
    if schedule.is_operation_running {
        return;
    }
    if cause == ReadCause::RepositoryChosen
        && let Some(running) = schedule.running.take()
    {
        actions.push(ReadAction::Discard(running.number));
        schedule.again = None;
    }
    let shows_bar = cause.shows_bar();
    if let Some(running) = &schedule.running {
        if shows_bar && schedule.bar_until.is_none() {
            schedule.bar_until = Some(running.number + 1);
        }
        schedule.again = Some(cause);
    } else {
        schedule.in_a_row = 0;
        let number = schedule.start(cause, actions);
        if shows_bar && schedule.bar_until.is_none() {
            schedule.bar_until = Some(number);
        }
    }
}

/// The moment it is: the read waiting starts when its moment came, or is noted when one runs.
fn tick(schedule: &mut ReadSchedule, at: Duration, actions: &mut Vec<ReadAction>) {
    let Some(due) = schedule.due.clone() else {
        return;
    };
    if due.at > at {
        actions.push(ReadAction::WakeAt(due.at));
        return;
    }
    schedule.due = None;
    if schedule.is_minimized || schedule.is_operation_running {
        return;
    }
    if schedule.running.is_some() {
        schedule.again = Some(due.cause);
    } else {
        if !schedule.is_changed_during_read {
            schedule.in_a_row = 0;
        }
        let _number = schedule.start(due.cause, actions);
    }
}

/// A read ended: shown, or thrown away when an operation ran meanwhile; then the read noted meanwhile follows.
fn finished(schedule: &mut ReadSchedule, number: u64, actions: &mut Vec<ReadAction>) {
    let Some(running) = schedule.running.take_if(|running| running.number == number) else {
        return;
    };
    let follow = if running.is_stale {
        actions.push(ReadAction::Discard(number));
        Some(schedule.again.take().unwrap_or(running.cause))
    } else {
        actions.push(ReadAction::Show(number));
        if schedule.bar_until.is_some_and(|until| until <= number) {
            schedule.bar_until = None;
            actions.push(ReadAction::StopBar);
        }
        schedule.again.take()
    };
    let has_changed = schedule.is_changed_during_read;
    match follow {
        Some(cause) if !schedule.is_operation_running => {
            let _number = schedule.start(cause, actions);
        }
        _ if has_changed => {}
        _ => schedule.in_a_row = 0,
    }
}

#[cfg(test)]
mod tests;
