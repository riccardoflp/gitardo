#![allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]

use super::super::testing::{in_sequence, task, tasks, workspace};
use super::{OrderRefusal, RunOrder, run_order};
use crate::tasks::{NotRunnable, TaskKind, TaskReference, Tasks};

/// The steps of an order as `task < those it waits for`.
fn steps(order: &RunOrder) -> Vec<String> {
    order
        .steps
        .iter()
        .map(|step| {
            let waits: Vec<String> = step.waits_for.iter().map(ToString::to_string).collect();
            format!("{} < {}", step.task, waits.join(" "))
        })
        .collect()
}

fn order(tasks: &Tasks, name: &str) -> Vec<String> {
    steps(&run_order(tasks, &workspace(name)).unwrap())
}

#[test]
fn tasks_run__a_task_alone_runs_alone() {
    let tasks = tasks(vec![task("build", &[])]);
    assert_eq!(order(&tasks, "build"), ["build < "]);
}

#[test]
fn tasks_run__in_sequence_the_second_waits_for_the_first_and_the_task_for_both() {
    let tasks = tasks(vec![
        in_sequence("all", &["first", "second"]),
        task("first", &[]),
        task("second", &[]),
    ]);
    assert_eq!(
        order(&tasks, "all"),
        ["first < ", "second < first", "all < first second"]
    );
}

#[test]
fn tasks_run__in_parallel_those_it_depends_on_wait_for_nothing() {
    let tasks = tasks(vec![
        task("all", &["first", "second"]),
        task("first", &[]),
        task("second", &[]),
    ]);
    assert_eq!(order(&tasks, "all"), ["first < ", "second < ", "all < first second"]);
}

#[test]
fn tasks_run__in_sequence_what_the_second_depends_on_waits_for_the_first_too() {
    let tasks = tasks(vec![
        in_sequence("all", &["install", "check"]),
        task("install", &[]),
        task("check", &["lint", "test"]),
        task("lint", &[]),
        task("test", &[]),
    ]);
    assert_eq!(
        order(&tasks, "all"),
        [
            "install < ",
            "lint < install",
            "test < install",
            "check < install lint test",
            "all < install check"
        ]
    );
}

#[test]
fn tasks_run__a_task_two_others_depend_on_runs_once_and_waits_for_nothing_placed_after_it() {
    let tasks = tasks(vec![
        in_sequence("all", &["api", "web"]),
        task("api", &["install"]),
        task("web", &["install"]),
        task("install", &[]),
    ]);
    assert_eq!(
        order(&tasks, "all"),
        ["install < ", "api < install", "web < api install", "all < api web"]
    );
}

#[test]
fn tasks_run__two_tasks_that_depend_on_each_other_run_nothing_and_the_loop_is_said() {
    let tasks = tasks(vec![task("a", &["b"]), task("b", &["a"])]);
    let refusal = run_order(&tasks, &workspace("a")).unwrap_err();
    assert_eq!(
        refusal,
        OrderRefusal::Loop(vec![workspace("a"), workspace("b"), workspace("a")])
    );
    assert_eq!(
        refusal.said(),
        "Nothing ran: the tasks depend on each other in a loop, `a` → `b` → `a`."
    );
    let itself = Tasks {
        workspace: vec![task("watch", &["watch"])],
        ..Tasks::default()
    };
    let refusal = run_order(&itself, &workspace("watch")).unwrap_err();
    assert_eq!(
        refusal,
        OrderRefusal::Loop(vec![workspace("watch"), workspace("watch")])
    );
}

#[test]
fn tasks_run__a_loop_further_down_names_only_its_own_tasks() {
    let tasks = tasks(vec![
        task("all", &["a"]),
        task("a", &["b"]),
        task("b", &["c"]),
        task("c", &["a"]),
    ]);
    let refusal = run_order(&tasks, &workspace("all")).unwrap_err();
    let named: Vec<TaskReference> = ["a", "b", "c", "a"].iter().map(|name| workspace(name)).collect();
    assert_eq!(refusal, OrderRefusal::Loop(named));
}

#[test]
fn tasks_run__a_task_named_in_depends_on_that_the_worktree_lacks_runs_nothing_and_is_said() {
    let tasks = tasks(vec![task("all", &["build", "gone"]), task("build", &[])]);
    let refusal = run_order(&tasks, &workspace("all")).unwrap_err();
    assert_eq!(
        refusal,
        OrderRefusal::Missing {
            task: workspace("all"),
            named: workspace("gone")
        }
    );
    assert_eq!(
        refusal.said(),
        "Nothing ran: `all` depends on `gone`, which is not in this worktree."
    );
    assert_eq!(
        run_order(&tasks, &workspace("typo")).unwrap_err().said(),
        "Nothing ran: `typo` is not in this worktree."
    );
}

#[test]
fn tasks_run__a_task_that_depends_on_one_gitardo_does_not_run_runs_nothing() {
    let mut cargo = task("cargo: build", &[]);
    cargo.kind = TaskKind::Other(String::from("cargo"));
    let tasks = tasks(vec![task("all", &["cargo: build"]), cargo]);
    let refusal = run_order(&tasks, &workspace("all")).unwrap_err();
    assert_eq!(
        refusal,
        OrderRefusal::NotRunnable(workspace("cargo: build"), NotRunnable::OtherKind(String::from("cargo")))
    );
    assert_eq!(
        refusal.said(),
        "Nothing ran: `cargo: build` cannot be run here. This kind of task runs in VS Code only."
    );
}

#[test]
fn tasks_run__a_script_of_a_package_runs_before_the_task_that_depends_on_it() {
    let mut all = task("all", &[]);
    all.depends_on = vec![TaskReference::npm("extension", "build")];
    let mut script = task("build", &[]);
    script.reference = TaskReference::npm("extension", "build");
    let tasks = Tasks {
        workspace: vec![all],
        packages: vec![crate::tasks::PackageScripts {
            folder: String::from("extension"),
            name: String::from("extension"),
            manager: crate::tasks::PackageManager::Npm,
            scripts: vec![script],
        }],
        problems: Vec::new(),
    };
    assert_eq!(
        order(&tasks, "all"),
        ["npm: build - extension < ", "all < npm: build - extension"]
    );
}
