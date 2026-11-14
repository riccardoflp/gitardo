#![allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
#![allow(clippy::unwrap_used, reason = "a test that cannot go on fails")]

use std::path::PathBuf;
use std::time::Duration;

use super::{ReadAction, ReadEvent, ReadSchedule, schedule_reads};
use crate::repo::watch::{Change, ReadCause, watch_changes};

/// A window that follows the schedule as the application does: it starts the reads it is asked to, each taking a
/// fixed time, and wakes the schedule at the moments it names.
struct Window {
    /// The schedule.
    schedule: ReadSchedule,
    /// The reads under way, with the moment each ends.
    running: Vec<(u64, Duration)>,
    /// The moments asked for.
    wakes: Vec<Duration>,
    /// Every read started: its number, its moment and how many in a row it was.
    started: Vec<(u64, Duration, u32)>,
    /// The reads shown and thrown away, by number.
    shown: Vec<u64>,
    /// The reads thrown away, by number.
    discarded: Vec<u64>,
    /// The most reads under way at once.
    most_at_once: usize,
    /// When the bar stopped.
    bar_stopped: Option<Duration>,
    /// How long each read takes.
    read_takes: Duration,
    /// The moment it is.
    clock: Duration,
}

impl Window {
    fn new(read_takes: Duration) -> Window {
        Window {
            schedule: ReadSchedule::default(),
            running: Vec::new(),
            wakes: Vec::new(),
            started: Vec::new(),
            shown: Vec::new(),
            discarded: Vec::new(),
            most_at_once: 0,
            bar_stopped: None,
            read_takes,
            clock: Duration::ZERO,
        }
    }

    fn tell(&mut self, event: &ReadEvent, at: Duration) {
        let (next, actions) = schedule_reads(&self.schedule, event);
        self.schedule = next;
        for action in actions {
            match action {
                ReadAction::Start(read) => {
                    self.running.push((read.number, at + self.read_takes));
                    self.most_at_once = self.most_at_once.max(self.running.len());
                    self.started.push((read.number, at, read.in_a_row));
                }
                ReadAction::WakeAt(moment) => self.wakes.push(moment),
                ReadAction::Show(number) => self.shown.push(number),
                ReadAction::Discard(number) => {
                    self.discarded.push(number);
                    self.running.retain(|(running, _)| *running != number);
                }
                ReadAction::StopBar => self.bar_stopped = Some(at),
            }
        }
    }

    fn change(&mut self, path: &str, at: Duration) {
        let request = watch_changes(&Change::WorkingTree(PathBuf::from(path))).unwrap();
        self.tell(&ReadEvent::Changed { request, at }, at);
    }

    /// Lets time go on to a moment, millisecond by millisecond: reads end, and the schedule wakes when it asked.
    fn run_until(&mut self, end: Duration) {
        let mut now = self.now();
        while now < end {
            now += Duration::from_millis(1);
            self.advance(now);
        }
    }

    fn advance(&mut self, now: Duration) {
        let ended: Vec<u64> = self
            .running
            .iter()
            .filter(|(_, ends)| *ends <= now)
            .map(|(number, _)| *number)
            .collect();
        self.running.retain(|(_, ends)| *ends > now);
        for number in ended {
            self.tell(&ReadEvent::Finished(number), now);
        }
        if self.wakes.iter().any(|moment| *moment <= now) {
            self.wakes.retain(|moment| *moment > now);
            self.tell(&ReadEvent::Tick { at: now }, now);
        }
        self.clock = now;
    }

    fn now(&self) -> Duration {
        self.clock
    }
}

fn milliseconds(count: u64) -> Duration {
    Duration::from_millis(count)
}

#[test]
fn repo_schedule_reads__ten_files_written_within_half_a_second_lead_to_one_read_a_second_after_the_first() {
    let mut window = Window::new(milliseconds(100));
    for index in 0..10 {
        let at = milliseconds(index * 50);
        window.run_until(at);
        window.change(&format!("target/out{index}.o"), at);
    }
    window.run_until(milliseconds(3_000));
    assert_eq!(window.started.len(), 1, "{:?}", window.started);
    assert_eq!(window.started.first().map(|read| read.1), Some(milliseconds(1_000)));
}

#[test]
fn repo_schedule_reads__a_file_changing_every_200_ms_for_5_s_reads_one_after_another_and_counts_them() {
    let mut window = Window::new(milliseconds(300));
    for index in 0..25 {
        let at = milliseconds(index * 200);
        window.run_until(at);
        window.change("src/a.rs", at);
    }
    window.run_until(milliseconds(8_000));
    assert_eq!(window.most_at_once, 1, "never two reads at once");
    assert!(window.started.len() >= 4, "{:?}", window.started);
    let counts: Vec<u32> = window.started.iter().map(|read| read.2).collect();
    assert_eq!(counts.first(), Some(&1));
    assert!(
        counts.windows(2).all(|pair| pair[1] == pair[0] + 1),
        "counted in a row: {counts:?}"
    );
    assert_eq!(window.running, [] as [(u64, Duration); 0]);
}

#[test]
fn repo_schedule_reads__refresh_during_a_read_leads_to_exactly_one_more_read() {
    let mut window = Window::new(milliseconds(500));
    window.tell(&ReadEvent::Asked(ReadCause::Refresh), milliseconds(0));
    window.run_until(milliseconds(100));
    for _ in 0..3 {
        window.tell(&ReadEvent::Asked(ReadCause::Refresh), milliseconds(100));
    }
    window.run_until(milliseconds(3_000));
    assert_eq!(window.started.len(), 2, "{:?}", window.started);
    assert_eq!(window.started.get(1).map(|read| read.1), Some(milliseconds(500)));
    assert_eq!(window.most_at_once, 1);
}

#[test]
fn repo_schedule_reads__nothing_is_read_while_an_operation_runs_and_one_read_follows_it() {
    let mut window = Window::new(milliseconds(100));
    window.tell(&ReadEvent::OperationStarted, milliseconds(0));
    for index in 0..10 {
        let at = milliseconds(index * 100);
        window.run_until(at);
        window.change("src/a.rs", at);
        window.tell(&ReadEvent::Asked(ReadCause::Refresh), at);
    }
    window.run_until(milliseconds(3_000));
    assert!(window.started.is_empty(), "{:?}", window.started);
    window.tell(&ReadEvent::OperationEnded, milliseconds(3_000));
    window.run_until(milliseconds(6_000));
    assert_eq!(window.started.len(), 1);
}

#[test]
fn repo_schedule_reads__a_read_started_before_a_commit_of_gitardo_is_thrown_away_and_read_again() {
    let mut window = Window::new(milliseconds(500));
    window.tell(&ReadEvent::Asked(ReadCause::Refresh), milliseconds(0));
    window.run_until(milliseconds(100));
    window.tell(&ReadEvent::OperationStarted, milliseconds(100));
    window.run_until(milliseconds(700));
    assert_eq!(
        window.discarded,
        [0],
        "the read half from before the commit is not shown"
    );
    assert_eq!(window.shown, [] as [u64; 0]);
    window.tell(&ReadEvent::OperationEnded, milliseconds(700));
    window.run_until(milliseconds(2_000));
    assert_eq!(window.shown, [1], "the read after the commit is shown");
}

#[test]
fn repo_schedule_reads__choosing_local_shows_the_bar_until_its_commits_are_there_while_files_keep_changing() {
    let mut window = Window::new(milliseconds(300));
    window.change("target/build.o", milliseconds(0));
    window.run_until(milliseconds(1_100));
    assert_eq!(window.started.len(), 1, "a read for the build is under way");
    window.tell(&ReadEvent::Asked(ReadCause::BranchesShown), milliseconds(1_100));
    assert!(window.schedule.is_bar_running());
    for index in 0..30 {
        let at = milliseconds(1_100 + index * 100);
        window.run_until(at);
        window.change("target/build.o", at);
    }
    window.run_until(milliseconds(6_000));
    assert_eq!(
        window.bar_stopped,
        Some(milliseconds(1_600)),
        "the read started after Local ends at 1.6 s"
    );
    assert!(
        window.started.len() > 3,
        "the files go on being read: {:?}",
        window.started
    );
}

#[test]
fn a_minimized_window_reads_nothing_for_its_changes() {
    let mut window = Window::new(milliseconds(100));
    window.tell(&ReadEvent::Minimized(true), milliseconds(0));
    window.change("src/a.rs", milliseconds(0));
    window.run_until(milliseconds(2_000));
    assert_eq!(window.started, [] as [(u64, Duration, u32); 0]);
    window.tell(&ReadEvent::Minimized(false), milliseconds(2_000));
    window.change("src/a.rs", milliseconds(2_000));
    window.run_until(milliseconds(4_000));
    assert_eq!(window.started.len(), 1);
}

#[test]
fn a_change_with_a_shorter_delay_brings_the_read_forward() {
    let mut window = Window::new(milliseconds(100));
    window.change("src/a.rs", milliseconds(0));
    let request = watch_changes(&Change::GitFolder(PathBuf::from("HEAD"))).unwrap();
    window.run_until(milliseconds(100));
    window.tell(
        &ReadEvent::Changed {
            request,
            at: milliseconds(100),
        },
        milliseconds(100),
    );
    window.run_until(milliseconds(2_000));
    assert_eq!(window.started.first().map(|read| read.1), Some(milliseconds(500)));
    assert_eq!(window.started.len(), 1);
}
