//! `tasks.edit-groups`: a group of tasks made, renamed, deleted or moved, and a task put in a group, moved in it or
//! taken out, each written to the file of the groups of the repository.

use model::contract::{
    Answer, AnswerKey, Answers, Context, Danger, Outcome, Plan, Question, QuestionKind, Refusal, Target, Text, Undo,
    Validation,
};
use model::tasks::{GroupsAction, GroupsEdit, GroupsRefusal, GroupsTarget, TaskGroup, edited_groups};

use super::groups_file::kept_groups;

/// The key of the name of a new group, or of the new name of a group.
pub const NAME: AnswerKey = AnswerKey("name");

/// The key of the confirmation that deletes a group.
pub const DELETE: AnswerKey = AnswerKey("delete");

/// Makes the change the Tasks tab asked of the groups of the repository, and writes them whole to
/// `gitardo/task-groups.json` in the common git folder, which every worktree reads: a new group, whose name is asked,
/// empty or with the task it was made for; a group renamed, whose new name is asked; a group deleted, after asking; a
/// group moved; a task put in a group, moved in it or to another, or taken out. A name another group has is refused,
/// and so is a change of a group another window has deleted since.
///
/// Feature: tasks.edit-groups
#[must_use]
pub fn edit_groups(context: &Context, answers: &Answers) -> Outcome {
    let action = match &context.target {
        Target::TaskGroups(target) => match target.as_ref() {
            GroupsTarget::Edit(action) => action,
            GroupsTarget::Import(_) => return refuse("Choose a group or a task in the Tasks tab."),
        },
        // Every other target is no group and no task of one.
        _ => return refuse("Choose a group or a task in the Tasks tab."),
    };
    let kept = match kept_groups(context, answers) {
        Ok(kept) => kept,
        Err(outcome) => return *outcome,
    };
    let edit = match edit_of(action, &kept.groups, answers) {
        Ok(edit) => edit,
        Err(outcome) => return *outcome,
    };
    let groups = match edited_groups(&kept.groups, &edit) {
        Ok(groups) => groups,
        Err(refusal) => return refuse(&refusal.said()),
    };
    Outcome::Run(Plan {
        title: Text::from(title(&edit, &kept.groups)),
        steps: vec![kept.write_step(&groups)],
        danger: Danger::None,
        undo: Undo::Impossible(Text::from(
            "A change of the groups is not taken back by Undo: a group deleted is made again by hand, or imported.",
        )),
        after: Vec::new(),
        on_failure: Vec::new(),
    })
}

/// The change with the names it needs, once they are known.
///
/// # Errors
///
/// The question still to ask, or the refusal for a group that is no longer there or a task its group has already.
fn edit_of(action: &GroupsAction, groups: &[TaskGroup], answers: &Answers) -> Result<GroupsEdit, Box<Outcome>> {
    let has = |name: &str| groups.iter().any(|group| group.name == name);
    let gone = |name: &str| Box::new(refuse(&GroupsRefusal::NoGroup(name.to_owned()).said()));
    match action {
        GroupsAction::New { task } => {
            let Some(name) = answers.text(NAME) else {
                return Err(Box::new(Outcome::Ask(name_question("New group", None))));
            };
            Ok(GroupsEdit::Create {
                name: name.to_owned(),
                task: task.clone(),
            })
        }
        GroupsAction::Rename { group } | GroupsAction::Delete { group } if !has(group) => Err(gone(group)),
        GroupsAction::Rename { group } => {
            let Some(name) = answers.text(NAME) else {
                let title = format!("Rename the group `{group}`");
                return Err(Box::new(Outcome::Ask(name_question(&title, Some(group)))));
            };
            Ok(GroupsEdit::Rename {
                group: group.clone(),
                name: name.to_owned(),
            })
        }
        GroupsAction::Delete { group } if !answers.is_confirmed(DELETE) => Err(Box::new(Outcome::Ask(Question {
            key: DELETE,
            kind: QuestionKind::Confirm {
                accept: Text::from("Delete"),
            },
            title: Text::from(format!("Delete the group `{group}`?")),
            detail: Some(Text::from("Its tasks stay where they are.")),
            default: None,
        }))),
        GroupsAction::PlaceTask {
            group,
            task,
            before: None,
            from: None,
        } if groups
            .iter()
            .any(|other| other.name == *group && other.tasks.contains(task)) =>
        {
            Err(Box::new(refuse(&format!(
                "`{task}` is already in the group `{group}`."
            ))))
        }
        GroupsAction::Delete { .. }
        | GroupsAction::PlaceGroup { .. }
        | GroupsAction::ShiftGroup { .. }
        | GroupsAction::PlaceTask { .. }
        | GroupsAction::ShiftTask { .. }
        | GroupsAction::RemoveTask { .. } => Ok(GroupsEdit::Do(action.clone())),
    }
}

/// The question that asks the name of a group, starting from the one it has.
fn name_question(title: &str, name: Option<&String>) -> Question {
    Question {
        key: NAME,
        kind: QuestionKind::Text {
            placeholder: Some(Text::from("Servers")),
            validation: Validation::Required(Text::from(GroupsRefusal::NoName.said())),
        },
        title: Text::from(title),
        detail: None,
        default: name.map(|name| Answer::Text(name.clone())),
    }
}

/// What the plan of a change is called, from the groups as they were.
fn title(edit: &GroupsEdit, groups: &[TaskGroup]) -> String {
    match edit {
        GroupsEdit::Create { name, .. } => format!("Make the group `{}`", name.trim()),
        GroupsEdit::Rename { group, name } => format!("Rename the group `{group}` to `{}`", name.trim()),
        GroupsEdit::Do(action) => match action {
            // A new group and a new name come with their names, as the two arms above.
            GroupsAction::New { .. } | GroupsAction::Rename { .. } => String::from("Change the groups"),
            GroupsAction::Delete { group } => format!("Delete the group `{group}`"),
            GroupsAction::PlaceGroup { group, .. } | GroupsAction::ShiftGroup { group, .. } => {
                format!("Move the group `{group}`")
            }
            GroupsAction::PlaceTask { group, task, from, .. } => {
                let was_there = groups
                    .iter()
                    .any(|other| other.name == *group && other.tasks.contains(task));
                match from.as_ref().filter(|from| *from != group) {
                    Some(from) => format!("Move `{task}` from the group `{from}` to `{group}`"),
                    None if was_there => format!("Move `{task}` in the group `{group}`"),
                    None => format!("Add `{task}` to the group `{group}`"),
                }
            }
            GroupsAction::ShiftTask { group, task, .. } => format!("Move `{task}` in the group `{group}`"),
            GroupsAction::RemoveTask { group, task } => format!("Remove `{task}` from the group `{group}`"),
        },
    }
}

/// The refusal that says a sentence.
fn refuse(message: &str) -> Outcome {
    Outcome::Refuse(Refusal::saying(message))
}

#[cfg(test)]
#[path = "edit_groups_tests.rs"]
mod tests;
