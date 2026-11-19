//! The list of the Activity tab as the clock of the window keeps it from what the engine says
//! (`activity.what-is-running`, D64): what is listed and what is not, the reads that are one row, what ran and how
//! long it is kept, the list that scrolls; and the same rows in the activity indicator (`activity.status-bar`).

use std::time::{Duration, Instant};

use app::desktop::running::RunningClock;
use platform::accesskit::Role;
use render::{Point, ThemeName};
use ui::{InputEvent, Modifiers, WheelDelta, WheelEvent};
use views::activity::{ActivityState, RanTask, RunningTask};

use super::activity_burst::{Scripted, ms, report_at};
use super::activity_tab::{draw_running, tab};
use super::support::{NOW, Window, find, has, says};
use super::{repository_with_history, showing};

/// The size of the window.
const WIDE: (u32, u32) = (1000, 640);

/// What the clock lists after it looked at what the engine says at each of some moments of a script, in milliseconds:
/// what runs and what ran, at the last of them.
fn listed(script: &[Scripted], moments: &[u64]) -> (Vec<RunningTask>, Vec<RanTask>) {
    let mut clock = RunningClock::default();
    let start = Instant::now();
    let mut looked = None;
    let mut running = Vec::new();
    for at in moments.iter().map(|at| ms(*at)) {
        running = clock.tasks(&report_at(script, (looked, at), start), start + at);
        looked = Some(at);
    }
    (running, clock.ran())
}

/// The titles of what runs, each with how long it has run in milliseconds.
fn running_said(running: &[RunningTask]) -> Vec<(&str, u128)> {
    running
        .iter()
        .map(|task| (task.title.as_str(), task.elapsed.as_millis()))
        .collect()
}

/// The titles of what ran, each with how long it took in milliseconds.
fn ran_said(ran: &[RanTask]) -> Vec<(&str, u128)> {
    ran.iter()
        .map(|ran| (ran.task.title.as_str(), ran.task.elapsed.as_millis()))
        .collect()
}

#[test]
fn activity_what_is_running__what_runs_less_than_half_a_second_is_listed_neither_while_it_runs_nor_under_ran() {
    let script = [
        Scripted::operation(0, "Fetching origin", (0, 300)),
        Scripted::read(1, "Reading app", (100, 400)),
        Scripted::operation(2, "Pushing main to origin", (0, 5_000)),
    ];
    let mut window = Window::new(ThemeName::Dark, WIDE);
    let (running, ran) = listed(&script, &[200]);
    assert_eq!(running.len(), 3, "{running:?}");
    let tree = tab(
        &mut window,
        &ActivityState {
            running: &running,
            ran: &ran,
        },
    );
    assert!(says(&tree, "Nothing is running now"));
    for title in ["Fetching origin", "Reading app", "Pushing main to origin"] {
        assert!(!says(&tree, title), "{title} has run less than half a second");
    }
    let (running, ran) = listed(&script, &[200, 600, 2_000]);
    assert_eq!(running_said(&running), [("Pushing main to origin", 2_000)]);
    assert_eq!(ran, [], "what took less than half a second did not run for the list");
    let state = ActivityState {
        running: &running,
        ran: &ran,
    };
    let tree = tab(&mut window, &state);
    assert!(says(&tree, "Pushing main to origin") && says(&tree, "2 s"));
    assert!(!says(&tree, "Fetching origin") && !says(&tree, "RAN"));
}

#[test]
fn activity_what_is_running__a_fetch_that_ended_is_under_ran_with_its_time_and_still_there_ten_minutes_later() {
    let script = [Scripted::operation(0, "Fetching origin", (1_000, 5_200))];
    let (running, ran) = listed(&script, &[2_000, 5_300]);
    assert_eq!(running, []);
    assert_eq!(ran_said(&ran), [("Fetching origin", 4_200)]);
    let (running, later) = listed(&script, &[2_000, 5_300, 605_300]);
    assert_eq!(later, ran, "what ran is kept, however long ago");
    assert_eq!(later.first().map(|ran| ran.ended), Some(NOW + 5));
    let mut window = Window::new(ThemeName::Dark, WIDE);
    let tree = tab(
        &mut window,
        &ActivityState {
            running: &running,
            ran: &later,
        },
    );
    for words in [
        "Nothing is running now",
        "RAN",
        "Fetching origin",
        "03:00:05 \u{b7} took 4 s",
    ] {
        assert!(says(&tree, words), "{words}");
    }
}

#[test]
fn activity_what_is_running__ran_lists_the_latest_first_and_keeps_the_last_50() {
    let script: Vec<Scripted> = (0..60)
        .map(|number| {
            Scripted::operation(
                number,
                &format!("Step {number}"),
                (number * 1_000, number * 1_000 + 600),
            )
        })
        .collect();
    let moments: Vec<u64> = (0..60).map(|number| number * 1_000 + 800).collect();
    let (running, ran) = listed(&script, &moments);
    assert_eq!(running, []);
    let titles: Vec<&str> = ran.iter().map(|ran| ran.task.title.as_str()).collect();
    assert_eq!(titles.len(), 50);
    assert_eq!((titles.first(), titles.last()), (Some(&"Step 59"), Some(&"Step 10")));
    // What ended between two looks is listed by when it ended, whatever the order it started in.
    let together = [
        Scripted::operation(0, "Fetching origin", (0, 2_000)),
        Scripted::operation(1, "Pushing main to origin", (500, 1_500)),
    ];
    let (_, ran) = listed(&together, &[100, 3_000]);
    assert_eq!(
        ran_said(&ran),
        [("Fetching origin", 2_000), ("Pushing main to origin", 1_000)]
    );
}

#[test]
fn activity_what_is_running__reads_of_a_repository_one_after_the_other_are_one_row_that_counts_them() {
    let script = [
        Scripted::read(0, "Reading app", (0, 300)),
        Scripted::read(1, "Reading app", (350, 650)),
        Scripted::operation(2, "Fetching origin", (400, 9_000)),
        Scripted::read(3, "Reading app", (700, 1_000)),
        Scripted::read(4, "Reading app", (1_050, 1_350)),
    ];
    let (running, _) = listed(&script, &[100, 600]);
    assert_eq!(
        running_said(&running),
        [("Reading app \u{d7}2", 600), ("Fetching origin", 200)]
    );
    // The read that started and ended between two looks is counted, and the row stays where it is.
    let (running, ran) = listed(&script, &[100, 600, 1_200]);
    assert_eq!(
        running_said(&running),
        [("Reading app \u{d7}4", 1_200), ("Fetching origin", 800)]
    );
    assert_eq!(ran, []);
    // Between the last read and half a second after it, the row waits for the next, its time stopped.
    let (running, _) = listed(&script, &[100, 600, 1_200, 1_700]);
    assert_eq!(
        running_said(&running),
        [("Reading app \u{d7}4", 1_350), ("Fetching origin", 1_300)]
    );
    let (running, ran) = listed(&script, &[100, 600, 1_200, 1_700, 2_000]);
    assert_eq!(running_said(&running), [("Fetching origin", 1_600)]);
    assert_eq!(ran_said(&ran), [("Reading app \u{d7}4", 1_350)]);
}

#[test]
fn activity_what_is_running__reads_that_ended_one_after_the_other_are_one_row_under_ran_until_something_else_ends() {
    let script = [
        Scripted::read(0, "Reading app", (0, 1_000)),
        Scripted::read(1, "Reading app", (3_000, 4_000)),
        Scripted::operation(2, "Fetching origin", (5_000, 6_000)),
        Scripted::read(3, "Reading app", (8_000, 9_000)),
    ];
    let (_, ran) = listed(&script, &[500, 2_000, 3_500, 4_600]);
    assert_eq!(ran_said(&ran), [("Reading app \u{d7}2", 2_000)]);
    assert_eq!(
        ran.first().map(|ran| ran.ended),
        Some(NOW + 4),
        "when the last of them ended"
    );
    let (_, ran) = listed(&script, &[500, 2_000, 3_500, 4_600, 5_500, 8_500, 10_000]);
    assert_eq!(
        ran_said(&ran),
        [
            ("Reading app", 1_000),
            ("Fetching origin", 1_000),
            ("Reading app \u{d7}2", 2_000)
        ]
    );
}

#[test]
fn activity_what_is_running__with_more_rows_than_fit_the_wheel_brings_the_last_into_sight() {
    let script: Vec<Scripted> = (0..12)
        .map(|number| {
            Scripted::operation(
                number,
                &format!("Step {number}"),
                (number * 1_000, number * 1_000 + 600),
            )
        })
        .collect();
    let moments: Vec<u64> = (0..12).map(|number| number * 1_000 + 800).collect();
    let (running, ran) = listed(&script, &moments);
    let state = ActivityState {
        running: &running,
        ran: &ran,
    };
    let mut window = Window::new(ThemeName::Dark, WIDE);
    let tree = tab(&mut window, &state);
    assert!(says(&tree, "Step 11") && !says(&tree, "Step 0"));
    let turn = InputEvent::Wheel(WheelEvent {
        delta: WheelDelta::Lines { x: 0.0, y: 40.0 },
        position: Point::new(500.0, 520.0),
        modifiers: Modifiers::NONE,
    });
    let _ = window.frame(&[turn], |ui| {
        ui.clip(super::activity_tab::TAB, |ui| {
            views::activity::what_is_running_view(&state, ui)
        })
    });
    let mut tree = tab(&mut window, &state);
    for _ in 0..20 {
        window.now += Duration::from_millis(50);
        tree = tab(&mut window, &state);
    }
    assert!(says(&tree, "Step 0") && !says(&tree, "Step 11"));
    assert!(
        says(&tree, "WHAT IS RUNNING"),
        "the title of the column stays over the list"
    );
}

#[test]
fn activity_status_bar__reads_one_after_the_other_are_one_entry_of_the_indicator_that_counts_them() {
    let (repository, _remote) = repository_with_history();
    let state = showing(repository.path().to_path_buf());
    let script = [
        Scripted::read(0, "Reading app", (0, 300)),
        Scripted::read(1, "Reading app", (350, 650)),
        Scripted::read(2, "Reading app", (700, 1_200)),
        Scripted::operation(3, "Fetching origin", (900, 9_000)),
    ];
    let mut window = Window::new(ThemeName::Dark, WIDE);
    let (running, ran) = listed(&script, &[100, 400]);
    let (_, tree) = draw_running(&mut window, &state, (&running, &ran), &[]);
    assert!(
        !has(&tree, Role::Button, "Reading app \u{d7}2 (0 s)"),
        "nothing is shown before half a second"
    );
    let (running, ran) = listed(&script, &[100, 400, 1_100]);
    let (_, tree) = draw_running(&mut window, &state, (&running, &ran), &[]);
    let _indicator = find(&tree, Role::Button, "Reading app \u{d7}3 (1 s)");
    assert_eq!(
        views::activity::tooltip(&running),
        "Reading app \u{d7}3 (1 s)\nClick to show Activity in the bottom panel.",
        "the fetch, which has run a fifth of a second, is not listed yet"
    );
}

#[test]
fn activity_what_is_running__an_operation_that_asks_git_for_its_progress_has_its_last_line_under_its_title() {
    let start = Instant::now();
    let mut clone = engine::ActivityEntry {
        number: 7,
        description: model::contract::Text::from("Clone https://example.com/acme/lib.git into lib"),
        cause: model::contract::Text::from("repo.clone"),
        started: model::Timestamp(NOW),
        since: start,
        is_read: false,
        progress: Some(String::from("Receiving objects:  12% (321/2671)")),
    };
    let mut clock = RunningClock::default();
    let report = |entry: &engine::ActivityEntry| engine::ActivityReport {
        running: vec![entry.clone()],
        ended: Vec::new(),
    };
    let running = clock.tasks(&report(&clone), start + ms(800));
    assert_eq!(
        running.first().and_then(|task| task.detail.as_deref()),
        Some("Receiving objects:  12% (321/2671)")
    );
    clone.progress = Some(String::from(
        "Receiving objects:  45% (1203/2671), 1.20 MiB | 3.40 MiB/s",
    ));
    let running = clock.tasks(&report(&clone), start + ms(1_600));
    let mut window = Window::new(ThemeName::Dark, WIDE);
    let tree = tab(
        &mut window,
        &ActivityState {
            running: &running,
            ran: &[],
        },
    );
    for words in [
        "Clone https://example.com/acme/lib.git into lib",
        "Receiving objects:  45% (1203/2671), 1.20 MiB | 3.40 MiB/s",
    ] {
        assert!(says(&tree, words), "{words}");
    }
}
