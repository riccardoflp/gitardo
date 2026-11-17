#![allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]

use super::super::testing::{in_background, in_sequence, task, tasks, workspace};
use super::{Blocked, Run, StepState};
use crate::tasks::{TaskEnd, TaskReference, Tasks, run_order};

fn run(tasks: &Tasks, name: &str, running: &[&str]) -> Run {
    let running: Vec<TaskReference> = running.iter().map(|name| workspace(name)).collect();
    Run::new(run_order(tasks, &workspace(name)).unwrap(), &running)
}

fn names(started: &[TaskReference]) -> Vec<String> {
    started.iter().map(ToString::to_string).collect()
}

fn sequence() -> Tasks {
    tasks(vec![
        in_sequence("all", &["first", "second"]),
        task("first", &[]),
        task("second", &[]),
    ])
}

#[test]
fn tasks_run__in_sequence_the_second_starts_once_the_first_ended_with_exit_code_0() {
    let mut run = run(&sequence(), "all", &[]);
    assert_eq!(names(&run.start()), ["first"]);
    assert_eq!(
        names(&run.start()),
        Vec::<String>::new(),
        "nothing more until the first ends"
    );
    run.ended(&workspace("first"), Ok(()));
    assert_eq!(names(&run.start()), ["second"]);
    run.ended(&workspace("second"), Ok(()));
    assert_eq!(names(&run.start()), ["all"]);
    assert!(!run.is_over());
    run.ended(&workspace("all"), Ok(()));
    assert!(run.is_over());
}

#[test]
fn tasks_run__in_sequence_the_second_does_not_run_at_all_when_the_first_ended_with_1() {
    let mut run = run(&sequence(), "all", &[]);
    assert_eq!(names(&run.start()), ["first"]);
    run.ended(&workspace("first"), Err(TaskEnd::Code(1)));
    assert_eq!(names(&run.start()), Vec::<String>::new());
    let blocked = Blocked {
        by: workspace("first"),
        end: TaskEnd::Code(1),
    };
    assert_eq!(blocked.said(), "It did not run: `first` ended with exit code 1.");
    assert_eq!(
        run.state(&workspace("second")),
        Some(&StepState::DidNotRun(blocked.clone()))
    );
    assert_eq!(
        run.state(&workspace("all")),
        Some(&StepState::DidNotRun(blocked)),
        "the task says the first cause, not the task that did not run because of it"
    );
    assert!(run.is_over());
}

#[test]
fn tasks_run__in_parallel_both_start_at_once_and_one_that_fails_stops_nothing_that_runs() {
    let tasks = tasks(vec![
        task("all", &["first", "second"]),
        task("first", &[]),
        task("second", &[]),
    ]);
    let mut run = run(&tasks, "all", &[]);
    assert_eq!(names(&run.start()), ["first", "second"]);
    run.ended(&workspace("first"), Err(TaskEnd::Code(2)));
    assert_eq!(names(&run.start()), Vec::<String>::new());
    assert_eq!(run.state(&workspace("second")), Some(&StepState::Running), "it goes on");
    assert!(matches!(run.state(&workspace("all")), Some(StepState::DidNotRun(_))));
    assert!(!run.is_over(), "the second still runs");
    run.ended(&workspace("second"), Ok(()));
    assert!(run.is_over());
}

#[test]
fn tasks_run__a_task_that_depends_on_a_background_task_starts_once_that_one_is_ready() {
    let tasks = tasks(vec![task("test", &["serve"]), in_background("serve")]);
    let mut run = run(&tasks, "test", &[]);
    assert_eq!(names(&run.start()), ["serve"]);
    assert_eq!(names(&run.start()), Vec::<String>::new());
    run.ready(&workspace("serve"));
    assert_eq!(names(&run.start()), ["test"]);
    assert_eq!(
        run.state(&workspace("serve")),
        Some(&StepState::Ready),
        "it keeps running"
    );
    run.ended(&workspace("serve"), Err(TaskEnd::Stopped));
    assert_eq!(
        run.state(&workspace("test")),
        Some(&StepState::Running),
        "what started once it was ready goes on"
    );
}

#[test]
fn tasks_run__a_background_task_that_ends_before_it_is_ready_keeps_those_that_wait_from_running() {
    let tasks = tasks(vec![task("test", &["serve"]), in_background("serve")]);
    let mut run = run(&tasks, "test", &[]);
    assert_eq!(names(&run.start()), ["serve"]);
    run.ended(&workspace("serve"), Err(TaskEnd::Code(3)));
    assert_eq!(names(&run.start()), Vec::<String>::new());
    let StepState::DidNotRun(blocked) = run.state(&workspace("test")).unwrap() else {
        panic!("the task did not run");
    };
    assert_eq!(blocked.said(), "It did not run: `serve` ended with exit code 3.");
}

#[test]
fn tasks_run__a_task_it_depends_on_already_running_is_not_run_again_and_is_waited_for() {
    let mut run = run(&sequence(), "all", &["first"]);
    assert_eq!(names(&run.start()), Vec::<String>::new(), "the first runs already");
    assert_eq!(run.state(&workspace("first")), Some(&StepState::Running));
    run.ended(&workspace("first"), Ok(()));
    assert_eq!(names(&run.start()), ["second"]);
}

#[test]
fn tasks_run__a_task_stopped_or_not_started_says_so_to_those_that_waited() {
    let stopped = Blocked {
        by: workspace("first"),
        end: TaskEnd::Stopped,
    };
    assert_eq!(stopped.said(), "It did not run: `first` was stopped.");
    let not_started = Blocked {
        by: workspace("first"),
        end: TaskEnd::NotStarted(String::from("The system cannot find the file specified.\n")),
    };
    assert_eq!(
        not_started.said(),
        "It did not run: `first` could not be started: The system cannot find the file specified."
    );
}

#[test]
fn tasks_run__the_steps_of_a_run_are_listed_in_their_order_with_how_each_stands() {
    let mut run = run(&sequence(), "all", &[]);
    let _started = run.start();
    let listed: Vec<(String, StepState)> = run
        .steps()
        .map(|(task, state)| (task.to_string(), state.clone()))
        .collect();
    assert_eq!(
        listed,
        [
            (String::from("first"), StepState::Running),
            (String::from("second"), StepState::Waiting),
            (String::from("all"), StepState::Waiting),
        ]
    );
}
