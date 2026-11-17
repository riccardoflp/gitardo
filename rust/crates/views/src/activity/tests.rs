//! The words of what runs: those of `activity.describe-task`, which live in `model` but are checked here, where time
//! may be read; and the tooltip of the activity indicator.

#![allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]

use std::path::PathBuf;
use std::time::{Duration, Instant};

use model::activity::{Task, TaskKind, counted, describe_task};
use model::repo::ReadCause;

use super::{RunningTask, tooltip};

/// A read of `work/app` asked for by a change to `src/a.rs`.
fn read(is_search: bool, in_a_row: u32, started: Instant) -> Task {
    Task {
        kind: TaskKind::Read {
            name: String::from("work/app"),
            is_search,
            cause: Some(ReadCause::File(PathBuf::from("src/a.rs"))),
            in_a_row,
        },
        started,
    }
}

/// Something running for some seconds, with its detail.
fn running(title: &str, seconds: u64, detail: Option<&str>) -> RunningTask {
    RunningTask {
        title: title.to_owned(),
        detail: detail.map(str::to_owned),
        elapsed: Duration::from_secs(seconds),
    }
}

#[test]
fn activity_describe_task__the_third_read_in_a_row_says_so_and_why() {
    let started = Instant::now();
    let text = describe_task(&read(false, 3, started), started + Duration::from_secs(2));
    assert_eq!(text.title, "Reading work/app");
    assert_eq!(
        text.detail.as_deref(),
        Some("Read 3 times in a row: something changed during each read. Because src/a.rs changed.")
    );
    assert_eq!(text.elapsed, "2 s");
    let first = describe_task(&read(false, 1, started), started);
    assert_eq!(first.detail.as_deref(), Some("Because src/a.rs changed."));
}

#[test]
fn activity_describe_task__65_seconds_is_1_min_5_s_and_59_9_seconds_is_59_s() {
    let started = Instant::now();
    let operation = Task {
        kind: TaskKind::Operation {
            title: String::from("Pushing main to origin"),
        },
        started,
    };
    assert_eq!(
        describe_task(&operation, started + Duration::from_secs(65)).elapsed,
        "1 min 5 s"
    );
    assert_eq!(
        describe_task(&operation, started + Duration::from_millis(59_900)).elapsed,
        "59 s"
    );
}

#[test]
fn activity_describe_task__a_read_narrowed_by_a_search_in_the_changes_is_searching() {
    let started = Instant::now();
    assert_eq!(
        describe_task(&read(true, 1, started), started).title,
        "Searching work/app"
    );
    let compare = Task {
        kind: TaskKind::Comparison {
            from: String::from("main"),
            to: String::from("feature"),
        },
        started,
    };
    assert_eq!(
        describe_task(&compare, started).title,
        "Comparing main \u{2194} feature"
    );
}

#[test]
fn activity_status_bar__a_fetch_and_a_read_at_once_are_both_in_the_tooltip_the_older_first() {
    let tasks = [
        running("Fetching origin", 4, None),
        running("Reading work/app", 1, Some("Because .git/FETCH_HEAD changed.")),
    ];
    assert_eq!(
        tooltip(&tasks),
        "Fetching origin (4 s)\nReading work/app (1 s)\nBecause .git/FETCH_HEAD changed.\nClick to show Activity in the bottom panel."
    );
}

#[test]
fn activity_status_bar__what_has_run_less_than_half_a_second_is_not_in_the_tooltip() {
    let tasks = [
        running("Fetching origin", 4, None),
        RunningTask {
            title: String::from("Reading work/app"),
            detail: Some(String::from("Because src/a.rs changed.")),
            elapsed: Duration::from_millis(300),
        },
    ];
    assert_eq!(
        tooltip(&tasks),
        "Fetching origin (4 s)\nClick to show Activity in the bottom panel."
    );
}

#[test]
fn activity_describe_task__reads_one_after_the_other_are_titled_with_how_many_they_are_from_the_second_on() {
    assert_eq!(counted("Reading work/app", 1), "Reading work/app");
    assert_eq!(counted("Reading work/app", 4), "Reading work/app \u{d7}4");
}

#[test]
fn activity_status_bar__the_tooltip_of_a_read_asked_for_by_a_change_says_which_file_changed() {
    let started = Instant::now();
    let text = describe_task(&read(false, 1, started), started + Duration::from_secs(1));
    let task = RunningTask {
        title: text.title,
        detail: text.detail,
        elapsed: Duration::from_secs(1),
    };
    let said = tooltip(&[task]);
    assert!(said.contains("Because src/a.rs changed."), "{said}");
}
