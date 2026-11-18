#![allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]

use std::path::PathBuf;

use model::contract::{Answer, Answers, Context, FollowUp, Outcome, Plan, Question, QuestionKind, Target};
use model::tasks::{GroupsAction, GroupsTarget, PackageManager, PackageScripts, Task, TaskReference, Tasks};

use super::{CHOSEN, CONFLICTS, FILE, import_groups};
use crate::tasks::testing::{Disk, disk_with, group, kept, main_worktree};
use crate::testing::{context, plan, question};

/// Where the file to import is in these tests.
const EXPORTED: &str = "/downloads/task-groups.json";

/// A file as **Export Groups...** of Run My Tasks 0.8.0 writes it: `JSON.stringify` with two spaces, each group with
/// its tasks by the names VS Code gives them and its launch configurations.
const OF_RUN_MY_TASKS: &str = r#"{
  "version": 1,
  "groups": [
    {
      "name": "Servers",
      "tasks": [
        "serve the docs",
        "watch - extension"
      ],
      "launches": []
    },
    {
      "name": "Checks",
      "tasks": [
        "lint",
        "test"
      ],
      "launches": [
        "Debug the tests"
      ]
    }
  ]
}
"#;

/// The tasks of the worktree shown: `serve the docs` and `lint` in `tasks.json`, `watch` in `extension/package.json`.
fn tasks() -> Tasks {
    let task = |reference: TaskReference| Task::named(reference, PathBuf::from("/work/app"));
    Tasks {
        workspace: vec![
            task(TaskReference::workspace("serve the docs")),
            task(TaskReference::workspace("lint")),
        ],
        packages: vec![PackageScripts {
            folder: String::from("extension"),
            name: String::from("extension"),
            manager: PackageManager::Npm,
            scripts: vec![task(TaskReference::npm("extension", "watch"))],
        }],
        problems: Vec::new(),
    }
}

fn import() -> Context {
    context(
        main_worktree(),
        Target::TaskGroups(Box::new(GroupsTarget::Import(tasks()))),
    )
}

/// The answers once the file is chosen, with the groups of these names chosen when there are any.
fn chosen(names: &[&str]) -> Answers {
    let answers = Answers::new().with(FILE, Answer::Path(PathBuf::from(EXPORTED)));
    if names.is_empty() {
        return answers;
    }
    answers.with(
        CHOSEN,
        Answer::Choices(names.iter().map(|name| (*name).to_owned()).collect()),
    )
}

fn asked(disk: &Disk, answers: Answers) -> Question {
    question(disk.asked(import_groups, &import(), answers))
}

fn planned(disk: &Disk, answers: Answers) -> Plan {
    plan(disk.asked(import_groups, &import(), answers))
}

fn refused(disk: &Disk, answers: Answers) -> String {
    match disk.asked(import_groups, &import(), answers) {
        Outcome::Refuse(refusal) => refusal.message.to_string(),
        other => panic!("expected a refusal, got {other:?}"),
    }
}

/// What a plan says once it ran.
fn said(plan: &Plan) -> String {
    match plan.after.as_slice() {
        [FollowUp::Say(text)] => text.to_string(),
        other => panic!("one thing said: {other:?}"),
    }
}

/// The identifiers of the choices of a question.
fn offered(question: &Question) -> Vec<&str> {
    let (QuestionKind::Pick { choices } | QuestionKind::PickMany { choices }) = &question.kind else {
        panic!("choices: {:?}", question.kind);
    };
    choices.iter().map(|choice| choice.identifier.as_str()).collect()
}

#[test]
fn tasks_import_groups__a_file_exported_by_run_my_tasks_with_two_groups_imports_both_with_their_tasks_in_order() {
    let mut disk = Disk::default().with(EXPORTED, OF_RUN_MY_TASKS);
    let file = asked(&disk, Answers::new());
    assert_eq!((file.key, file.title.as_str()), (FILE, "Import groups"));
    let QuestionKind::OpenFile(dialog) = &file.kind else {
        panic!("the system's dialog: {:?}", file.kind);
    };
    assert_eq!(dialog.folder, PathBuf::from("/work/app"));
    assert_eq!(
        dialog.filters.first().map(|filter| filter.extensions.clone()),
        Some(vec![String::from("json")])
    );
    let which = asked(&disk, chosen(&[]));
    assert_eq!((which.key, which.title.as_str()), (CHOSEN, "Import groups"));
    assert!(matches!(which.kind, QuestionKind::PickMany { .. }));
    assert_eq!(offered(&which), ["Servers", "Checks"]);
    let all = Answer::Choices(vec![String::from("Servers"), String::from("Checks")]);
    assert_eq!(which.default, Some(all), "all chosen");
    let plan = planned(&disk, chosen(&["Servers", "Checks"]));
    assert_eq!(plan.title.as_str(), "Import 2 groups");
    disk.run(&plan);
    assert_eq!(
        kept(&disk),
        ["Servers: serve the docs, npm: watch - extension", "Checks: lint, test"]
    );
    assert_eq!(
        said(&plan),
        "2 groups were imported. This worktree does not have `test`. 1 launch configuration was left out, since \
         Gitardo has no debugger."
    );
}

#[test]
fn tasks_import_groups__only_the_groups_chosen_are_imported_after_those_of_the_repository() {
    let mut disk = disk_with(&[group("Docs", &["serve the docs"])]).with(EXPORTED, OF_RUN_MY_TASKS);
    let plan = planned(&disk, chosen(&["Servers"]));
    assert_eq!(plan.title.as_str(), "Import 1 group");
    disk.run(&plan);
    assert_eq!(
        kept(&disk),
        [
            "Docs: serve the docs",
            "Servers: serve the docs, npm: watch - extension"
        ]
    );
    assert_eq!(said(&plan), "1 group was imported.");
    let none = chosen(&[]).with(CHOSEN, Answer::Choices(Vec::new()));
    assert_eq!(refused(&disk, none), "No group was chosen: nothing was imported.");
}

#[test]
fn tasks_import_groups__merge_on_a_group_named_as_one_of_the_repository_adds_only_the_tasks_it_lacks() {
    let ours = [group("Checks", &["test", "format"]), group("Docs", &[])];
    let mut disk = disk_with(&ours).with(EXPORTED, OF_RUN_MY_TASKS);
    let conflict = asked(&disk, chosen(&["Checks"]));
    assert_eq!(conflict.key, CONFLICTS[0]);
    assert_eq!(conflict.title.as_str(), "`Checks` is already a group.");
    assert_eq!(conflict.detail, None);
    assert!(matches!(conflict.kind, QuestionKind::Pick { .. }));
    assert_eq!(
        offered(&conflict),
        ["merge", "replace", "skip"],
        "no others to do the same for"
    );
    let answered = |answer: &str| chosen(&["Checks"]).with(CONFLICTS[0], Answer::Choice(answer.to_owned()));
    let merged = planned(&disk, answered("merge"));
    assert_eq!(
        refused(&disk, answered("skip")),
        "Every group chosen was skipped: nothing was imported."
    );
    let replaced = planned(&disk, answered("replace"));
    disk.run(&merged);
    assert_eq!(
        kept(&disk),
        ["Checks: test, format, lint", "Docs: "],
        "after its own, and `test` once"
    );
    assert_eq!(
        said(&merged),
        "1 group was imported. This worktree does not have `test`. 1 launch configuration was left out, since \
         Gitardo has no debugger."
    );
    disk.run(&replaced);
    assert_eq!(
        kept(&disk),
        ["Checks: lint, test", "Docs: "],
        "the tasks of the file, where the group was"
    );
}

#[test]
fn tasks_import_groups__do_the_same_for_the_others_is_offered_when_more_groups_are_named_as_ours() {
    let ours = [group("Servers", &["lint"]), group("Checks", &["format"])];
    let mut disk = disk_with(&ours).with(EXPORTED, OF_RUN_MY_TASKS);
    let both = || chosen(&["Servers", "Checks"]);
    let first = asked(&disk, both());
    assert_eq!(first.title.as_str(), "`Servers` is already a group.");
    assert_eq!(
        first.detail.as_ref().map(ToString::to_string).as_deref(),
        Some("1 more group of the file is named as a group of the repository.")
    );
    assert_eq!(
        offered(&first),
        ["merge", "replace", "skip", "merge all", "replace all", "skip all"]
    );
    let one = both().with(CONFLICTS[0], Answer::Choice(String::from("skip")));
    let second = asked(&disk, one.clone());
    assert_eq!(
        (second.key, second.title.as_str()),
        (CONFLICTS[1], "`Checks` is already a group.")
    );
    assert_eq!(offered(&second), ["merge", "replace", "skip"]);
    let each = planned(&disk, one.with(CONFLICTS[1], Answer::Choice(String::from("merge"))));
    assert_eq!(each.title.as_str(), "Import 1 group");
    let all = planned(
        &disk,
        both().with(CONFLICTS[0], Answer::Choice(String::from("replace all"))),
    );
    assert_eq!(all.title.as_str(), "Import 2 groups", "the second is not asked about");
    disk.run(&each);
    assert_eq!(kept(&disk), ["Servers: lint", "Checks: format, lint, test"]);
    disk.run(&all);
    assert_eq!(
        kept(&disk),
        ["Servers: serve the docs, npm: watch - extension", "Checks: lint, test"]
    );
}

#[test]
fn tasks_import_groups__a_group_with_a_launch_configuration_is_imported_without_it_and_one_is_said_left_out() {
    let file = r#"{ "version": 1, "groups": [
        { "name": "Debugging", "tasks": ["lint"], "launches": ["Run the extension", "Attach"] },
        { "name": "Old", "tasks": ["lint"] }
    ] }"#;
    let mut disk = Disk::default().with(EXPORTED, file);
    let which = asked(&disk, chosen(&[]));
    let QuestionKind::PickMany { choices } = &which.kind else {
        panic!("choices: {:?}", which.kind);
    };
    let described: Vec<String> = choices
        .iter()
        .map(|choice| choice.description.as_ref().unwrap().to_string())
        .collect();
    assert_eq!(described, ["1 task, and 2 launch configurations left out", "1 task"]);
    let plan = planned(&disk, chosen(&["Debugging", "Old"]));
    disk.run(&plan);
    assert_eq!(kept(&disk), ["Debugging: lint", "Old: lint"]);
    assert_eq!(
        said(&plan),
        "2 groups were imported. 2 launch configurations were left out, since Gitardo has no debugger."
    );
    let one = r#"{ "version": 1, "groups": [{ "name": "Debugging", "tasks": [], "launches": ["Attach"] }] }"#;
    let disk = Disk::default().with(EXPORTED, one);
    assert_eq!(
        said(&planned(&disk, chosen(&["Debugging"]))),
        "1 group was imported. 1 launch configuration was left out, since Gitardo has no debugger."
    );
}

#[test]
fn tasks_import_groups__a_json_file_of_another_kind_imports_nothing_and_says_it_is_not_a_file_of_groups() {
    let not_groups = "`task-groups.json` is not a file of groups: nothing was imported.";
    for file in [
        "{ \"name\": \"app\", \"scripts\": { \"build\": \"node build.mjs\" } }",
        "[]",
        "not json",
        "",
        "{ \"groups\": [] }",
        "{ \"version\": \"1\", \"groups\": [] }",
        "{ \"version\": 1, \"groups\": [{ \"name\": \"\", \"tasks\": [] }] }",
        "{ \"version\": 1, \"groups\": [{ \"name\": \"Servers\" }] }",
        "{ \"version\": 1, \"groups\": [{ \"name\": \"Servers\", \"tasks\": [1] }] }",
        "{ \"version\": 1, \"groups\": [{ \"name\": \"Servers\", \"tasks\": [], \"launches\": \"Attach\" }] }",
        "{ \"version\": 1, \"groups\": [{ \"name\": \"A\", \"tasks\": [] }, { \"name\": \" A \", \"tasks\": [] }] }",
    ] {
        let disk = disk_with(&[group("Docs", &[])]).with(EXPORTED, file);
        assert_eq!(refused(&disk, chosen(&[])), not_groups, "{file}");
        assert_eq!(kept(&disk), ["Docs: "]);
    }
    let disk = Disk::default().with(EXPORTED, "{ \"version\": 2, \"groups\": [] }");
    assert_eq!(
        refused(&disk, chosen(&[])),
        "`task-groups.json` is a file of groups of version 2, newer than Gitardo reads: nothing was imported."
    );
    let disk = Disk::default().with(EXPORTED, "{ \"version\": 1, \"groups\": [] }");
    assert_eq!(
        refused(&disk, chosen(&[])),
        "`task-groups.json` has no groups: nothing was imported."
    );
}

#[test]
fn tasks_import_groups__a_name_is_the_task_of_tasks_json_first_then_the_script_vs_code_names_so() {
    let file = r#"{ "version": 1, "groups": [{ "name": "Mixed", "tasks": [
        "watch - extension", "lint", "watch", "lint", "build - docs", "a", "b", "c", "d", "e"
    ] }] }"#;
    let mut disk = Disk::default().with(EXPORTED, file);
    let plan = planned(&disk, chosen(&["Mixed"]));
    disk.run(&plan);
    assert_eq!(
        kept(&disk),
        ["Mixed: npm: watch - extension, lint, watch, build - docs, a, b, c, d, e"],
        "a name the worktree lacks is kept as a task of tasks.json, and one written twice is in the group once"
    );
    assert_eq!(
        said(&plan),
        "1 group was imported. This worktree does not have `watch`, `build - docs`, `a`, `b`, `c` and 2 more."
    );
}

#[test]
fn tasks_import_groups__the_groups_that_cannot_be_read_and_anything_but_an_import_change_nothing() {
    let disk = Disk::default()
        .with(crate::tasks::testing::FILE, "{ \"version\": 9, \"groups\": [] }")
        .with(EXPORTED, OF_RUN_MY_TASKS);
    assert_eq!(
        refused(&disk, chosen(&["Servers"])),
        "`gitardo/task-groups.json` could not be read: a newer Gitardo wrote it, with version 9. Nothing was changed."
    );
    let edit = GroupsTarget::Edit(GroupsAction::New { task: None });
    for target in [Target::TaskGroups(Box::new(edit)), Target::Head] {
        let Outcome::Refuse(refusal) = import_groups(&context(main_worktree(), target), &Answers::new()) else {
            panic!("a refusal");
        };
        assert_eq!(refusal.message.as_str(), "Choose Import groups… in the Tasks tab.");
    }
    assert!(matches!(
        import_groups(&import(), &Answers::new()),
        Outcome::Read(_, model::contract::Query::FileTexts { .. })
    ));
}
