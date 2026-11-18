#![allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]

use std::path::PathBuf;

use model::contract::{Answer, Answers, Context, FollowUp, Outcome, QuestionKind, Step, Target};
use model::tasks::{
    DependsOrder, Task, TaskChoice, TaskGroup, TaskKind, TaskReference, TaskTarget, Tasks, TasksToRun, Word,
};
use model::trust::Trust;

use super::{LEAVE, MAKE_FILE, NOT_TRUSTED, RESTART, RUNNING, STOP, TRUST, run_group, run_task, stop_task};
use crate::testing::{context, plan, question, state};

fn reference(name: &str) -> TaskReference {
    TaskReference::workspace(name)
}

/// A shell task of `tasks.json` that depends on others.
fn task(name: &str, depends_on: &[&str]) -> Task {
    let mut task = Task::named(reference(name), PathBuf::from("/work/app"));
    task.kind = TaskKind::Shell;
    task.command = Some(Word::plain(&format!("run-{name}")));
    task.depends_on = depends_on.iter().map(|name| reference(name)).collect();
    task
}

/// The tasks of the tests: `all` runs `build` then `test`, one after the other; `left` and `right` depend on each
/// other.
fn tasks() -> Tasks {
    let mut all = task("all", &["build", "test"]);
    all.depends_order = DependsOrder::Sequence;
    Tasks {
        workspace: vec![
            task("build", &[]),
            task("test", &[]),
            all,
            task("left", &["right"]),
            task("right", &["left"]),
            task("lost", &["gone"]),
        ],
        ..Tasks::default()
    }
}

/// A trusted folder asked for a task or a group, with some tasks running.
fn asked(choice: TaskChoice, running: &[&str]) -> Context {
    let target = TaskTarget {
        tasks: tasks(),
        choice,
        running: running.iter().map(|name| reference(name)).collect(),
    };
    let mut context = context(state(), Target::Task(Box::new(target)));
    context.settings.trust = Trust::Trusted;
    context
}

fn for_task(name: &str, running: &[&str]) -> Context {
    asked(TaskChoice::Task(reference(name)), running)
}

fn for_group(names: &[&str], running: &[&str]) -> Context {
    let group = TaskGroup {
        name: String::from("Servers"),
        tasks: names.iter().map(|name| reference(name)).collect(),
    };
    asked(TaskChoice::Group(group), running)
}

/// What a plan asks the window to run.
fn to_run(outcome: Outcome) -> TasksToRun {
    let plan = plan(outcome);
    assert_eq!(plan.steps, Vec::new(), "the window runs the tasks, not the engine");
    match plan.after.first() {
        Some(FollowUp::RunTasks(run)) => run.as_ref().clone(),
        other => panic!("expected tasks to run, got {other:?}"),
    }
}

fn refused(outcome: Outcome) -> String {
    match outcome {
        Outcome::Refuse(refusal) => refusal.message.to_string(),
        other => panic!("expected a refusal, got {other:?}"),
    }
}

/// The tasks of the runs asked for, each run in its order.
fn steps(run: &TasksToRun) -> Vec<Vec<String>> {
    run.orders
        .iter()
        .map(|order| order.steps.iter().map(|step| step.task.to_string()).collect())
        .collect()
}

#[test]
fn tasks_run__a_task_runs_after_those_it_depends_on_and_is_the_one_shown() {
    let run = to_run(run_task(&for_task("all", &[]), &Answers::new()));
    assert_eq!(steps(&run), [["build", "test", "all"]]);
    assert_eq!(run.show, Some(reference("all")));
    assert_eq!(run.restart, Vec::new());
    let second = run.orders.first().and_then(|order| order.steps.get(1)).unwrap();
    assert_eq!(
        second.waits_for,
        [reference("build")],
        "in a sequence, the second waits for the first"
    );
}

#[test]
fn tasks_run__a_task_that_runs_asks_to_restart_it_or_to_leave_it_running() {
    let context = for_task("build", &["build"]);
    let asked = question(run_task(&context, &Answers::new()));
    assert_eq!(asked.title.as_str(), "`build` is running.");
    let QuestionKind::Pick { choices } = &asked.kind else {
        panic!("expected a pick, got {:?}", asked.kind);
    };
    let labels: Vec<&str> = choices.iter().map(|choice| choice.label.as_str()).collect();
    assert_eq!(labels, ["Restart", "Leave it running"]);
    let restart = Answers::new().with(RUNNING, Answer::Choice(RESTART.to_owned()));
    let run = to_run(run_task(&context, &restart));
    assert_eq!(
        run.restart,
        [reference("build")],
        "the first run is ended before the second starts"
    );
    assert_eq!(steps(&run), [["build"]]);
    let leave = Answers::new().with(RUNNING, Answer::Choice(LEAVE.to_owned()));
    let run = to_run(run_task(&context, &leave));
    assert_eq!((steps(&run), run.restart.len()), (Vec::new(), 0), "nothing runs");
    assert_eq!(run.show, Some(reference("build")), "it is shown");
}

#[test]
fn tasks_run__two_tasks_that_depend_on_each_other_run_nothing_and_the_loop_is_said() {
    let said = refused(run_task(&for_task("left", &[]), &Answers::new()));
    assert!(said.contains("`left`") && said.contains("`right`"), "{said}");
    let said = refused(run_task(&for_task("lost", &[]), &Answers::new()));
    assert!(said.contains("`gone`"), "{said}");
}

#[test]
fn tasks_run__in_a_folder_not_trusted_nothing_runs() {
    let mut context = for_task("build", &[]);
    context.settings.trust = Trust::NotTrusted;
    assert_eq!(refused(run_task(&context, &Answers::new())), NOT_TRUSTED);
    let mut group = for_group(&["build", "test"], &[]);
    group.settings.trust = Trust::NotTrusted;
    assert_eq!(refused(run_group(&group, &Answers::new())), NOT_TRUSTED);
}

#[test]
fn tasks_view__trust_this_folder_asks_first_and_trusts_the_folder_of_the_worktree() {
    let mut context = asked(TaskChoice::Trust, &[]);
    context.settings.trust = Trust::NotTrusted;
    let asked_first = question(run_task(&context, &Answers::new()));
    assert_eq!(asked_first.title.as_str(), "Trust this folder?");
    let detail = asked_first.detail.map(|detail| detail.to_string()).unwrap_or_default();
    assert!(
        detail.contains("is not trusted") && detail.contains(".vscode/tasks.json"),
        "{detail}"
    );
    assert!(!detail.contains("  "), "one space between its words: {detail}");
    let trusted = plan(run_task(&context, &Answers::new().with(TRUST, Answer::Confirmed)));
    assert_eq!(
        trusted.after.first(),
        Some(&FollowUp::TrustFolder(context.repository.folder.clone()))
    );
    context.settings.trust = Trust::Trusted;
    assert!(refused(run_task(&context, &Answers::new())).contains("trusted already"));
}

#[test]
fn tasks_stop__a_task_that_runs_is_stopped_and_one_that_does_not_is_refused() {
    let stopped = plan(stop_task(&for_task("build", &["build", "test"]), &Answers::new()));
    assert_eq!(stopped.steps, Vec::new());
    assert_eq!(stopped.after, [FollowUp::StopTasks(vec![reference("build")])]);
    assert_eq!(
        refused(stop_task(&for_task("build", &["test"]), &Answers::new())),
        "`build` is not running."
    );
}

#[test]
fn tasks_run_group__run_all_runs_each_task_in_the_order_of_the_group_and_shows_the_first() {
    let run = to_run(run_group(&for_group(&["test", "build"], &[]), &Answers::new()));
    assert_eq!(steps(&run), [["test"], ["build"]], "the first started first");
    assert_eq!(run.show, Some(reference("test")));
}

#[test]
fn tasks_run_group__a_task_already_running_is_not_run_again_nor_asked_about() {
    let run = to_run(run_group(&for_group(&["test", "build"], &["test"]), &Answers::new()));
    assert_eq!(steps(&run), [["build"]]);
    assert_eq!(run.restart, Vec::new());
    assert_eq!(run.show, Some(reference("test")));
}

#[test]
fn tasks_run_group__stop_all_stops_the_tasks_of_the_group_that_run() {
    let stop = Answers::new().with(STOP, Answer::Confirmed);
    let stopped = plan(run_group(
        &for_group(&["test", "build"], &["build", "test", "all"]),
        &stop,
    ));
    assert_eq!(
        stopped.after,
        [FollowUp::StopTasks(vec![reference("test"), reference("build")])]
    );
    assert_eq!(
        refused(run_group(&for_group(&["test", "build"], &["all"]), &stop)),
        "No task of `Servers` is running."
    );
}

#[test]
fn tasks_run_group__a_task_missing_from_the_worktree_is_left_out_and_said() {
    let outcome = run_group(&for_group(&["build", "gone", "test"], &[]), &Answers::new());
    let after = plan(outcome.clone()).after;
    assert_eq!(steps(&to_run(outcome)), [["build"], ["test"]]);
    let said: Vec<String> = after
        .iter()
        .filter_map(|follow_up| match follow_up {
            FollowUp::Say(text) => Some(text.to_string()),
            _ => None,
        })
        .collect();
    assert_eq!(said, ["`gone` is not in this worktree."]);
    assert_eq!(
        refused(run_group(&for_group(&["gone"], &[]), &Answers::new())),
        "`gone` is not in this worktree."
    );
}

#[test]
fn tasks_view__open_tasks_json_in_a_worktree_with_none_asks_then_makes_it_with_no_tasks_and_opens_it() {
    let context = asked(TaskChoice::MakeFile, &[]);
    let asked_first = question(run_task(&context, &Answers::new()));
    assert_eq!(asked_first.title.as_str(), "Make .vscode/tasks.json?");
    let made = plan(run_task(&context, &Answers::new().with(MAKE_FILE, Answer::Confirmed)));
    let [Step::Write { path, contents }] = made.steps.as_slice() else {
        panic!("expected one file written, got {:?}", made.steps);
    };
    assert_eq!(*path, context.repository.folder.join(".vscode/tasks.json"));
    let text = String::from_utf8(contents.clone()).unwrap();
    assert!(text.contains("\"tasks\": []"), "{text}");
    assert_eq!(made.after, [FollowUp::EditFile(String::from(".vscode/tasks.json"), 1)]);
}
