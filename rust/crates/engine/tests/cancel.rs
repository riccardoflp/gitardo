//! A plan cancelled while it runs, or before it starts: the process of the step running is killed, no step after it
//! starts, and the log says so.

#![cfg(test)]

use std::path::Path;
use std::thread;
use std::time::{Duration, Instant};

use engine::{CommandLog, Engine, FixedClock, LogKind, LogLevel, StepFailure};
use git::{CancelToken, Git};
use model::Timestamp;
use model::contract::{Danger, Plan, Step, Text, Undo};
use testkit::Repository;

/// Long enough that a test waiting for it to end on its own would be seen to.
const LONG: Duration = Duration::from_secs(20);

/// A command of the shell that waits [`LONG`] and prints nothing.
fn waiting_command() -> String {
    let seconds = LONG.as_secs();
    if cfg!(windows) {
        format!("ping -n {seconds} 127.0.0.1 > NUL")
    } else {
        format!("sleep {seconds}")
    }
}

/// A plan of a step that waits, then a step that writes `after.txt`, which must never run.
fn plan(folder: &Path, waiting: Step) -> Plan {
    Plan {
        title: Text::from("Wait, then write"),
        steps: vec![
            waiting,
            Step::Write {
                path: folder.join("after.txt"),
                contents: b"written".to_vec(),
            },
        ],
        danger: Danger::None,
        undo: Undo::Impossible(Text::from("A test plan.")),
        after: Vec::new(),
        on_failure: Vec::new(),
    }
}

fn engine(repository: &Repository, cancel: &CancelToken) -> Engine {
    let mut engine = Engine::new(
        Git::new(repository.path()),
        CommandLog::new(LogLevel::Info),
        Box::new(FixedClock(Timestamp(0))),
    );
    engine.cancel_with(cancel.clone());
    engine
}

/// Runs the plan, cancelling it a moment after it started; how it ended and how long it took.
fn run_and_cancel(engine: &mut Engine, plan: &Plan, cancel: &CancelToken) -> (StepFailure, Duration) {
    let canceller = cancel.clone();
    let cancelling = thread::spawn(move || {
        thread::sleep(Duration::from_millis(300));
        canceller.cancel();
    });
    let started = Instant::now();
    let failure = engine.run_plan(plan).expect_err("a cancelled plan does not end well");
    let took = started.elapsed();
    cancelling.join().unwrap();
    (failure, took)
}

fn assert_cancelled_at_the_first_step(engine: &Engine, failure: &StepFailure, took: Duration, repository: &Repository) {
    assert!(failure.cancelled, "{failure:?}");
    assert_eq!(failure.ran, Vec::<String>::new());
    assert!(took < LONG / 2, "the step was not stopped: it took {took:?}");
    assert!(
        !repository.path().join("after.txt").exists(),
        "no step after the cancelled one runs"
    );
    let kinds: Vec<(LogKind, String)> = engine
        .log()
        .entries()
        .map(|entry| (entry.kind.clone(), entry.line.clone()))
        .collect();
    assert_eq!(
        kinds,
        [
            (LogKind::Step, failure.failed.clone()),
            (LogKind::Failure, String::from("Cancelled."))
        ]
    );
}

#[test]
fn a_command_of_the_shell_is_killed_when_the_plan_is_cancelled() {
    let repository = Repository::new();
    let cancel = CancelToken::new();
    let mut engine = engine(&repository, &cancel);
    let waiting = Step::Run {
        command: waiting_command(),
        directory: repository.path().to_path_buf(),
    };
    let plan = plan(repository.path(), waiting);
    let (failure, took) = run_and_cancel(&mut engine, &plan, &cancel);
    assert_cancelled_at_the_first_step(&engine, &failure, took, &repository);
}

#[test]
fn git_is_killed_when_the_plan_is_cancelled() {
    let repository = Repository::new();
    let cancel = CancelToken::new();
    let mut engine = engine(&repository, &cancel);
    // An alias of git that runs a shell command: git waits for it as for any of its own work.
    let alias = format!("alias.wait=!sleep {}", LONG.as_secs());
    let plan = plan(
        repository.path(),
        Step::git(repository.path().to_path_buf(), ["-c", &alias, "wait"]),
    );
    let (failure, took) = run_and_cancel(&mut engine, &plan, &cancel);
    assert_cancelled_at_the_first_step(&engine, &failure, took, &repository);
}

#[test]
fn a_plan_cancelled_before_it_starts_runs_and_logs_nothing() {
    let repository = Repository::new();
    let cancel = CancelToken::new();
    let mut engine = engine(&repository, &cancel);
    cancel.cancel();
    let plan = plan(
        repository.path(),
        Step::git(repository.path().to_path_buf(), ["status"]),
    );
    let failure = engine.run_plan(&plan).expect_err("a cancelled plan does not run");
    assert!(failure.cancelled);
    assert_eq!(failure.failed, "git status");
    assert_eq!(engine.log().entries().count(), 0);
    assert!(!repository.path().join("after.txt").exists());
}
