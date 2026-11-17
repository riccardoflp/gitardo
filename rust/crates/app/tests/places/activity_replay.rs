//! The list of what runs, replayed through a burst of changes (M18, D64): what the engine says at each frame is given
//! to the clock of the window, the window is drawn, and the list of each frame is written, so that a row moving,
//! flashing or going while it is read fails the test.

use std::path::Path;
use std::time::{Duration, Instant};

use app::desktop::running::RunningClock;
use app::store::{Message, State, ViewData};
use platform::accesskit::TreeUpdate;
use render::{Point, ThemeName};
use ui::{InputEvent, Modifiers, WheelDelta, WheelEvent};
use views::Intent;
use views::activity::{RanTask, RunningTask};

use super::activity_burst::{
    FETCH, Frame, LISTED_AFTER, READ, Row, Scripted, Section, Stood, base_title, faults, frame_times, ms, report_at,
    script, seconds_said, summary, transcript,
};
use super::support::{NOW, Window, run};
use super::{repository_with_history, showing};

/// The size of the window.
const WIDE: (u32, u32) = (1000, 640);
/// The least height of a row in sight.
const IN_SIGHT: f32 = 16.0;

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

/// A window whose bottom panel shows its Activity tab, with the clock that keeps the list of what runs.
struct Stage {
    /// The repository shown, kept while the window is.
    _repository: (testkit::Repository, testkit::Remote),
    state: State,
    window: Window,
    clock: RunningClock,
    /// The moment the burst starts.
    start: Instant,
    /// The moment of the burst the engine was last asked at.
    looked: Option<Duration>,
    /// How long the window was given, beside the burst, to let a turn of the wheel glide to its end.
    glided: Duration,
}

impl Stage {
    fn new() -> Stage {
        let repository = repository_with_history();
        let mut state = showing(repository.0.path().to_path_buf());
        let _shown = run(
            &mut state,
            Message::Intent(Intent::Panel(views::panel::PanelIntent::ToggleActivity)),
        );
        Stage {
            _repository: repository,
            state,
            window: Window::new(ThemeName::Dark, WIDE),
            clock: RunningClock::default(),
            start: Instant::now(),
            looked: None,
            glided: Duration::ZERO,
        }
    }

    /// What the clock lists at a moment of the burst: what runs, then what ran.
    fn listed(&mut self, at: Duration, script: &[Scripted]) -> (Vec<RunningTask>, Vec<RanTask>) {
        let report = report_at(script, (self.looked, at), self.start);
        self.looked = Some(at);
        let running = self.clock.tasks(&report, self.start + at);
        (running, self.clock.ran())
    }

    /// Draws the frame of a moment of the burst, with some input, and gives its list: what has run for half a second,
    /// then what ran.
    fn frame(&mut self, at: Duration, script: &[Scripted], events: &[InputEvent]) -> Frame {
        let (running, ran) = self.listed(at, script);
        let data = ViewData::new(&self.state, running.clone(), NOW).with_ran(ran.clone());
        self.window.now = Duration::from_secs(1) + at + self.glided;
        let state = &self.state;
        let (_, tree) = self
            .window
            .frame(events, |ui| views::view(&data.window_state(state), ui));
        let row = |task: &RunningTask, section| Row {
            y: drawn_at(&tree, &task.title),
            key: base_title(&task.title).to_owned(),
            title: task.title.clone(),
            time: seconds_said(task.elapsed),
            section,
        };
        let mut rows: Vec<Row> = running
            .iter()
            .filter(|task| task.elapsed >= LISTED_AFTER)
            .map(|task| row(task, Section::Running))
            .collect();
        rows.extend(ran.iter().map(|ran| row(&ran.task, Section::Ran)));
        Frame { at, rows }
    }

    /// Whether turning the wheel over the list, and waiting for it to glide, brings its last row into sight.
    fn scrolls_to_the_last_row(&mut self, at: Duration, script: &[Scripted], over: f32) -> bool {
        let turn = InputEvent::Wheel(WheelEvent {
            delta: WheelDelta::Lines { x: 0.0, y: 12.0 },
            position: Point::new(400.0, over),
            modifiers: Modifiers::NONE,
        });
        let _ = self.frame(at, script, &[turn]);
        let mut after = self.frame(at, script, &[]);
        for _ in 0..20 {
            self.glided += ms(50);
            after = self.frame(at, script, &[]);
        }
        after.rows.last().is_some_and(|row| row.y.is_some())
    }
}

/// What the row of a key stands for: the things of the burst of that title, from the start of the first to the end of
/// the last.
fn stands_for(script: &[Scripted], key: &str) -> Option<Stood> {
    let same: Vec<&Scripted> = script.iter().filter(|scripted| scripted.title == key).collect();
    Some(Stood {
        starts: same.iter().map(|scripted| scripted.starts).min()?,
        ends: same.iter().map(|scripted| scripted.ends).max()?,
        is_many: same.iter().all(|scripted| scripted.is_read),
    })
}

/// Replays the burst in a window, and gives the list of every frame, with the first moment a row is out of sight and
/// how far down the first row in sight is then.
fn replay(script: &[Scripted]) -> (Vec<Frame>, Option<(Duration, f32)>) {
    let mut stage = Stage::new();
    let mut frames = Vec::new();
    let mut cut = None;
    for at in frame_times() {
        let frame = stage.frame(at, script, &[]);
        let is_cut = frame.rows.iter().any(|row| row.y.is_none());
        if let (true, None, Some(over)) = (is_cut, cut, frame.rows.iter().find_map(|row| row.y)) {
            cut = Some((at, over));
        }
        frames.push(frame);
    }
    (frames, cut)
}

/// Replays the burst again, up to a moment at which a row is out of sight, and says so as a fault when the wheel does
/// not bring the last row in.
fn cut_fault(script: &[Scripted], (cut_at, over): (Duration, f32)) -> Option<String> {
    let mut stage = Stage::new();
    for at in frame_times().into_iter().filter(|at| *at <= cut_at) {
        let _ = stage.frame(at, script, &[]);
    }
    (!stage.scrolls_to_the_last_row(cut_at, script, over)).then(|| {
        format!(
            "cut: {:.1} s, a row is out of sight and the wheel does not bring the last one in",
            cut_at.as_secs_f64()
        )
    })
}

/// Writes the lists of the frames in the folder of what the tests leave, and gives where.
fn write(frames: &[Frame]) -> std::path::PathBuf {
    let file = Path::new(env!("CARGO_TARGET_TMPDIR")).join("activity-replay.txt");
    std::fs::write(&file, transcript(frames)).unwrap();
    file
}

#[test]
fn activity_what_is_running__during_a_fetch_with_forty_reads_in_a_row_no_row_moves_flashes_or_goes() {
    let script = script();
    let fetch_ends = script.iter().find(|scripted| scripted.title == FETCH).unwrap().ends;
    let (frames, cut) = replay(&script);
    let written = write(&frames);
    let mut found = faults(&frames, &|key| stands_for(&script, key), fetch_ends);
    found.extend(cut.and_then(|cut| cut_fault(&script, cut)));
    assert!(
        found.is_empty(),
        "{} faults, the list of every frame is in {}:\n{}",
        found.len(),
        written.display(),
        summary(&found)
    );
    let titles = |at: Duration, section: Section| -> Vec<String> {
        let frame = frames.iter().find(|frame| frame.at == at).unwrap();
        let rows = frame.rows.iter().filter(|row| row.section == section);
        rows.map(|row| format!("{} ({})", row.title, row.time)).collect()
    };
    assert_eq!(
        titles(ms(900), Section::Running),
        ["Fetching origin (0 s)"],
        "the fetch, first drawn 0.85 s after it started, counts from its start"
    );
    assert_eq!(
        titles(ms(6_000), Section::Running),
        [
            "Fetching origin (5 s)",
            "Reading app \u{d7}21 (5 s)",
            "Reading lib (1 s)",
            "Reading docs (0 s)"
        ],
        "the reads of a repository are one row that counts them, each row in the order it started"
    );
    assert_eq!(
        titles(ms(13_000), Section::Ran),
        [
            "Fetching origin (11 s)",
            "Reading app \u{d7}40 (9 s)",
            "Reading docs (0 s)",
            "Reading lib (1 s)"
        ],
        "what ran, the latest first"
    );
    assert_eq!(titles(Duration::from_secs(50), Section::Ran).len(), 4, "what ran stays");
    assert!(frames.iter().all(|frame| {
        let reads = frame.rows.iter().filter(|row| row.key == READ);
        reads.count() <= 1
    }));
}

#[test]
fn activity_what_is_running__two_reads_that_end_within_the_same_frame_are_listed_in_the_same_order_each_time() {
    let script = script();
    let orders: Vec<Vec<String>> = (0..16)
        .map(|_| {
            let mut clock = RunningClock::default();
            let start = Instant::now();
            let mut looked = None;
            let mut order = Vec::new();
            for at in frame_times() {
                let _running = clock.tasks(&report_at(&script, (looked, at), start), start + at);
                looked = Some(at);
                let ran: Vec<String> = clock.ran().into_iter().map(|ran| ran.task.title).collect();
                order.push(ran.join(" | "));
            }
            order
        })
        .collect();
    let first = orders.first().unwrap();
    let differing = orders.iter().filter(|order| *order != first).count();
    assert_eq!(differing, 0, "{differing} of 16 replays list what ran in another order");
    assert_eq!(
        first.last().map(String::as_str),
        Some("Fetching origin | Reading app \u{d7}40 | Reading docs | Reading lib"),
        "the read that ended last is listed first"
    );
}
