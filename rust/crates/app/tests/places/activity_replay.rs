//! The list of what runs, replayed through a burst of changes (M18, D64): what the engine says at each frame is given
//! to the clock of the window, the window is drawn, and the list of each frame is written, so that a row moving,
//! flashing or going while it is read fails the test.

use std::path::Path;
use std::time::{Duration, Instant};

use app::desktop::running::RunningClock;
use app::store::{Message, State, ViewData};
use engine::ActivityEntry;
use model::Timestamp;
use model::contract::Text;
use platform::accesskit::TreeUpdate;
use render::{Point, ThemeName};
use ui::{InputEvent, Modifiers, WheelDelta, WheelEvent};
use views::Intent;
use views::activity::RunningTask;

use super::activity_burst::{
    FETCH, Frame, Row, Scripted, Section, Stood, faults, frame_times, script, seconds_said, summary, transcript,
};
use super::support::{NOW, Window, run};
use super::{repository_with_history, showing};

/// The size of the window.
const WIDE: (u32, u32) = (1000, 640);
/// The least height of a row in sight: one cut by the foot of the list is lower.
const IN_SIGHT: f32 = 16.0;

/// What the engine says runs at a moment of the burst, in the order it started.
fn entries_at(script: &[Scripted], at: Duration) -> Vec<ActivityEntry> {
    script
        .iter()
        .filter(|scripted| scripted.starts <= at && at < scripted.ends)
        .map(|scripted| ActivityEntry {
            number: scripted.number,
            description: Text::from(scripted.title.clone()),
            cause: Text::from(scripted.cause.clone()),
            started: Timestamp(NOW + i64::try_from(scripted.starts.as_secs()).unwrap()),
        })
        .collect()
}

/// How far down the window some words are drawn, when they are in sight.
fn drawn_at(tree: &TreeUpdate, words: &str) -> Option<f32> {
    tree.nodes
        .iter()
        .filter(|(_, node)| node.label() == Some(words) || node.value() == Some(words))
        .filter_map(|(_, node)| node.bounds())
        .find(|bounds| bounds.y1 - bounds.y0 >= f64::from(IN_SIGHT))
        .map(|bounds| {
            #[allow(clippy::cast_possible_truncation, reason = "the bounds of a window fit in an f32")]
            let y = bounds.y0 as f32;
            y
        })
}

/// A window whose list of what runs is open, with the clock that says how long each thing has run.
struct Stage {
    /// The repository shown, kept while the window is.
    _repository: (testkit::Repository, testkit::Remote),
    state: State,
    window: Window,
    clock: RunningClock,
    /// The moment the burst starts.
    start: Instant,
}

impl Stage {
    fn new() -> Stage {
        let repository = repository_with_history();
        let mut state = showing(repository.0.path().to_path_buf());
        let _opened = run(&mut state, Message::Intent(Intent::OpenActivity));
        Stage {
            _repository: repository,
            state,
            window: Window::new(ThemeName::Dark, WIDE),
            clock: RunningClock::default(),
            start: Instant::now(),
        }
    }

    /// What the clock lists at a moment of the burst: what runs, then what ran.
    fn listed(&mut self, at: Duration, script: &[Scripted]) -> (Vec<RunningTask>, Vec<RunningTask>) {
        let running = self.clock.tasks(&entries_at(script, at), self.start + at);
        (running, self.clock.finished())
    }

    /// Draws the frame of a moment of the burst, with some input, and gives its list.
    fn frame(&mut self, at: Duration, script: &[Scripted], events: &[InputEvent]) -> Frame {
        let (running, finished) = self.listed(at, script);
        let data = ViewData::new(&self.state, running.clone(), NOW).with_finished(finished.clone());
        self.window.now = Duration::from_secs(1) + at;
        let state = &self.state;
        let (_, tree) = self
            .window
            .frame(events, |ui| views::view(&data.window_state(state), ui));
        let row = |task: &RunningTask, section| {
            let key = task.detail.clone().unwrap_or_default();
            Row {
                y: drawn_at(&tree, &key),
                key,
                title: task.title.clone(),
                time: seconds_said(task.elapsed),
                section,
            }
        };
        let mut rows: Vec<Row> = running.iter().map(|task| row(task, Section::Running)).collect();
        rows.extend(finished.iter().map(|task| row(task, Section::Ran)));
        Frame { at, rows }
    }

    /// Whether turning the wheel over the list brings its last row into sight.
    fn scrolls_to_the_last_row(&mut self, at: Duration, script: &[Scripted], over: f32) -> bool {
        let turn = InputEvent::Wheel(WheelEvent {
            delta: WheelDelta::Lines { x: 0.0, y: 12.0 },
            position: Point::new(400.0, over),
            modifiers: Modifiers::NONE,
        });
        let _ = self.frame(at, script, &[turn]);
        let after = self.frame(at, script, &[]);
        after.rows.last().is_some_and(|row| row.y.is_some())
    }
}

/// What the row of a key stands for: the thing of the burst that was asked for by the cause the key is.
fn stands_for(script: &[Scripted], key: &str) -> Option<Stood> {
    script
        .iter()
        .find(|scripted| scripted.cause == key)
        .map(|scripted| Stood {
            starts: scripted.starts,
            ends: scripted.ends,
            is_many: false,
        })
}

/// Replays the burst in a window, and gives the list of every frame and whether a row out of sight could not be
/// reached with the wheel.
fn replay(script: &[Scripted]) -> (Vec<Frame>, Option<String>) {
    let mut stage = Stage::new();
    let mut frames = Vec::new();
    let mut cut = None;
    for at in frame_times() {
        let frame = stage.frame(at, script, &[]);
        let hidden = frame.rows.iter().filter(|row| row.y.is_none()).count();
        let first = frame.rows.iter().find_map(|row| row.y);
        if let (true, None, Some(over)) = (hidden > 0, &cut, first)
            && !stage.scrolls_to_the_last_row(at, script, over)
        {
            cut = Some(format!(
                "cut: {:.1} s, {hidden} of {} rows are out of sight and the wheel does not bring the last one in",
                at.as_secs_f64(),
                frame.rows.len()
            ));
        }
        frames.push(frame);
    }
    (frames, cut)
}

/// Writes the lists of the frames in the folder of what the tests leave, and gives where.
fn write(frames: &[Frame]) -> std::path::PathBuf {
    let file = Path::new(env!("CARGO_TARGET_TMPDIR")).join("activity-replay.txt");
    std::fs::write(&file, transcript(frames)).unwrap();
    file
}

#[test]
#[ignore = "the list of the drawer jumps: phase 3 of M18 lays it out by the rules of D64 and turns this on"]
fn activity_what_is_running__during_a_fetch_with_forty_reads_in_a_row_no_row_moves_flashes_or_goes() {
    let script = script();
    let fetch_ends = script.iter().find(|scripted| scripted.title == FETCH).unwrap().ends;
    let (frames, cut) = replay(&script);
    let written = write(&frames);
    let mut found = faults(&frames, &|key| stands_for(&script, key), fetch_ends);
    found.extend(cut);
    assert!(
        found.is_empty(),
        "{} faults, the list of every frame is in {}:\n{}",
        found.len(),
        written.display(),
        summary(&found)
    );
}

#[test]
#[ignore = "what ends within the same frame is listed in an order that changes from one run to the next: phase 3 of \
            M18 turns this on"]
fn activity_what_is_running__two_reads_that_end_within_the_same_frame_are_listed_in_the_same_order_each_time() {
    let script = script();
    let orders: Vec<Vec<String>> = (0..16)
        .map(|_| {
            let mut clock = RunningClock::default();
            let start = Instant::now();
            let mut order = Vec::new();
            for at in frame_times() {
                let _running = clock.tasks(&entries_at(&script, at), start + at);
                let ran: Vec<String> = clock.finished().into_iter().filter_map(|task| task.detail).collect();
                order.push(ran.join(" | "));
            }
            order
        })
        .collect();
    let first = orders.first().unwrap();
    let differing = orders.iter().filter(|order| *order != first).count();
    assert_eq!(differing, 0, "{differing} of 16 replays list what ran in another order");
}
