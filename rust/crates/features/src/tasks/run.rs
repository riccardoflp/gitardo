//! `tasks.run`, `tasks.stop` and `tasks.run-group`: what the Tasks tab is asked to run, each task after those it
//! depends on, and what it is asked to stop. The operations decide and say; the window starts the terminals and ends
//! the processes.

use model::contract::{
    AnswerKey, Answers, Choice, Context, Danger, FollowUp, Outcome, Plan, Question, QuestionKind, Refusal, Step,
    Target, Text, Undo,
};
use model::tasks::{TASKS_FILE, TaskChoice, TaskGroup, TaskReference, TaskTarget, TasksToRun, run_order};
use model::trust::Trust;

/// The key of what to do with a task that runs already.
pub const RUNNING: AnswerKey = AnswerKey("running");
/// The choice that stops the task and runs it again.
pub const RESTART: &str = "restart";
/// The choice that leaves the task running.
pub const LEAVE: &str = "leave";
/// The key of the confirmation that trusts the folder of the worktree shown.
pub const TRUST: AnswerKey = AnswerKey("trust-tasks");
/// The key of the confirmation that makes `.vscode/tasks.json`.
pub const MAKE_FILE: AnswerKey = AnswerKey("make-tasks-file");
/// A `tasks.json` with no tasks, as VS Code makes it.
pub const EMPTY_TASKS_FILE: &str = "{
    \"version\": \"2.0.0\",
    \"tasks\": []
}
";
/// The key that, answered yes, has `tasks.run-group` stop the tasks of the group that run in place of running them.
pub const STOP: AnswerKey = AnswerKey("stop");

/// What is said in a folder not trusted, where no task runs.
pub const NOT_TRUSTED: &str = "Tasks run commands of the repository: trust this folder to run them.";

/// Runs a task of the worktree shown in a terminal of its own in the Tasks tab, after the tasks of its `dependsOn`:
/// one after the other with `dependsOrder` `sequence`, all at once otherwise, each waiting for those it depends on to
/// end with exit code 0 or, for a background task, to be ready. A task that runs already asks **`<task>` is
/// running.** with **Restart**, which stops it and runs it again, and **Leave it running**. A dependency in a loop,
/// or a task named in `dependsOn` that the worktree does not have, runs nothing and says which; nothing runs in a
/// folder not trusted: **Trust this folder** of the Tasks tab asks **Trust this folder?** first, and trusts it. **Open
/// tasks.json** of the tab, in a worktree that has none, asks **Make .vscode/tasks.json?** and writes one with no
/// tasks, which the File page then opens.
///
/// Feature: tasks.run
#[must_use]
pub fn run_task(context: &Context, answers: &Answers) -> Outcome {
    let Some(target) = target_of(context) else {
        return refuse("Choose a task in the Tasks tab.");
    };
    let task = match &target.choice {
        TaskChoice::Task(task) => task,
        TaskChoice::Group(_) => return run_group(context, answers),
        TaskChoice::Trust => return trust_folder(context, answers),
        TaskChoice::MakeFile => return make_file(context, answers),
    };
    if context.settings.trust != Trust::Trusted {
        return refuse(NOT_TRUSTED);
    }
    let order = match run_order(&target.tasks, task) {
        Ok(order) => order,
        Err(refusal) => return refuse(&refusal.said()),
    };
    let mut run = TasksToRun {
        show: Some(task.clone()),
        ..TasksToRun::default()
    };
    if target.running.contains(task) {
        match answers.choice(RUNNING) {
            None => return Outcome::Ask(running_question(task)),
            Some(RESTART) => {
                run.restart = vec![task.clone()];
                run.orders = vec![order];
            }
            // **Leave it running**: the task is shown, and nothing runs.
            Some(_) => {}
        }
    } else {
        run.orders = vec![order];
    }
    no_step(format!("Run `{task}`"), vec![FollowUp::RunTasks(Box::new(run))])
}

/// Stops a task that runs in the Tasks tab, or every task of a group that does: the window sends each an interrupt,
/// as `Ctrl+C` does, and ends it with every process it started if it still runs three seconds later. A task that does
/// not run is refused, saying so.
///
/// Feature: tasks.stop
#[must_use]
pub fn stop_task(context: &Context, _answers: &Answers) -> Outcome {
    let Some(target) = target_of(context) else {
        return refuse("Choose a task in the Tasks tab.");
    };
    let (title, asked): (String, &[TaskReference]) = match &target.choice {
        TaskChoice::Task(task) => (format!("Stop `{task}`"), std::slice::from_ref(task)),
        TaskChoice::Group(group) => (format!("Stop the tasks of `{}`", group.name), group.tasks.as_slice()),
        TaskChoice::Trust | TaskChoice::MakeFile => return refuse("Choose a task in the Tasks tab."),
    };
    let running: Vec<TaskReference> = asked
        .iter()
        .filter(|task| target.running.contains(task))
        .cloned()
        .collect();
    if running.is_empty() {
        return refuse(&match &target.choice {
            TaskChoice::Group(group) => format!("No task of `{}` is running.", group.name),
            TaskChoice::Task(_) | TaskChoice::Trust | TaskChoice::MakeFile => {
                format!("`{}` is not running.", title_task(asked))
            }
        });
    }
    no_step(title, vec![FollowUp::StopTasks(running)])
}

/// Runs every task of a group in the order of the group, each as `tasks.run` runs it and started as soon as the one
/// before it started; a task that runs already is left running, and one the worktree does not have is left out and
/// said. With [`STOP`] answered yes, it stops those of them that run instead, as `tasks.stop` does. Nothing runs in a
/// folder not trusted: **Trust this folder** of the Tasks tab asks **Trust this folder?** first, and trusts it. **Open
/// tasks.json** of the tab, in a worktree that has none, asks **Make .vscode/tasks.json?** and writes one with no
/// tasks, which the File page then opens.
///
/// Feature: tasks.run-group
#[must_use]
pub fn run_group(context: &Context, answers: &Answers) -> Outcome {
    let Some(target) = target_of(context) else {
        return refuse("Choose a group in the Tasks tab.");
    };
    let TaskChoice::Group(group) = &target.choice else {
        return refuse("Choose a group in the Tasks tab.");
    };
    if answers.is_confirmed(STOP) {
        return stop_task(context, answers);
    }
    if context.settings.trust != Trust::Trusted {
        return refuse(NOT_TRUSTED);
    }
    let mut run = TasksToRun::default();
    let mut said = Vec::new();
    for task in &group.tasks {
        if target.tasks.find(task).is_none() {
            said.push(format!("`{task}` is not in this worktree."));
            continue;
        }
        run.show = run.show.or_else(|| Some(task.clone()));
        if target.running.contains(task) {
            continue;
        }
        match run_order(&target.tasks, task) {
            Ok(order) => run.orders.push(order),
            Err(refusal) => said.push(refusal.said()),
        }
    }
    if run.show.is_none() {
        return refuse(&nothing_to_run(group, &said));
    }
    let mut after = vec![FollowUp::RunTasks(Box::new(run))];
    if !said.is_empty() {
        after.push(FollowUp::Say(Text::from(said.join(" "))));
    }
    no_step(format!("Run the tasks of `{}`", group.name), after)
}

/// What is said of a group none of whose tasks can run.
fn nothing_to_run(group: &TaskGroup, said: &[String]) -> String {
    if said.is_empty() {
        format!("The group `{}` has no tasks.", group.name)
    } else {
        said.join(" ")
    }
}

/// What the operation was invoked on, when it is a task or a group.
fn target_of(context: &Context) -> Option<&TaskTarget> {
    match &context.target {
        Target::Task(target) => Some(target),
        // Every other target is no task and no group of them.
        _ => None,
    }
}

/// **`<task>` is running.**, with **Restart** and **Leave it running**.
fn running_question(task: &TaskReference) -> Question {
    let choice = |identifier: &str, label: &str, description: &str| Choice {
        identifier: identifier.to_owned(),
        label: Text::from(label),
        description: Some(Text::from(description)),
    };
    Question {
        key: RUNNING,
        kind: QuestionKind::Pick {
            choices: vec![
                choice(RESTART, "Restart", "Stop it, then run it again."),
                choice(LEAVE, "Leave it running", "Show its terminal."),
            ],
        },
        title: Text::from(format!("`{task}` is running.")),
        detail: None,
        default: None,
    }
}

/// The name of the only task asked about.
fn title_task(asked: &[TaskReference]) -> String {
    asked.first().map(ToString::to_string).unwrap_or_default()
}

/// **Trust this folder** of the Tasks tab: asks **Trust this folder?**, naming the folder and what its tasks run,
/// and trusts it from then on (`trust.is-trusted`); in a folder trusted already it says so.
fn trust_folder(context: &Context, answers: &Answers) -> Outcome {
    let folder = &context.repository.folder;
    if context.settings.trust == Trust::Trusted {
        return refuse(&format!("`{}` is trusted already.", folder.display()));
    }
    if !answers.is_confirmed(TRUST) {
        return Outcome::Ask(Question {
            key: TRUST,
            kind: QuestionKind::Confirm {
                accept: Text::from("Trust this folder"),
            },
            title: Text::from("Trust this folder?"),
            detail: Some(Text::from(format!(
                "`{}` is not trusted, and its tasks run the commands of its `.vscode/tasks.json` and of its \
                 `package.json` files. Trust it only if you trust who wrote them: what it and the folders in it ask \
                 to run is run from now on.",
                folder.display()
            ))),
            default: None,
        });
    }
    no_step(
        format!("Trust `{}`", folder.display()),
        vec![
            FollowUp::TrustFolder(folder.clone()),
            FollowUp::Say(Text::from(format!(
                "`{}` is trusted: its tasks can run.",
                folder.display()
            ))),
        ],
    )
}

/// **Open tasks.json** in a worktree that has none: asks **Make .vscode/tasks.json?**, writes one with no tasks and
/// has the File page open it.
fn make_file(context: &Context, answers: &Answers) -> Outcome {
    if !answers.is_confirmed(MAKE_FILE) {
        return Outcome::Ask(Question {
            key: MAKE_FILE,
            kind: QuestionKind::Confirm {
                accept: Text::from("Make the file"),
            },
            title: Text::from("Make .vscode/tasks.json?"),
            detail: Some(Text::from(
                "This worktree has none yet: it is made with no tasks, for you to add them.",
            )),
            default: None,
        });
    }
    let path = context.repository.folder.join(TASKS_FILE);
    Outcome::Run(Plan {
        title: Text::from("Make .vscode/tasks.json"),
        steps: vec![Step::Write {
            path,
            contents: EMPTY_TASKS_FILE.as_bytes().to_vec(),
        }],
        danger: Danger::None,
        undo: Undo::Impossible(Text::from("The file is yours: delete it to take it back.")),
        after: vec![FollowUp::EditFile(TASKS_FILE.to_owned(), 1)],
        on_failure: Vec::new(),
    })
}

/// A refusal that offers nothing else.
fn refuse(said: &str) -> Outcome {
    Outcome::Refuse(Refusal::saying(said))
}

/// A plan with no step: what runs and stops is the window's, told by what follows the plan.
fn no_step(title: String, after: Vec<FollowUp>) -> Outcome {
    Outcome::Run(Plan {
        title: Text::from(title),
        steps: Vec::new(),
        danger: Danger::None,
        undo: Undo::Impossible(Text::from(
            "A task run or stopped is not taken back by Undo: what it did is the task's own.",
        )),
        after,
        on_failure: Vec::new(),
    })
}

#[cfg(test)]
#[path = "run_tests.rs"]
mod tests;
