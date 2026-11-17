#![allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]

use std::path::PathBuf;

use model::contract::{Answer, Answers, Context, FollowUp, Outcome, QuestionKind, Target};
use model::state::RepositoryState;
use model::tasks::{GroupsTarget, PackageManager, PackageScripts, Task, TaskGroup, TaskReference, Tasks};

use super::{CHOSEN, PATH, export_groups};
use crate::tasks::import_groups::{self, import_groups};
use crate::tasks::testing::{Disk, FILE, disk_with, group, kept, main_worktree};
use crate::testing::{context, plan, question};

/// Where the file is saved in these tests.
const SAVED: &str = "/downloads/task-groups.json";

/// The groups of the repository exported: tasks of `tasks.json`, a script of a folder and one of the top.
fn groups() -> Vec<TaskGroup> {
    vec![
        TaskGroup {
            name: String::from("Servers"),
            tasks: vec![
                TaskReference::workspace("serve the docs"),
                TaskReference::npm("extension", "watch"),
                TaskReference::npm("", "dev"),
            ],
        },
        group("Checks", &["lint", "test"]),
        group("Empty", &[]),
    ]
}

fn export() -> Context {
    context(main_worktree(), Target::Head)
}

fn answers(names: &[&str]) -> Answers {
    let names = names.iter().map(|name| (*name).to_owned()).collect();
    Answers::new()
        .with(CHOSEN, Answer::Choices(names))
        .with(PATH, Answer::Path(PathBuf::from(SAVED)))
}

/// Another repository, in `/work/other`, whose worktree has the same tasks.
fn other_repository() -> Context {
    let task = |reference: TaskReference| Task::named(reference, PathBuf::from("/work/other"));
    let package = |folder: &str, script: &str| PackageScripts {
        folder: folder.to_owned(),
        name: folder.to_owned(),
        manager: PackageManager::Npm,
        scripts: vec![task(TaskReference::npm(folder, script))],
    };
    let tasks = Tasks {
        workspace: ["serve the docs", "lint", "test"]
            .iter()
            .map(|name| task(TaskReference::workspace(name)))
            .collect(),
        packages: vec![package("", "dev"), package("extension", "watch")],
        problems: Vec::new(),
    };
    let state = RepositoryState {
        folder: PathBuf::from("/work/other"),
        git_folder: PathBuf::from("/work/other/.git"),
        ..crate::testing::state()
    };
    context(state, Target::TaskGroups(Box::new(GroupsTarget::Import(tasks))))
}

#[test]
fn tasks_export_groups__asks_which_groups_all_chosen_then_where_with_task_groups_json_proposed() {
    let disk = disk_with(&groups());
    let which = question(disk.asked(export_groups, &export(), Answers::new()));
    assert_eq!((which.key, which.title.as_str()), (CHOSEN, "Export groups"));
    let QuestionKind::PickMany { choices } = &which.kind else {
        panic!("choices: {:?}", which.kind);
    };
    let offered: Vec<String> = choices
        .iter()
        .map(|choice| format!("{} ({})", choice.label, choice.description.as_ref().unwrap()))
        .collect();
    assert_eq!(offered, ["Servers (3 tasks)", "Checks (2 tasks)", "Empty (0 tasks)"]);
    let all = ["Servers", "Checks", "Empty"].map(String::from).to_vec();
    assert_eq!(which.default, Some(Answer::Choices(all.clone())));
    let chosen = Answers::new().with(CHOSEN, Answer::Choices(all));
    let place = question(disk.asked(export_groups, &export(), chosen));
    assert_eq!(place.key, PATH);
    let QuestionKind::SaveFile(dialog) = &place.kind else {
        panic!("the system's dialog: {:?}", place.kind);
    };
    assert_eq!(dialog.folder, PathBuf::from("/work/app"));
    assert_eq!(dialog.file_name.as_deref(), Some("task-groups.json"));
}

#[test]
fn tasks_export_groups__two_groups_exported_and_imported_in_another_repository_give_the_same_groups_in_order() {
    let mut disk = disk_with(&groups());
    let exported = plan(disk.asked(export_groups, &export(), answers(&["Checks", "Servers"])));
    assert_eq!(exported.title.as_str(), "Export 2 groups");
    let [FollowUp::Say(said)] = exported.after.as_slice() else {
        panic!("one thing said: {:?}", exported.after);
    };
    assert_eq!(
        said.to_string(),
        format!("2 groups were saved in `{}`.", PathBuf::from(SAVED).display())
    );
    disk.run(&exported);
    let chosen = Answers::new()
        .with(import_groups::FILE, Answer::Path(PathBuf::from(SAVED)))
        .with(
            import_groups::CHOSEN,
            Answer::Choices(vec![String::from("Servers"), String::from("Checks")]),
        );
    let imported = plan(disk.asked(import_groups, &other_repository(), chosen));
    disk.run(&imported);
    let theirs = disk.text("/work/other/.git/gitardo/task-groups.json");
    let theirs = crate::tasks::groups_file::parse_groups(theirs.as_deref()).unwrap();
    assert_eq!(
        theirs,
        &groups()[..2],
        "the same groups, in the order of the repository they came from"
    );
    let [FollowUp::Say(said)] = imported.after.as_slice() else {
        panic!("one thing said: {:?}", imported.after);
    };
    assert_eq!(said.to_string(), "2 groups were imported.");
    assert_eq!(
        kept(&disk).len(),
        3,
        "the groups of the first repository stay as they were"
    );
}

#[test]
fn tasks_export_groups__the_file_exported_is_the_one_run_my_tasks_reads_with_its_groups() {
    let mut disk = disk_with(&groups());
    let exported = plan(disk.asked(export_groups, &export(), answers(&["Servers", "Empty"])));
    disk.run(&exported);
    let written = disk.text(SAVED).unwrap();
    let expected = r#"{
    "version": 1,
    "groups": [
        {
            "name": "Servers",
            "tasks": [
                "serve the docs",
                "watch - extension",
                "dev"
            ],
            "launches": []
        },
        {
            "name": "Empty",
            "tasks": [],
            "launches": []
        }
    ]
}
"#;
    assert_eq!(written, expected);
    // What Run My Tasks 0.8.0 asks of a file before it imports it: an object with `groups`, a numeric `version` no
    // newer than its own, and for each group a name that is not empty, `tasks` of strings and `launches` of strings.
    let root = json::parse(&written).unwrap();
    assert_eq!(
        root.get("version")
            .and_then(json::Value::as_number)
            .and_then(json::Number::as_u64),
        Some(1)
    );
    for group in root.get("groups").and_then(json::Value::as_array).unwrap() {
        let name = group.get("name").and_then(json::Value::as_str).unwrap();
        assert_ne!(name.trim(), "");
        for member in ["tasks", "launches"] {
            let items = group.get(member).and_then(json::Value::as_array).unwrap();
            assert!(items.iter().all(|item| item.as_str().is_some()), "{member}");
        }
    }
}

#[test]
fn tasks_export_groups__with_no_groups_or_none_chosen_nothing_is_saved() {
    let Outcome::Refuse(refusal) = Disk::default().asked(export_groups, &export(), Answers::new()) else {
        panic!("a refusal");
    };
    assert_eq!(refusal.message.as_str(), "There are no groups to export.");
    let disk = disk_with(&groups());
    let Outcome::Refuse(refusal) = disk.asked(export_groups, &export(), answers(&[])) else {
        panic!("a refusal");
    };
    assert_eq!(refusal.message.as_str(), "No group was chosen: nothing was exported.");
    let one = plan(disk.asked(export_groups, &export(), answers(&["Empty"])));
    assert_eq!(one.title.as_str(), "Export 1 group");
    assert!(
        matches!(one.after.as_slice(), [FollowUp::Say(said)] if said.as_str().starts_with("1 group was saved in "))
    );
    assert!(disk.text(FILE).is_some());
}
