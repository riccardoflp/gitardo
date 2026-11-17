#![allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]

use super::super::testing::{task, tasks, workspace};
use super::{GroupsAction, GroupsEdit, GroupsRefusal, Shift, TaskGroup, edited_groups, group_entries};
use crate::tasks::TaskReference;

fn group(name: &str, names: &[&str]) -> TaskGroup {
    TaskGroup {
        name: name.to_owned(),
        tasks: names.iter().map(|name| workspace(name)).collect(),
    }
}

/// The groups as `name: task task`.
fn shown(groups: &[TaskGroup]) -> Vec<String> {
    groups
        .iter()
        .map(|group| {
            let tasks: Vec<String> = group.tasks.iter().map(ToString::to_string).collect();
            format!("{}: {}", group.name, tasks.join(" "))
        })
        .collect()
}

fn groups() -> Vec<TaskGroup> {
    vec![group("Servers", &["api", "web", "db"]), group("Checks", &["lint"])]
}

fn done(action: GroupsAction) -> Result<Vec<String>, GroupsRefusal> {
    edited_groups(&groups(), &GroupsEdit::Do(action)).map(|groups| shown(&groups))
}

fn place(group: &str, name: &str, before: Option<&str>, from: Option<&str>) -> GroupsAction {
    GroupsAction::PlaceTask {
        group: group.to_owned(),
        task: workspace(name),
        before: before.map(workspace),
        from: from.map(str::to_owned),
    }
}

#[test]
fn tasks_edit_groups__a_new_group_is_empty_and_last() {
    let create = GroupsEdit::Create {
        name: String::from("  Watchers "),
        task: None,
    };
    let edited = edited_groups(&groups(), &create).unwrap();
    assert_eq!(shown(&edited), ["Servers: api web db", "Checks: lint", "Watchers: "]);
    let first = edited_groups(&[], &create).unwrap();
    assert_eq!(shown(&first), ["Watchers: "]);
}

#[test]
fn tasks_edit_groups__a_new_group_made_for_a_task_starts_with_it() {
    let create = GroupsEdit::Create {
        name: String::from("Watchers"),
        task: Some(TaskReference::npm("extension", "watch")),
    };
    let edited = edited_groups(&groups(), &create).unwrap();
    assert_eq!(shown(&edited).last().unwrap(), "Watchers: npm: watch - extension");
}

#[test]
fn tasks_edit_groups__a_name_empty_or_taken_is_refused() {
    let create = |name: &str| {
        edited_groups(
            &groups(),
            &GroupsEdit::Create {
                name: name.to_owned(),
                task: None,
            },
        )
    };
    assert_eq!(create("   "), Err(GroupsRefusal::NoName));
    assert_eq!(
        create("Servers"),
        Err(GroupsRefusal::NameTaken(String::from("Servers")))
    );
    assert_eq!(
        GroupsRefusal::NameTaken(String::from("Servers")).said(),
        "There is already a group named `Servers`."
    );
}

#[test]
fn tasks_edit_groups__renaming_a_group_to_the_name_of_another_is_refused_saying_so() {
    let rename = |name: &str| {
        let edit = GroupsEdit::Rename {
            group: String::from("Checks"),
            name: name.to_owned(),
        };
        edited_groups(&groups(), &edit).map(|groups| shown(&groups))
    };
    assert_eq!(
        rename("Servers"),
        Err(GroupsRefusal::NameTaken(String::from("Servers")))
    );
    assert_eq!(rename("Checks"), Err(GroupsRefusal::Unchanged));
    assert_eq!(rename("Lints").unwrap(), ["Servers: api web db", "Lints: lint"]);
    let gone = GroupsEdit::Rename {
        group: String::from("Gone"),
        name: String::from("Back"),
    };
    assert_eq!(
        edited_groups(&groups(), &gone),
        Err(GroupsRefusal::NoGroup(String::from("Gone")))
    );
}

#[test]
fn tasks_edit_groups__a_group_deleted_goes_and_the_others_stay() {
    let delete = GroupsAction::Delete {
        group: String::from("Servers"),
    };
    assert_eq!(done(delete).unwrap(), ["Checks: lint"]);
    let gone = GroupsAction::Delete {
        group: String::from("Gone"),
    };
    assert_eq!(done(gone), Err(GroupsRefusal::NoGroup(String::from("Gone"))));
}

#[test]
fn tasks_edit_groups__a_task_put_in_a_group_goes_last_or_before_another_of_its_tasks() {
    assert_eq!(
        done(place("Servers", "cache", None, None)).unwrap(),
        ["Servers: api web db cache", "Checks: lint"]
    );
    assert_eq!(
        done(place("Servers", "cache", Some("web"), None)).unwrap(),
        ["Servers: api cache web db", "Checks: lint"]
    );
    assert_eq!(
        done(place("Gone", "cache", None, None)),
        Err(GroupsRefusal::NoGroup(String::from("Gone")))
    );
}

#[test]
fn tasks_edit_groups__a_task_is_in_a_group_once_at_most_and_can_be_in_several_groups() {
    assert_eq!(done(place("Servers", "web", None, None)), Err(GroupsRefusal::Unchanged));
    assert_eq!(
        done(place("Servers", "db", Some("api"), None)).unwrap(),
        ["Servers: db api web", "Checks: lint"],
        "dragged above another task of its group, it moves there"
    );
    assert_eq!(
        done(place("Servers", "db", Some("db"), None)),
        Err(GroupsRefusal::Unchanged)
    );
    assert_eq!(
        done(place("Checks", "api", None, None)).unwrap(),
        ["Servers: api web db", "Checks: lint api"],
        "added from its menu, it is in both groups"
    );
}

#[test]
fn tasks_edit_groups__a_task_dragged_from_a_group_onto_another_leaves_the_first() {
    assert_eq!(
        done(place("Checks", "api", Some("lint"), Some("Servers"))).unwrap(),
        ["Servers: web db", "Checks: api lint"]
    );
    assert_eq!(
        done(place("Servers", "web", Some("api"), Some("Servers"))).unwrap(),
        ["Servers: web api db", "Checks: lint"],
        "dragged inside its own group, it only moves"
    );
}

#[test]
fn tasks_edit_groups__move_up_and_move_down_shift_a_task_by_one_and_stop_at_the_ends() {
    let shift = |name: &str, shift: Shift| {
        done(GroupsAction::ShiftTask {
            group: String::from("Servers"),
            task: workspace(name),
            shift,
        })
    };
    assert_eq!(
        shift("web", Shift::Up).unwrap(),
        ["Servers: web api db", "Checks: lint"]
    );
    assert_eq!(
        shift("web", Shift::Down).unwrap(),
        ["Servers: api db web", "Checks: lint"]
    );
    assert_eq!(shift("api", Shift::Up), Err(GroupsRefusal::Unchanged));
    assert_eq!(shift("db", Shift::Down), Err(GroupsRefusal::Unchanged));
    let missing = GroupsRefusal::NoTask {
        group: String::from("Servers"),
        task: workspace("lint"),
    };
    assert_eq!(missing.said(), "The group `Servers` does not have `lint`.");
    assert_eq!(shift("lint", Shift::Up), Err(missing));
}

#[test]
fn tasks_edit_groups__a_task_removed_from_a_group_leaves_the_others_in_their_order() {
    let remove = GroupsAction::RemoveTask {
        group: String::from("Servers"),
        task: workspace("web"),
    };
    assert_eq!(done(remove).unwrap(), ["Servers: api db", "Checks: lint"]);
}

#[test]
fn tasks_edit_groups__a_group_moves_up_down_or_before_another() {
    let shift = |shift: Shift| {
        done(GroupsAction::ShiftGroup {
            group: String::from("Checks"),
            shift,
        })
    };
    assert_eq!(shift(Shift::Up).unwrap(), ["Checks: lint", "Servers: api web db"]);
    assert_eq!(shift(Shift::Down), Err(GroupsRefusal::Unchanged));
    let place = |group: &str, before: Option<&str>| {
        done(GroupsAction::PlaceGroup {
            group: group.to_owned(),
            before: before.map(str::to_owned),
        })
    };
    assert_eq!(
        place("Checks", Some("Servers")).unwrap(),
        ["Checks: lint", "Servers: api web db"]
    );
    assert_eq!(place("Servers", None).unwrap(), ["Checks: lint", "Servers: api web db"]);
    assert_eq!(place("Servers", Some("Servers")), Err(GroupsRefusal::Unchanged));
    assert_eq!(
        place("Servers", Some("Gone")),
        Err(GroupsRefusal::NoGroup(String::from("Gone")))
    );
}

#[test]
fn tasks_edit_groups__a_task_the_worktree_no_longer_has_stays_in_its_group_and_says_it_is_not_here() {
    let here = tasks(vec![task("api", &[]), task("db", &[])]);
    let entries = group_entries(&group("Servers", &["api", "web", "db"]), &here);
    let listed: Vec<(String, bool)> = entries
        .iter()
        .map(|entry| (entry.task.to_string(), entry.is_here))
        .collect();
    assert_eq!(
        listed,
        [
            (String::from("api"), true),
            (String::from("web"), false),
            (String::from("db"), true)
        ]
    );
    let said: Vec<Option<String>> = entries.iter().map(super::GroupEntry::missing_said).collect();
    assert_eq!(said, [None, Some(String::from("`web` is not in this worktree.")), None]);
}
