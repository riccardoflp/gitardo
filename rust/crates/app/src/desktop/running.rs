//! The list of what runs and of what ran, kept from what the engine says (`activity.what-is-running`, D64). Each
//! activity is timed from the moment the engine started it, and ends at the moment the engine says it ended, whatever
//! the frames that looked at it: a row keeps its place, in the order things started, while it runs; the reads of the
//! same words one after the other are one row that counts them, among what runs while less than half a second passes
//! from one to the next, and among what ran while nothing else ended between them; and what ended goes among what ran,
//! the latest first, when it took half a second, with when it ended. The moment of each look is given, so that a test
//! replays what the engine said at the moments it chooses.

use std::collections::VecDeque;
use std::time::{Duration, Instant};

use engine::{ActivityEntry, ActivityReport};
use model::activity::counted;
use views::activity::{LISTED_AFTER, RanTask, RunningTask};

/// How many of the things that ran the Activity tab keeps, the latest.
const RAN_KEPT: usize = 50;
/// How long after a read ended another of the same words still joins its row among what runs, the row waiting that
/// long before it ends: as long as something runs before it is listed.
const IN_A_ROW_WITHIN: Duration = LISTED_AFTER;

/// A row of what runs: one activity, or the reads of the same words one after the other.
#[derive(Debug)]
struct Row {
    /// What it is, without how many it stands for.
    title: String,
    /// Why the latest of them runs.
    detail: Option<String>,
    /// Whether it stands for reads, which others of the same words join.
    is_read: bool,
    /// The moment the first of them started.
    since: Instant,
    /// When the first of them started, in seconds since 1970.
    started: i64,
    /// How many it stands for.
    count: u32,
    /// The numbers of those that still run.
    running: Vec<u64>,
    /// The moment the last of them ended, while none runs.
    idle_since: Option<Instant>,
}

impl Row {
    /// How long the row has run at a moment: since its first started, until now while one of them runs, until the
    /// last ended between one read and the next.
    fn elapsed(&self, now: Instant) -> Duration {
        self.idle_since.unwrap_or(now).saturating_duration_since(self.since)
    }

    /// The row as the views say it, at a moment.
    fn task(&self, now: Instant) -> RunningTask {
        RunningTask {
            title: counted(&self.title, self.count),
            detail: self.detail.clone(),
            elapsed: self.elapsed(now),
        }
    }
}

/// A row of what ran: one activity, or the reads of the same words that ended one after the other.
#[derive(Debug)]
struct Ran {
    /// What it was, without how many it stands for.
    title: String,
    /// Why the latest of them ran.
    detail: Option<String>,
    /// Whether it stands for reads, which the next reads of the same words to end join.
    is_read: bool,
    /// How many it stands for.
    count: u32,
    /// How long they took, together.
    took: Duration,
    /// The moment the last of them ended.
    ended: Instant,
    /// When the last of them ended, in seconds since 1970.
    ended_at: i64,
}

/// Something the engine said of an activity.
#[derive(Clone, Copy)]
enum Event<'a> {
    /// It started.
    Started(&'a ActivityEntry),
    /// The one of a number ended.
    Ended(u64),
}

/// The rows of what runs, and what ran.
#[derive(Debug, Default)]
pub struct RunningClock {
    /// What runs, in the order it started.
    rows: Vec<Row>,
    /// What ran, the latest first.
    ran: VecDeque<Ran>,
}

impl RunningClock {
    /// What runs at a moment, for the views, in the order it started, each with how long it has run, after what the
    /// engine says is taken in: what started, what ended since the last look, and the reads in a row that are over.
    pub fn tasks(&mut self, report: &ActivityReport, now: Instant) -> Vec<RunningTask> {
        for (at, event) in self.events(report) {
            match event {
                Event::Started(entry) => self.start(entry),
                Event::Ended(number) => self.end(number, at),
            }
        }
        self.end_idle(now);
        self.rows.iter().map(|row| row.task(now)).collect()
    }

    /// What ran, the latest first, each with when it ended and how long it took.
    #[must_use]
    pub fn ran(&self) -> Vec<RanTask> {
        self.ran
            .iter()
            .map(|ran| RanTask {
                task: RunningTask {
                    title: counted(&ran.title, ran.count),
                    detail: ran.detail.clone(),
                    elapsed: ran.took,
                },
                ended: ran.ended_at,
            })
            .collect()
    }

    /// What a report says that is new, in the order it happened: the start of each activity not seen yet, and the end
    /// of each that ended.
    fn events<'a>(&self, report: &'a ActivityReport) -> Vec<(Instant, Event<'a>)> {
        let is_known = |number: u64| self.rows.iter().any(|row| row.running.contains(&number));
        let mut events = Vec::new();
        for ended in &report.ended {
            if !is_known(ended.entry.number) {
                events.push((ended.entry.since, Event::Started(&ended.entry)));
            }
            events.push((ended.ended, Event::Ended(ended.entry.number)));
        }
        for entry in &report.running {
            if !is_known(entry.number) {
                events.push((entry.since, Event::Started(entry)));
            }
        }
        // The sort keeps the order of what happens at the same moment: an activity starts before it ends.
        events.sort_by_key(|(at, _)| *at);
        events
    }

    /// Takes in an activity that started: a read joins the row of the reads of the same words that runs or ended a
    /// moment before, anything else is a row of its own after the others.
    fn start(&mut self, entry: &ActivityEntry) {
        self.end_idle(entry.since);
        let title = entry.description.to_string();
        let detail = Some(entry.cause.to_string()).filter(|cause| !cause.is_empty());
        let same = self
            .rows
            .iter_mut()
            .find(|row| entry.is_read && row.is_read && row.title == title);
        if let Some(row) = same {
            row.count += 1;
            row.detail = detail;
            row.running.push(entry.number);
            row.idle_since = None;
            return;
        }
        self.rows.push(Row {
            title,
            detail,
            is_read: entry.is_read,
            since: entry.since,
            started: entry.started.0,
            count: 1,
            running: vec![entry.number],
            idle_since: None,
        });
    }

    /// Takes in the end of an activity at a moment: its row ends with it, unless it stands for reads, which wait for
    /// the next of them.
    fn end(&mut self, number: u64, at: Instant) {
        let Some(index) = self.rows.iter().position(|row| row.running.contains(&number)) else {
            return;
        };
        let Some(row) = self.rows.get_mut(index) else {
            return;
        };
        row.running.retain(|running| *running != number);
        if !row.running.is_empty() {
            return;
        }
        if row.is_read {
            row.idle_since = Some(at);
        } else {
            let row = self.rows.remove(index);
            self.keep(&row, at);
        }
    }

    /// Ends the rows of reads whose last read ended long enough before a moment that no read joins them any more.
    fn end_idle(&mut self, at: Instant) {
        let mut index = 0;
        while let Some(row) = self.rows.get(index) {
            match row.idle_since {
                Some(idle) if at.saturating_duration_since(idle) >= IN_A_ROW_WITHIN => {
                    let row = self.rows.remove(index);
                    self.keep(&row, idle);
                }
                _ => index += 1,
            }
        }
    }

    /// Keeps a row that ended at a moment among what ran, when it took long enough to have been listed: reads join the
    /// reads of the same words that ended last, anything else goes before what ended no later; the oldest go past the
    /// last 50.
    fn keep(&mut self, row: &Row, ended: Instant) {
        let took = ended.saturating_duration_since(row.since);
        if took < LISTED_AFTER {
            return;
        }
        let seconds = i64::try_from(took.as_secs()).unwrap_or(i64::MAX);
        let ended_at = row.started.saturating_add(seconds);
        let last = self
            .ran
            .front_mut()
            .filter(|last| row.is_read && last.is_read && last.title == row.title && last.ended <= ended);
        if let Some(last) = last {
            last.count += row.count;
            last.took += took;
            last.detail.clone_from(&row.detail);
            last.ended = ended;
            last.ended_at = ended_at;
            return;
        }
        let place = self
            .ran
            .iter()
            .position(|other| other.ended <= ended)
            .unwrap_or(self.ran.len());
        self.ran.insert(
            place,
            Ran {
                title: row.title.clone(),
                detail: row.detail.clone(),
                is_read: row.is_read,
                count: row.count,
                took,
                ended,
                ended_at,
            },
        );
        self.ran.truncate(RAN_KEPT);
    }
}
