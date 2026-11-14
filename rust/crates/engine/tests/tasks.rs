//! The tasks of a plan, the commands of a new worktree: handed over by the engine from the first one on, then run apart
//! one after the other, with what they print told as it comes and what is typed given to them.

#![cfg(test)]

use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

use engine::{
    Asker, CommandLog, Engine, Finished, FixedClock, LogLevel, TaskEvent, TaskRunner, TaskSteps, TaskTalk, TasksFailure,
};
use git::read::repo::{StateRequest, read_state};
use git::{CancelToken, Git};
use model::Timestamp;
use model::contract::{
    Answer, Answers, Context, Danger, FeatureId, FeatureSettings, Outcome, Plan, Question, Step, Target, Text, Undo,
};
use model::tasks::TaskEnd;
use testkit::Repository;

/// A task of a command in the folder of a repository.
fn task(repository: &Repository, command: &str) -> Step {
    Step::Task {
        command: command.to_owned(),
        directory: repository.path().to_path_buf(),
        name: String::from("fix"),
    }
}

/// A command that asks a name, reads a line, and says hello to it: a shell alias of git, which git runs with its own
/// shell on every system.
fn asking_command() -> String {
    let alias = "alias.ask=!printf 'Name? '; read name; echo Hello $name";
    if cfg!(windows) {
        format!("git -c \"{alias}\" ask")
    } else {
        // The alias holds single quotes, so `sh` is given it in double quotes, with its `$` kept from `sh`.
        format!("git -c \"{}\" ask", alias.replace('$', "\\$"))
    }
}

/// What the tasks told, with how they ended, the user typing a line once a task printed `typed` when it is given.
fn run(
    repository: &Repository,
    steps: &[Step],
    typed: Option<(&str, &str)>,
) -> (Vec<TaskEvent>, Result<(), TasksFailure>) {
    let (input, received) = mpsc::channel();
    let mut runner = TaskRunner::new(
        Git::new(repository.path()),
        LogLevel::Info,
        Box::new(FixedClock(Timestamp(0))),
        CancelToken::new(),
    );
    let mut told = Vec::new();
    let mut tell = |event: TaskEvent| {
        if let (Some((after, line)), TaskEvent::Printed(_, text)) = (typed, &event)
            && text.contains(after)
        {
            input.send(line.to_owned()).unwrap();
        }
        told.push(event);
    };
    let ended = runner.run(
        steps,
        &mut TaskTalk {
            input: &received,
            tell: &mut tell,
        },
    );
    (told, ended)
}

/// What a task told it printed, in one text.
fn printed(told: &[TaskEvent], index: usize) -> String {
    told.iter()
        .filter_map(|event| match event {
            TaskEvent::Printed(printed, text) if *printed == index => Some(text.as_str()),
            _ => None,
        })
        .collect()
}

#[test]
fn the_tasks_run_one_after_the_other_and_tell_what_they_print() {
    let repository = Repository::new();
    let (told, ended) = run(
        &repository,
        &[task(&repository, "echo one"), task(&repository, "echo two")],
        None,
    );
    assert_eq!(ended, Ok(()));
    let started: Vec<usize> = told
        .iter()
        .filter_map(|event| match event {
            TaskEvent::Started(index) => Some(*index),
            TaskEvent::Printed(..) => None,
        })
        .collect();
    assert_eq!(started, [0, 1]);
    assert_eq!(printed(&told, 0).trim(), "one");
    assert_eq!(printed(&told, 1).trim(), "two");
    let second = told.iter().position(|event| *event == TaskEvent::Started(1));
    let first_printed = told.iter().position(|event| matches!(event, TaskEvent::Printed(0, _)));
    assert!(first_printed < second, "{told:?}");
}

#[test]
fn a_task_that_fails_stops_those_after_it() {
    let repository = Repository::new();
    let steps = [
        task(&repository, "exit 3"),
        task(&repository, "echo never"),
        task(&repository, "echo nor this"),
    ];
    let (told, ended) = run(&repository, &steps, None);
    let Err(TasksFailure::Task(failure)) = ended else {
        panic!("{ended:?}")
    };
    assert_eq!(
        (failure.command.as_str(), failure.end, failure.after),
        ("exit 3", TaskEnd::Code(3), 2)
    );
    assert!(!told.contains(&TaskEvent::Started(1)), "{told:?}");
}

#[test]
fn a_task_is_given_what_the_user_types() {
    let repository = Repository::new();
    let (told, ended) = run(
        &repository,
        &[task(&repository, &asking_command())],
        Some(("Name?", "Ada")),
    );
    assert_eq!(ended, Ok(()));
    assert!(printed(&told, 0).contains("Hello Ada"), "{told:?}");
}

#[test]
fn a_task_stopped_ends_at_once_and_those_after_it_do_not_run() {
    let repository = Repository::new();
    let waiting = if cfg!(windows) {
        "ping -n 20 127.0.0.1 > NUL"
    } else {
        "sleep 20"
    };
    let cancel = CancelToken::new();
    let stopping = cancel.clone();
    let stopper = thread::spawn(move || {
        thread::sleep(Duration::from_millis(300));
        stopping.cancel();
    });
    let (_input, received) = mpsc::channel();
    let mut runner = TaskRunner::new(
        Git::new(repository.path()),
        LogLevel::Info,
        Box::new(FixedClock(Timestamp(0))),
        cancel,
    );
    let started = Instant::now();
    let ended = runner.run(
        &[task(&repository, waiting), task(&repository, "echo never")],
        &mut TaskTalk {
            input: &received,
            tell: &mut |_| {},
        },
    );
    stopper.join().unwrap();
    assert!(started.elapsed() < Duration::from_secs(10), "it was not stopped");
    let Err(TasksFailure::Task(failure)) = ended else {
        panic!("{ended:?}")
    };
    assert_eq!((failure.end, failure.after), (TaskEnd::Stopped, 1));
    let lines: Vec<String> = runner.take_log().into_iter().map(|entry| entry.line).collect();
    assert_eq!(lines, [format!("run {waiting}"), String::from("It was stopped.")]);
}

/// Runs every plan.
struct RunsAll;

impl Asker for RunsAll {
    fn ask(&mut self, _question: &Question) -> Option<Answer> {
        None
    }

    fn confirm(&mut self, _plan: &Plan) -> bool {
        true
    }
}

/// The steps of a plan that writes a file, runs a task, then writes another file.
fn write_then_run_steps(folder: &std::path::Path) -> Vec<Step> {
    vec![
        Step::Write {
            path: folder.join("marker.txt"),
            contents: b"made".to_vec(),
        },
        Step::Task {
            command: String::from("echo one"),
            directory: folder.to_path_buf(),
            name: String::from("fix"),
        },
        Step::Write {
            path: folder.join("after.txt"),
            contents: b"after".to_vec(),
        },
    ]
}

/// An operation that writes a file, runs a task, then writes another file.
fn write_then_run(context: &Context, _answers: &Answers) -> Outcome {
    Outcome::Run(Plan {
        title: Text::from("Write, then run"),
        steps: write_then_run_steps(&context.repository.folder),
        danger: Danger::None,
        undo: Undo::Impossible(Text::from("A test plan.")),
        after: Vec::new(),
        on_failure: Vec::new(),
    })
}

#[test]
fn the_engine_hands_over_the_steps_from_the_first_task_on() {
    let repository = Repository::new();
    let request = StateRequest {
        folder: repository.path().to_path_buf(),
    };
    let state = read_state(&Git::new(repository.path()), &request).unwrap();
    let context = Context {
        repository: std::sync::Arc::new(state),
        target: Target::Head,
        settings: FeatureSettings::default(),
        now: Timestamp(0),
    };
    let mut engine = Engine::new(
        Git::new(repository.path()),
        CommandLog::new(LogLevel::Info),
        Box::new(FixedClock(Timestamp(0))),
    );
    engine.run_tasks(TaskSteps::HandedOver);
    let finished = engine
        .run_feature(FeatureId("test"), write_then_run, &context, &mut RunsAll)
        .unwrap();
    let Finished::Ran { waiting, .. } = finished else {
        panic!("{finished:?}")
    };
    let steps = write_then_run_steps(&context.repository.folder);
    assert_eq!(waiting, steps.get(1..).unwrap());
    assert!(context.repository.folder.join("marker.txt").exists());
    assert!(
        !context.repository.folder.join("after.txt").exists(),
        "what follows a task waits for it"
    );
}
