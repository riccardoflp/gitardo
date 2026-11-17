#![allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]

use std::path::PathBuf;

use model::contract::{Answer, Answers, Context, Danger, Outcome, Plan, QuestionKind, Step, Target, Undo, Validation};
use model::state::RepositoryState;
use model::tasks::{GroupsAction, GroupsTarget, Shift, TaskGroup, TaskReference};

use super::{DELETE, NAME, edit_groups};
use crate::tasks::groups_file::parse_groups;
use crate::tasks::testing::Disk;
use crate::testing::{context, plan, question};

/// The file of the groups of the repository of these tests.
const FILE: &str = "/work/app/.git/gitardo/task-groups.json";

/// The main worktree of a repository in `/work/app`.
fn main_worktree() -> RepositoryState {
    RepositoryState {
        folder: PathBuf::from("/work/app"),
        git_folder: PathBuf::from("/work/app/.git"),
        ..crate::testing::state()
    }
}

/// A linked worktree of the same repository.
fn linked_worktree() -> RepositoryState {
    RepositoryState {
        folder: PathBuf::from("/work/app.worktrees/feature"),
        git_folder: PathBuf::from("/work/app/.git/worktrees/feature"),
        ..crate::testing::state()
    }
}

/// A disk with the `commondir` of the linked worktree, as git writes it.
fn disk() -> Disk {
    Disk::default().with("/work/app/.git/worktrees/feature/commondir", "../..\n")
}

fn on(state: RepositoryState, action: GroupsAction) -> Context {
    context(state, Target::TaskGroups(Box::new(GroupsTarget::Edit(action))))
}

fn named(name: &str) -> Answers {
    Answers::new().with(NAME, Answer::Text(name.to_owned()))
}

/// Does a change in the main worktree, and gives the title of its plan.
fn done(disk: &mut Disk, action: GroupsAction, answers: Answers) -> String {
    let plan = plan(disk.asked(edit_groups, &on(main_worktree(), action), answers));
    disk.run(&plan);
    plan.title.to_string()
}

fn refused(disk: &Disk, state: RepositoryState, action: GroupsAction, answers: Answers) -> String {
    match disk.asked(edit_groups, &on(state, action), answers) {
        Outcome::Refuse(refusal) => refusal.message.to_string(),
        other => panic!("expected a refusal, got {other:?}"),
    }
}

/// The groups on the disk, each as `name: task, task`.
fn kept(disk: &Disk) -> Vec<String> {
    let groups = parse_groups(disk.text(FILE).as_deref()).unwrap();
    let shown = |group: &TaskGroup| {
        let tasks: Vec<String> = group.tasks.iter().map(ToString::to_string).collect();
        format!("{}: {}", group.name, tasks.join(", "))
    };
    groups.iter().map(shown).collect()
}

/// The files a plan writes.
fn written(plan: &Plan) -> Vec<PathBuf> {
    let path = |step: &Step| match step {
        Step::Write { path, .. } => path.clone(),
        other => panic!("a step that is no write: {other:?}"),
    };
    plan.steps.iter().map(path).collect()
}

fn new_group() -> GroupsAction {
    GroupsAction::New { task: None }
}

fn put(group: &str, task: TaskReference, before: Option<TaskReference>) -> GroupsAction {
    GroupsAction::PlaceTask {
        group: group.to_owned(),
        task,
        before,
        from: None,
    }
}

fn workspace(name: &str) -> TaskReference {
    TaskReference::workspace(name)
}

#[test]
fn tasks_edit_groups__new_group_named_servers_lists_an_empty_group_under_groups() {
    let mut disk = disk();
    let asked = question(disk.asked(edit_groups, &on(main_worktree(), new_group()), Answers::new()));
    assert_eq!((asked.key, asked.title.as_str()), (NAME, "New group"));
    let QuestionKind::Text { validation, .. } = &asked.kind else {
        panic!("a name is typed: {:?}", asked.kind);
    };
    assert_eq!(
        validation.problem("  ").map(|problem| problem.to_string()).as_deref(),
        Some("Enter a name for the group.")
    );
    assert_eq!(validation.problem("Servers"), None);
    assert!(matches!(validation, Validation::Required(_)));
    let plan = plan(disk.asked(edit_groups, &on(main_worktree(), new_group()), named(" Servers ")));
    assert_eq!(plan.title.as_str(), "Make the group `Servers`");
    assert_eq!(plan.danger, Danger::None);
    assert!(matches!(plan.undo, Undo::Impossible(_)));
    assert_eq!(written(&plan), [PathBuf::from(FILE)]);
    disk.run(&plan);
    assert_eq!(kept(&disk), ["Servers: "]);
    assert_eq!(done(&mut disk, new_group(), named("Checks")), "Make the group `Checks`");
    assert_eq!(kept(&disk), ["Servers: ", "Checks: "], "a new group is last");
}

#[test]
fn tasks_edit_groups__a_task_put_onto_a_group_is_listed_in_it_and_above_another_it_goes_before_it() {
    let mut disk = disk();
    let _made = done(&mut disk, new_group(), named("Servers"));
    let docs = workspace("serve the docs");
    let watch = TaskReference::npm("extension", "watch");
    assert_eq!(
        done(&mut disk, put("Servers", docs.clone(), None), Answers::new()),
        "Add `serve the docs` to the group `Servers`"
    );
    assert_eq!(
        done(
            &mut disk,
            put("Servers", watch.clone(), Some(docs.clone())),
            Answers::new()
        ),
        "Add `npm: watch - extension` to the group `Servers`"
    );
    assert_eq!(kept(&disk), ["Servers: npm: watch - extension, serve the docs"]);
    assert_eq!(
        done(
            &mut disk,
            put("Servers", docs.clone(), Some(watch.clone())),
            Answers::new()
        ),
        "Move `serve the docs` in the group `Servers`"
    );
    assert_eq!(kept(&disk), ["Servers: serve the docs, npm: watch - extension"]);
    assert_eq!(
        refused(&disk, main_worktree(), put("Servers", docs, None), Answers::new()),
        "`serve the docs` is already in the group `Servers`."
    );
    let made_for = GroupsAction::New {
        task: Some(watch.clone()),
    };
    let _made = done(&mut disk, made_for, named("Watchers"));
    assert_eq!(
        kept(&disk).last().map(String::as_str),
        Some("Watchers: npm: watch - extension"),
        "a task can be in several groups"
    );
}

#[test]
fn tasks_edit_groups__the_groups_are_the_same_in_another_worktree_of_the_repository() {
    let mut disk = disk();
    let from_linked = on(linked_worktree(), new_group());
    let plan = plan(disk.asked(edit_groups, &from_linked, named("Servers")));
    assert_eq!(
        written(&plan),
        [PathBuf::from(FILE)],
        "the file of the common git folder, not one of the linked worktree"
    );
    disk.run(&plan);
    assert_eq!(
        refused(&disk, main_worktree(), new_group(), named("Servers")),
        "There is already a group named `Servers`.",
        "the main worktree has the group made in the linked one"
    );
    let _added = done(&mut disk, put("Servers", workspace("build"), None), Answers::new());
    let shift = GroupsAction::ShiftTask {
        group: String::from("Servers"),
        task: workspace("build"),
        shift: Shift::Up,
    };
    assert_eq!(
        refused(&disk, linked_worktree(), shift, Answers::new()),
        "The groups are already so.",
        "the linked worktree has the task added in the main one, first already"
    );
}

#[test]
fn tasks_edit_groups__renaming_a_group_to_the_name_of_another_is_refused_saying_so() {
    let mut disk = disk();
    let _first = done(&mut disk, new_group(), named("Servers"));
    let _second = done(&mut disk, new_group(), named("Checks"));
    let rename = || GroupsAction::Rename {
        group: String::from("Checks"),
    };
    let asked = question(disk.asked(edit_groups, &on(main_worktree(), rename()), Answers::new()));
    assert_eq!(asked.title.as_str(), "Rename the group `Checks`");
    assert_eq!(
        asked.default,
        Some(Answer::Text(String::from("Checks"))),
        "it starts from its name"
    );
    assert_eq!(
        refused(&disk, main_worktree(), rename(), named("Servers")),
        "There is already a group named `Servers`."
    );
    assert_eq!(
        refused(&disk, main_worktree(), rename(), named("Checks")),
        "The groups are already so."
    );
    assert_eq!(kept(&disk), ["Servers: ", "Checks: "]);
    assert_eq!(
        done(&mut disk, rename(), named("Tests")),
        "Rename the group `Checks` to `Tests`"
    );
    assert_eq!(kept(&disk), ["Servers: ", "Tests: "]);
}

#[test]
fn tasks_edit_groups__delete_asks_first_and_the_others_stay() {
    let mut disk = disk();
    let _first = done(&mut disk, new_group(), named("Servers"));
    let _second = done(&mut disk, new_group(), named("Checks"));
    let delete = || GroupsAction::Delete {
        group: String::from("Servers"),
    };
    let asked = question(disk.asked(edit_groups, &on(main_worktree(), delete()), Answers::new()));
    assert_eq!(
        (asked.key, asked.title.as_str()),
        (DELETE, "Delete the group `Servers`?")
    );
    assert_eq!(
        asked.detail.as_ref().map(ToString::to_string).as_deref(),
        Some("Its tasks stay where they are.")
    );
    let QuestionKind::Confirm { accept } = &asked.kind else {
        panic!("a confirmation: {:?}", asked.kind);
    };
    assert_eq!(accept.as_str(), "Delete");
    assert_eq!(kept(&disk).len(), 2, "nothing is written before the answer");
    let confirmed = Answers::new().with(DELETE, Answer::Confirmed);
    assert_eq!(
        done(&mut disk, delete(), confirmed.clone()),
        "Delete the group `Servers`"
    );
    assert_eq!(kept(&disk), ["Checks: "]);
    assert_eq!(
        refused(&disk, main_worktree(), delete(), confirmed),
        "There is no group named `Servers` any more."
    );
}

#[test]
fn tasks_edit_groups__a_group_another_window_deleted_is_not_changed_and_nothing_is_asked() {
    let mut disk = disk();
    let _made = done(&mut disk, new_group(), named("Checks"));
    let gone = "There is no group named `Servers` any more.";
    let group = || String::from("Servers");
    let actions = [
        GroupsAction::Rename { group: group() },
        GroupsAction::Delete { group: group() },
        put("Servers", workspace("build"), None),
        GroupsAction::ShiftGroup {
            group: group(),
            shift: Shift::Down,
        },
        GroupsAction::PlaceGroup {
            group: String::from("Checks"),
            before: Some(group()),
        },
    ];
    for action in actions {
        assert_eq!(
            refused(&disk, main_worktree(), action.clone(), Answers::new()),
            gone,
            "{action:?}"
        );
    }
    let remove = GroupsAction::RemoveTask {
        group: String::from("Checks"),
        task: workspace("build"),
    };
    assert_eq!(
        refused(&disk, main_worktree(), remove, Answers::new()),
        "The group `Checks` does not have `build`."
    );
}

#[test]
fn tasks_edit_groups__a_group_and_a_task_move_and_a_task_is_taken_out() {
    let mut disk = disk();
    for name in ["Servers", "Checks", "Docs"] {
        let _made = done(&mut disk, new_group(), named(name));
    }
    for name in ["lint", "test"] {
        let _added = done(&mut disk, put("Checks", workspace(name), None), Answers::new());
    }
    let up = GroupsAction::ShiftGroup {
        group: String::from("Docs"),
        shift: Shift::Up,
    };
    assert_eq!(done(&mut disk, up, Answers::new()), "Move the group `Docs`");
    let first = GroupsAction::PlaceGroup {
        group: String::from("Checks"),
        before: Some(String::from("Servers")),
    };
    assert_eq!(done(&mut disk, first, Answers::new()), "Move the group `Checks`");
    assert_eq!(kept(&disk), ["Checks: lint, test", "Servers: ", "Docs: "]);
    let down = GroupsAction::ShiftTask {
        group: String::from("Checks"),
        task: workspace("lint"),
        shift: Shift::Down,
    };
    assert_eq!(
        done(&mut disk, down, Answers::new()),
        "Move `lint` in the group `Checks`"
    );
    let dragged = GroupsAction::PlaceTask {
        group: String::from("Docs"),
        task: workspace("test"),
        before: None,
        from: Some(String::from("Checks")),
    };
    assert_eq!(
        done(&mut disk, dragged, Answers::new()),
        "Move `test` from the group `Checks` to `Docs`"
    );
    assert_eq!(kept(&disk), ["Checks: lint", "Servers: ", "Docs: test"]);
    let remove = GroupsAction::RemoveTask {
        group: String::from("Checks"),
        task: workspace("lint"),
    };
    assert_eq!(
        done(&mut disk, remove, Answers::new()),
        "Remove `lint` from the group `Checks`"
    );
    assert_eq!(kept(&disk), ["Checks: ", "Servers: ", "Docs: test"]);
}

#[test]
fn tasks_edit_groups__a_file_that_cannot_be_read_is_not_written_over() {
    let disk = disk().with(FILE, "{ \"version\": 7, \"groups\": [] }");
    assert_eq!(
        refused(&disk, main_worktree(), new_group(), named("Servers")),
        "`gitardo/task-groups.json` could not be read: a newer Gitardo wrote it, with version 7. Nothing was changed."
    );
    let disk = disk.with(FILE, "not json");
    let said = refused(&disk, linked_worktree(), new_group(), named("Servers"));
    assert!(
        said.starts_with("`gitardo/task-groups.json` could not be read: at line 1, column 1, "),
        "{said}"
    );
    assert!(said.ends_with(". Nothing was changed."), "{said}");
}

#[test]
fn tasks_edit_groups__outside_a_repository_and_on_anything_but_the_groups_nothing_is_changed() {
    let disk = disk();
    assert_eq!(
        refused(&disk, RepositoryState::default(), new_group(), named("Servers")),
        "There are no groups of tasks outside a repository."
    );
    let on_a_branch = context(main_worktree(), Target::Branch(String::from("main")));
    let Outcome::Refuse(refusal) = disk.asked(edit_groups, &on_a_branch, Answers::new()) else {
        panic!("a refusal");
    };
    assert_eq!(refusal.message.as_str(), "Choose a group or a task in the Tasks tab.");
}
