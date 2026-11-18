//! `tasks.import-groups`: the groups of a file saved by **Export groups…**, or by **Export Groups...** of Run My
//! Tasks, added to those of the repository.

use model::contract::{
    Answer, AnswerKey, Answers, Choice, Context, Danger, FileQuestion, FollowUp, Outcome, Plan, Query, Question,
    QuestionKind, Refusal, Target, Text, Undo,
};
use model::tasks::{GroupsTarget, TaskGroup, TaskReference, Tasks};

use super::exchange::{Exported, counted, filters, parse_exchange};
use super::groups_file::kept_groups;

/// The key of the file chosen.
pub const FILE: AnswerKey = AnswerKey("file");

/// The key of the bytes of the file.
pub const CONTENTS: AnswerKey = AnswerKey("contents");

/// The key of the groups chosen, by their names.
pub const CHOSEN: AnswerKey = AnswerKey("groups");

/// The keys of what to do with each group chosen that is named as one of the repository, in the order of the file.
/// A file with more of them than there are keys gets, at the last, only the answers that hold for the others too.
pub const CONFLICTS: [AnswerKey; 12] = [
    AnswerKey("conflict 1"),
    AnswerKey("conflict 2"),
    AnswerKey("conflict 3"),
    AnswerKey("conflict 4"),
    AnswerKey("conflict 5"),
    AnswerKey("conflict 6"),
    AnswerKey("conflict 7"),
    AnswerKey("conflict 8"),
    AnswerKey("conflict 9"),
    AnswerKey("conflict 10"),
    AnswerKey("conflict 11"),
    AnswerKey("conflict 12"),
];

/// What the questions are called.
const TITLE: &str = "Import groups";

/// How many tasks the worktree lacks are named when the import is done; the others are counted.
const NAMED_LACKING: usize = 5;

/// What is done with a group of the file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Way {
    /// It is added, last: no group of the repository has its name.
    Add,
    /// **Merge**: the tasks the group of the repository lacks are added after its own.
    Merge,
    /// **Replace**: the group of the repository gets the tasks of the file in place of its own.
    Replace,
    /// **Skip**: the group of the repository stays as it is.
    Skip,
}

/// Adds the groups of a file to those of the repository: the file is chosen in the open dialog and read as Run My
/// Tasks 0.8.0 writes it; its groups are listed to choose from, all chosen; for each one named as a group of the
/// repository, **Merge**, **Replace** or **Skip** is asked, for it alone or for the others too. The tasks are taken by
/// the names VS Code gives them, the launch configurations are left out, and once done Gitardo says how many groups
/// were imported, the tasks the worktree shown lacks, and how many launch configurations were left out.
///
/// Feature: tasks.import-groups
#[must_use]
pub fn import_groups(context: &Context, answers: &Answers) -> Outcome {
    let tasks = match &context.target {
        Target::TaskGroups(target) => match target.as_ref() {
            GroupsTarget::Import(tasks) => tasks,
            GroupsTarget::Edit(_) => return refuse("Choose Import groups… in the Tasks tab."),
        },
        // Every other target comes without the tasks of the worktree shown, which the names of the file stand for.
        _ => return refuse("Choose Import groups… in the Tasks tab."),
    };
    let kept = match kept_groups(context, answers) {
        Ok(kept) => kept,
        Err(outcome) => return *outcome,
    };
    let exported = match exported(context, answers) {
        Ok(exported) => exported,
        Err(outcome) => return *outcome,
    };
    let Some(chosen) = answers.choices(CHOSEN) else {
        return Outcome::Ask(groups_question(&exported));
    };
    let chosen: Vec<&Exported> = exported.iter().filter(|group| chosen.contains(&group.name)).collect();
    if chosen.is_empty() {
        return refuse("No group was chosen: nothing was imported.");
    }
    let ways = match ways(&chosen, &kept.groups, answers) {
        Ok(ways) => ways,
        Err(question) => return Outcome::Ask(*question),
    };
    let import = Import::of(&kept.groups, &chosen, &ways, tasks);
    if import.count == 0 {
        return refuse("Every group chosen was skipped: nothing was imported.");
    }
    Outcome::Run(Plan {
        title: Text::from(format!("Import {}", counted(import.count, "group", "groups"))),
        // Groups merged that had every task already are written as they were, which changes nothing.
        steps: vec![kept.write_step(&import.groups)],
        danger: Danger::None,
        undo: Undo::Impossible(Text::from(
            "An import is not taken back by Undo: a group imported is deleted as any other.",
        )),
        after: vec![FollowUp::Say(Text::from(import.said()))],
        on_failure: Vec::new(),
    })
}

/// The groups of the file chosen.
///
/// # Errors
///
/// The question that chooses the file, the read of it, or the refusal for a file that is not a file of groups.
fn exported(context: &Context, answers: &Answers) -> Result<Vec<Exported>, Box<Outcome>> {
    let Some(path) = answers.path(FILE) else {
        return Err(Box::new(Outcome::Ask(Question {
            key: FILE,
            kind: QuestionKind::OpenFile(FileQuestion {
                folder: context.repository.folder.clone(),
                file_name: None,
                filters: filters(),
            }),
            title: Text::from(TITLE),
            detail: None,
            default: None,
        })));
    };
    let Some(Answer::Contents(bytes)) = answers.get(CONTENTS) else {
        let path = path.to_path_buf();
        return Err(Box::new(Outcome::Read(CONTENTS, Query::FileContents { path })));
    };
    let name = path.file_name().unwrap_or(path.as_os_str()).to_string_lossy();
    match parse_exchange(bytes) {
        Ok(groups) if groups.is_empty() => Err(Box::new(refuse(&format!(
            "`{name}` has no groups: nothing was imported."
        )))),
        Ok(groups) => Ok(groups),
        Err(problem) => Err(Box::new(refuse(&problem.said(&name)))),
    }
}

/// Which groups of the file to import, all chosen.
fn groups_question(exported: &[Exported]) -> Question {
    let choice = |group: &Exported| {
        let tasks = counted(group.tasks.len(), "task", "tasks");
        let description = if group.launches == 0 {
            tasks
        } else {
            let launches = counted(group.launches, "launch configuration", "launch configurations");
            format!("{tasks}, and {launches} left out")
        };
        Choice {
            identifier: group.name.clone(),
            label: Text::from(group.name.as_str()),
            description: Some(Text::from(description)),
        }
    };
    Question {
        key: CHOSEN,
        kind: QuestionKind::PickMany {
            choices: exported.iter().map(choice).collect(),
        },
        title: Text::from(TITLE),
        detail: None,
        default: Some(Answer::Choices(
            exported.iter().map(|group| group.name.clone()).collect(),
        )),
    }
}

/// What to do with each group chosen, in their order.
///
/// # Errors
///
/// The question for the first group named as one of the repository that has no answer yet.
fn ways(chosen: &[&Exported], kept: &[TaskGroup], answers: &Answers) -> Result<Vec<Way>, Box<Question>> {
    let is_kept = |group: &Exported| kept.iter().any(|other| other.name == group.name);
    let mut others = chosen.iter().filter(|group| is_kept(group)).count();
    let mut keys = CONFLICTS.iter();
    let mut for_the_others = None;
    let mut ways = Vec::new();
    for group in chosen {
        if !is_kept(group) {
            ways.push(Way::Add);
            continue;
        }
        others -= 1;
        if let Some(way) = for_the_others {
            ways.push(way);
            continue;
        }
        // The last key asks only for all the others, so that a key is never missing: a group past it is skipped.
        let Some(key) = keys.next() else {
            ways.push(Way::Skip);
            continue;
        };
        let only_for_all = keys.len() == 0 && others > 0;
        let Some((way, holds_for_others)) = answers.choice(*key).and_then(answered) else {
            return Err(Box::new(conflict_question(*key, &group.name, others, only_for_all)));
        };
        if holds_for_others || only_for_all {
            for_the_others = Some(way);
        }
        ways.push(way);
    }
    Ok(ways)
}

/// The answers of the question of a group named as one of the repository: the identifier, the label, what it does,
/// and whether it holds for the others too.
const ANSWERS: [(&str, &str, Way, bool); 6] = [
    ("merge", "Merge", Way::Merge, false),
    ("replace", "Replace", Way::Replace, false),
    ("skip", "Skip", Way::Skip, false),
    ("merge all", "Merge all", Way::Merge, true),
    ("replace all", "Replace all", Way::Replace, true),
    ("skip all", "Skip all", Way::Skip, true),
];

/// What an answer does, and whether it holds for the others too; none for an answer the question did not offer.
fn answered(identifier: &str) -> Option<(Way, bool)> {
    let found = ANSWERS.iter().find(|(known, ..)| *known == identifier)?;
    Some((found.2, found.3))
}

/// **`<name>` is already a group.**, with **Merge**, **Replace** and **Skip**, and the same for the others when there
/// are more.
fn conflict_question(key: AnswerKey, name: &str, others: usize, only_for_all: bool) -> Question {
    let does = |way: Way| match way {
        Way::Merge => "Add the tasks it lacks after its own",
        Way::Replace => "Take the tasks of the file in place of its own",
        Way::Skip | Way::Add => "Leave it as it is",
    };
    let choices = ANSWERS
        .iter()
        .filter(|(.., for_others)| if *for_others { others > 0 } else { !only_for_all })
        .map(|(identifier, label, way, for_others)| Choice {
            identifier: (*identifier).to_owned(),
            label: Text::from(*label),
            description: Some(Text::from(if *for_others {
                format!("{}, and do the same for the others", does(*way))
            } else {
                String::from(does(*way))
            })),
        })
        .collect();
    Question {
        key,
        kind: QuestionKind::Pick { choices },
        title: Text::from(format!("`{name}` is already a group.")),
        detail: (others > 0).then(|| {
            Text::from(if others == 1 {
                String::from("1 more group of the file is named as a group of the repository.")
            } else {
                format!("{others} more groups of the file are named as groups of the repository.")
            })
        }),
        default: None,
    }
}

/// What an import comes to.
struct Import {
    /// The groups of the repository after it.
    groups: Vec<TaskGroup>,
    /// How many groups were imported: added, merged or replaced.
    count: usize,
    /// How many launch configurations those groups had in the file.
    launches: usize,
    /// The tasks of those groups the worktree shown lacks, each once.
    lacking: Vec<TaskReference>,
}

impl Import {
    /// The import of the groups chosen, each done its way, into the groups of the repository.
    fn of(kept: &[TaskGroup], chosen: &[&Exported], ways: &[Way], tasks: &Tasks) -> Import {
        let mut import = Import {
            groups: kept.to_vec(),
            count: 0,
            launches: 0,
            lacking: Vec::new(),
        };
        for (group, way) in chosen.iter().zip(ways) {
            let references = group.references(tasks);
            let existing = import.groups.iter_mut().find(|other| other.name == group.name);
            match (way, existing) {
                (Way::Skip, _) => continue,
                (Way::Merge, Some(existing)) => {
                    let lacked: Vec<TaskReference> = references
                        .iter()
                        .filter(|task| !existing.tasks.contains(task))
                        .cloned()
                        .collect();
                    existing.tasks.extend(lacked);
                }
                (Way::Replace, Some(existing)) => existing.tasks.clone_from(&references),
                // A group of the repository deleted since the question is added as a new one.
                (Way::Add, _) | (Way::Merge | Way::Replace, None) => import.groups.push(TaskGroup {
                    name: group.name.clone(),
                    tasks: references.clone(),
                }),
            }
            import.count += 1;
            import.launches += group.launches;
            for task in references {
                if tasks.find(&task).is_none() && !import.lacking.contains(&task) {
                    import.lacking.push(task);
                }
            }
        }
        import
    }

    /// What Gitardo says once the import is done.
    fn said(&self) -> String {
        let were = if self.count == 1 { "was" } else { "were" };
        let mut said = vec![format!("{} {were} imported.", counted(self.count, "group", "groups"))];
        if !self.lacking.is_empty() {
            let named: Vec<String> = self
                .lacking
                .iter()
                .take(NAMED_LACKING)
                .map(|task| format!("`{task}`"))
                .collect();
            let more = self.lacking.len().saturating_sub(NAMED_LACKING);
            let more = if more == 0 {
                String::new()
            } else {
                format!(" and {more} more")
            };
            said.push(format!("This worktree does not have {}{more}.", named.join(", ")));
        }
        if self.launches > 0 {
            let were = if self.launches == 1 { "was" } else { "were" };
            let launches = counted(self.launches, "launch configuration", "launch configurations");
            said.push(format!("{launches} {were} left out, since Gitardo has no debugger."));
        }
        said.join(" ")
    }
}

/// The refusal that says a sentence.
fn refuse(message: &str) -> Outcome {
    Outcome::Refuse(Refusal::saying(message))
}

#[cfg(test)]
#[path = "import_groups_tests.rs"]
mod tests;
