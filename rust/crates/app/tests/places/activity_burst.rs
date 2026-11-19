//! A burst of changes as the engine says it, and what the list of what runs must not do while it is read (M18, D64):
//! forty reads of a repository in ten seconds, each over in a fifth of a second, a fetch running through them, and two
//! reads of other repositories that end within the same frame; the moments the window draws a frame; the list of each
//! frame, written as a line; and the faults of a list: a row that moves, one that goes, one listed for what ran a
//! moment, the same title twice, a time that is not the time since the engine started it.

use std::fmt::Write as _;
use std::time::{Duration, Instant};

use engine::{ActivityEntry, ActivityReport, EndedActivity};
use model::Timestamp;
use model::contract::Text;

use super::support::NOW;

/// The title of the fetch.
pub const FETCH: &str = "Fetching origin";
/// The title of the forty reads.
pub const READ: &str = "Reading app";
/// How long something runs before it is listed.
pub const LISTED_AFTER: Duration = views::activity::LISTED_AFTER;

/// Some milliseconds.
pub const fn ms(count: u64) -> Duration {
    Duration::from_millis(count)
}

/// Something the engine runs during the burst.
#[derive(Clone, Debug)]
pub struct Scripted {
    /// Its number, in the order things start.
    pub number: u64,
    /// What it is.
    pub title: String,
    /// What asked for it, said by nothing else of the burst.
    pub cause: String,
    /// When the engine starts it, from the start of the burst.
    pub starts: Duration,
    /// When it ends.
    pub ends: Duration,
    /// Whether it is a read.
    pub is_read: bool,
}

impl Scripted {
    /// Something that is not a read, with its number, from a moment to another in milliseconds.
    pub fn operation(number: u64, title: &str, (starts, ends): (u64, u64)) -> Scripted {
        Scripted {
            number,
            title: title.to_owned(),
            cause: String::new(),
            starts: ms(starts),
            ends: ms(ends),
            is_read: false,
        }
    }

    /// A read, with its number, from a moment to another in milliseconds.
    pub fn read(number: u64, title: &str, (starts, ends): (u64, u64)) -> Scripted {
        Scripted {
            is_read: true,
            ..Scripted::operation(number, title, (starts, ends))
        }
    }

    /// The activity as the engine says it, in a burst that started at a moment.
    fn entry(&self, start: Instant) -> ActivityEntry {
        ActivityEntry {
            number: self.number,
            description: Text::from(self.title.clone()),
            cause: Text::from(self.cause.clone()),
            started: Timestamp(NOW + i64::try_from(self.starts.as_secs()).unwrap()),
            since: start + self.starts,
            is_read: self.is_read,
            progress: None,
        }
    }
}

/// What the engine says at a moment of a burst that started at an instant: what runs, in the order it started, and
/// what ended since the look before, in the order it ended.
pub fn report_at(script: &[Scripted], (looked, at): (Option<Duration>, Duration), start: Instant) -> ActivityReport {
    let mut ended: Vec<&Scripted> = script
        .iter()
        .filter(|scripted| scripted.ends <= at && looked.is_none_or(|looked| scripted.ends > looked))
        .collect();
    ended.sort_by_key(|scripted| scripted.ends);
    ActivityReport {
        running: script
            .iter()
            .filter(|scripted| scripted.starts <= at && at < scripted.ends)
            .map(|scripted| scripted.entry(start))
            .collect(),
        ended: ended
            .into_iter()
            .map(|scripted| EndedActivity {
                entry: scripted.entry(start),
                ended: start + scripted.ends,
            })
            .collect(),
    }
}

/// The burst: the fetch from 0.05 s to 12 s; forty reads of `app` from 1 s, one every quarter of a second, each over in
/// a fifth of a second; a read of `lib` and one of `docs` that both end between the frames of 6.1 s and 6.2 s.
pub fn script() -> Vec<Scripted> {
    let mut all = vec![Scripted {
        number: 0,
        title: FETCH.to_owned(),
        cause: String::from("Fetch in the header"),
        starts: ms(50),
        ends: ms(12_000),
        is_read: false,
    }];
    for index in 0..40 {
        all.push(Scripted {
            number: 0,
            title: READ.to_owned(),
            cause: format!("Because src/file_{index}.rs changed."),
            starts: ms(1_000 + index * 250),
            ends: ms(1_200 + index * 250),
            is_read: true,
        });
    }
    for (title, cause, starts, ends) in [
        ("Reading lib", "Because lib/Cargo.toml changed.", 5_000, 6_150),
        ("Reading docs", "Because docs/README.md changed.", 5_300, 6_180),
    ] {
        all.push(Scripted {
            number: 0,
            title: title.to_owned(),
            cause: cause.to_owned(),
            starts: ms(starts),
            ends: ms(ends),
            is_read: true,
        });
    }
    all.sort_by_key(|scripted| scripted.starts);
    for (number, scripted) in (0..).zip(all.iter_mut()) {
        scripted.number = number;
    }
    all
}

/// The moments the window draws a frame: one as the burst starts, none until 0.9 s, as when nothing wakes the window
/// before the first read is asked for, then one every tenth of a second until 13 s, then one every second until 50 s,
/// while nothing runs any more.
pub fn frame_times() -> Vec<Duration> {
    let mut times = vec![ms(0)];
    times.extend((9..=130).map(|tenth| ms(tenth * 100)));
    times.extend((14..=50).map(Duration::from_secs));
    times
}

/// The part of the list a row is in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Section {
    /// What runs.
    Running,
    /// What ran.
    Ran,
}

/// A row of the list in a frame.
#[derive(Clone, Debug)]
pub struct Row {
    /// What tells it from the other rows, from one frame to the next.
    pub key: String,
    /// Its title, as drawn.
    pub title: String,
    /// Its time, as drawn.
    pub time: String,
    /// The part of the list it is in.
    pub section: Section,
    /// How far down the window it is drawn; none when it is not in sight.
    pub y: Option<f32>,
}

/// The list of a frame.
#[derive(Clone, Debug)]
pub struct Frame {
    /// When it is drawn, from the start of the burst.
    pub at: Duration,
    /// Its rows, from the top.
    pub rows: Vec<Row>,
}

/// What a row stands for in the burst: when the engine started it, when it ended, and whether it is read after read.
#[derive(Clone, Copy, Debug)]
pub struct Stood {
    /// When the first thing it stands for started.
    pub starts: Duration,
    /// When the last ended.
    pub ends: Duration,
    /// Whether it stands for reads in a row, whose time stops between one and the next.
    pub is_many: bool,
}

/// The lists of the frames, a line for each: its moment, then its rows from the top, `>` before one that runs and `=`
/// before one that ran, each with its time and how far down it is drawn.
pub fn transcript(frames: &[Frame]) -> String {
    let mut written = String::new();
    for frame in frames {
        let rows: Vec<String> = frame
            .rows
            .iter()
            .map(|row| {
                let mark = if row.section == Section::Running { '>' } else { '=' };
                let place = row
                    .y
                    .map_or_else(|| String::from("out of sight"), |y| format!("{y:.0}"));
                format!("{mark} {} ({}) @{place}", row.title, row.time)
            })
            .collect();
        let _ = writeln!(written, "{:>7.3} s  {}", frame.at.as_secs_f64(), rows.join(" | "));
    }
    written
}

/// The title of a row without the count of what it stands for: `Reading app` of `Reading app ×4`.
pub fn base_title(title: &str) -> &str {
    title.split(" \u{d7}").next().unwrap_or(title)
}

/// How long something has run, as the list says it.
pub fn seconds_said(elapsed: Duration) -> String {
    views::text::activity::duration(elapsed.as_secs())
}

/// The faults of the lists of a replay, each said with its moment: a row at another height from one frame to the next,
/// or pushed out of sight, while it stays in its part of the list, in a frame drawn before the fetch ends, or put
/// before one it was after; a row gone; a row that runs for something over in less than half a second; a title on two rows; a row that
/// runs whose time is not the time since the engine started it.
pub fn faults(frames: &[Frame], stands_for: &dyn Fn(&str) -> Option<Stood>, still_until: Duration) -> Vec<String> {
    let mut found = Vec::new();
    for frame in frames {
        within(frame, stands_for, &mut found);
    }
    for pair in frames.windows(2) {
        if let [before, after] = pair {
            between(before, after, still_until, &mut found);
        }
    }
    found
}

/// The faults of one frame.
fn within(frame: &Frame, stands_for: &dyn Fn(&str) -> Option<Stood>, found: &mut Vec<String>) {
    let at = frame.at.as_secs_f64();
    for (index, row) in frame.rows.iter().enumerate() {
        let title = base_title(&row.title);
        if frame
            .rows
            .iter()
            .take(index)
            .any(|other| base_title(&other.title) == title)
        {
            found.push(format!("twice: {at:.1} s, {title} is the title of more than one row"));
        }
        let (Section::Running, Some(stood)) = (row.section, stands_for(&row.key)) else {
            continue;
        };
        let ran = stood.ends.saturating_sub(stood.starts);
        if ran < LISTED_AFTER {
            found.push(format!(
                "flashed: {at:.1} s, {} is listed though it ran {} ms",
                row.key,
                ran.as_millis()
            ));
        }
        let longest = frame.at.saturating_sub(stood.starts);
        let is_right = if stood.is_many {
            said_seconds(&row.time) <= longest.as_secs()
        } else {
            row.time == seconds_said(longest)
        };
        if !is_right {
            found.push(format!(
                "time: {at:.1} s, {} says {} after {} ms",
                row.key,
                row.time,
                longest.as_millis()
            ));
        }
    }
}

/// The whole seconds a time says: `65` of `1 min 5 s`.
fn said_seconds(time: &str) -> u64 {
    let numbers: Vec<u64> = time.split(' ').filter_map(|word| word.parse().ok()).collect();
    match numbers.as_slice() {
        [minutes, seconds] => minutes * 60 + seconds,
        [seconds] => *seconds,
        _ => u64::MAX,
    }
}

/// The faults from one frame to the next.
fn between(before: &Frame, after: &Frame, still_until: Duration, found: &mut Vec<String>) {
    let at = after.at.as_secs_f64();
    for (index, row) in before.rows.iter().enumerate() {
        let Some((index_after, same)) = after.rows.iter().enumerate().find(|(_, other)| other.key == row.key) else {
            found.push(format!("went: {at:.1} s, {} is no longer listed", row.key));
            continue;
        };
        if row.section == Section::Ran && same.section == Section::Running {
            found.push(format!("moved: {at:.1} s, {} is back among what runs", row.key));
        }
        if row.section != same.section {
            continue;
        }
        if after.at < still_until {
            match (row.y, same.y) {
                (Some(y), Some(y_after)) if (y - y_after).abs() > 0.5 => {
                    found.push(format!("moved: {at:.1} s, {} from {y:.0} to {y_after:.0}", row.key));
                }
                (Some(_), None) => found.push(format!("moved: {at:.1} s, {} is pushed out of sight", row.key)),
                (None, Some(_)) => found.push(format!("moved: {at:.1} s, {} comes back into sight", row.key)),
                _ => {}
            }
        }
        let is_overtaken = before.rows.iter().take(index).any(|above| {
            above.section == row.section
                && after
                    .rows
                    .iter()
                    .skip(index_after + 1)
                    .any(|below| below.key == above.key && below.section == row.section)
        });
        if is_overtaken {
            found.push(format!("moved: {at:.1} s, {} is before a row it was after", row.key));
        }
    }
}

/// The faults by their kind, the word before the colon, each with how many times it was found and the first of them.
pub fn summary(faults: &[String]) -> String {
    let mut kinds: Vec<(&str, usize, &str)> = Vec::new();
    for fault in faults {
        let kind = fault.split(':').next().unwrap_or_default();
        match kinds.iter_mut().find(|(known, _, _)| *known == kind) {
            Some((_, count, _)) => *count += 1,
            None => kinds.push((kind, 1, fault)),
        }
    }
    kinds
        .iter()
        .map(|(kind, count, first)| format!("{kind} \u{d7}{count}, first {first}"))
        .collect::<Vec<_>>()
        .join("\n")
}
