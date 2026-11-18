#![allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]

use std::path::{Path, PathBuf};

use model::tasks::{TaskGroup, TaskReference};

use super::{GroupsFileProblem, common_folder, groups_path, groups_text, parse_groups};

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
        TaskGroup {
            name: String::from("Checks"),
            tasks: Vec::new(),
        },
    ]
}

const WRITTEN: &str = r#"{
    "version": 1,
    "groups": [
        {
            "name": "Servers",
            "tasks": [
                {
                    "source": "Workspace",
                    "name": "serve the docs"
                },
                {
                    "source": "npm",
                    "folder": "extension",
                    "name": "watch"
                },
                {
                    "source": "npm",
                    "folder": "",
                    "name": "dev"
                }
            ]
        },
        {
            "name": "Checks",
            "tasks": []
        }
    ]
}
"#;

#[test]
fn tasks_edit_groups__the_file_written_by_a_change_is_read_back_with_the_same_groups_in_the_same_order() {
    let text = groups_text(&groups());
    assert_eq!(text, WRITTEN);
    assert_eq!(parse_groups(Some(&text)), Ok(groups()));
    assert_eq!(parse_groups(Some(&groups_text(&[]))), Ok(Vec::new()));
}

#[test]
fn tasks_edit_groups__a_repository_with_no_file_of_groups_has_none() {
    assert_eq!(parse_groups(None), Ok(Vec::new()));
    assert_eq!(parse_groups(Some("")), Ok(Vec::new()));
    assert_eq!(parse_groups(Some("\u{feff}\n")), Ok(Vec::new()));
}

#[test]
fn tasks_edit_groups__a_file_that_is_not_the_groups_or_is_of_a_newer_gitardo_gives_none_and_says_why() {
    let said = |text: &str| parse_groups(Some(text)).unwrap_err().said();
    assert_eq!(
        said("{ \"groups\": [ }"),
        "`gitardo/task-groups.json` could not be read: at line 1, column 15, a value was expected here."
    );
    assert_eq!(
        said("{ \"version\": 2, \"groups\": [] }"),
        "`gitardo/task-groups.json` could not be read: a newer Gitardo wrote it, with version 2."
    );
    let not_groups = "`gitardo/task-groups.json` could not be read: it does not hold groups of tasks.";
    for text in [
        "[]",
        "{ \"version\": 1 }",
        "{ \"version\": \"1\", \"groups\": [] }",
        "{ \"groups\": [{ \"tasks\": [] }] }",
        "{ \"groups\": [{ \"name\": \" \", \"tasks\": [] }] }",
        "{ \"groups\": [{ \"name\": \"Servers\" }] }",
        "{ \"groups\": [{ \"name\": \"Servers\", \"tasks\": [\"build\"] }] }",
        "{ \"groups\": [{ \"name\": \"Servers\", \"tasks\": [{ \"source\": \"cargo\", \"name\": \"build\" }] }] }",
        "{ \"groups\": [{ \"name\": \"Servers\", \"tasks\": [{ \"source\": \"npm\", \"name\": \"build\" }] }] }",
        "{ \"groups\": [{ \"name\": \"Servers\", \"tasks\": [] }, { \"name\": \"Servers\", \"tasks\": [] }] }",
    ] {
        assert_eq!(parse_groups(Some(text)), Err(GroupsFileProblem::NotGroups), "{text}");
        assert_eq!(said(text), not_groups);
    }
}

#[test]
fn tasks_edit_groups__a_file_with_no_version_is_read_and_a_task_written_twice_is_in_its_group_once() {
    let text = r#"{ "groups": [{ "name": " Servers ", "tasks": [
        { "source": "Workspace", "name": "build" },
        { "source": "Workspace", "name": "build" },
        { "source": "npm", "folder": "extension", "name": "build" }
    ], "color": "teal" }], "more": true }"#;
    let read = parse_groups(Some(text)).unwrap();
    assert_eq!(
        read,
        [TaskGroup {
            name: String::from("Servers"),
            tasks: vec![
                TaskReference::workspace("build"),
                TaskReference::npm("extension", "build")
            ],
        }]
    );
}

#[test]
fn tasks_edit_groups__the_file_is_in_the_common_git_folder_which_commondir_names_for_a_linked_worktree() {
    let main = Path::new("/work/app/.git");
    assert_eq!(common_folder(main, None), main);
    assert_eq!(common_folder(main, Some("\n")), main);
    let linked = Path::new("/work/app/.git/worktrees/feature");
    assert_eq!(common_folder(linked, Some("../..\n")), main);
    assert_eq!(common_folder(linked, Some("./../../")), main);
    assert_eq!(
        common_folder(linked, Some("/elsewhere/app.git\n")),
        Path::new("/elsewhere/app.git")
    );
    assert_eq!(
        groups_path(&common_folder(linked, Some("../.."))),
        PathBuf::from("/work/app/.git/gitardo/task-groups.json")
    );
}
