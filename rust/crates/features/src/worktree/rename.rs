//! `worktree.rename`: gives a worktree's folder another name with `git worktree move`, and its branch too if wanted.

use std::path::{Path, PathBuf};

use model::contract::{
    Answer, AnswerKey, Answers, Context, Danger, Outcome, Plan, Question, QuestionKind, Refusal, Step, Text, Undo,
    Validation,
};
use model::state::Worktree;
use model::worktree::{EmptyFoldersInput, FolderBase, WorktreeNameRule, is_listed, remove_empty_folders};

use super::names::choice;
use super::{folder_of_worktrees, label, refuse_missing, target_worktree};

/// The new name of the folder.
pub const NEW_NAME: AnswerKey = AnswerKey("new name");
/// Whether the branch is renamed too: [`RENAME`] or [`KEEP`].
pub const RENAME_BRANCH: AnswerKey = AnswerKey("rename branch");
/// The choice that renames the branch too.
pub const RENAME: &str = "rename";
/// The choice that keeps the branch's name.
pub const KEEP: &str = "keep";
/// The new name of the branch.
pub const BRANCH_NAME: AnswerKey = AnswerKey("branch name");

/// Said beside the name: what keeps git from moving a folder.
const HELD: &str =
    "Git cannot move a folder that another program has open, such as another window or a terminal in it.";

/// Renames the folder of the worktree invoked on, inside the folder of the worktrees where its name is its path from
/// there, beside where it was otherwise; and its branch too when asked, the name typed for the folder proposed when
/// the folder was named after the branch. Then the folders left empty are deleted. The main worktree and the folder of
/// the window are not renamed.
///
/// Feature: worktree.rename
#[must_use]
pub fn rename(context: &Context, answers: &Answers) -> Outcome {
    let worktree = match target_worktree(context) {
        Ok(worktree) => worktree,
        Err(refusal) => return *refusal,
    };
    if let Some(refusal) = refused(context, worktree) {
        return refusal;
    }
    let old_label = label(context, &worktree.folder);
    let base = base_of(context, worktree, &old_label);
    let Some(typed) = answers.text(NEW_NAME) else {
        return Outcome::Ask(name_question(context, worktree, (&base, &old_label)));
    };
    let typed = typed.trim();
    let new_folder = base.folder_for(typed);
    if new_folder.to_string_lossy() == worktree.folder.to_string_lossy() {
        return Outcome::Refuse(Refusal::saying("The worktree has that name already."));
    }
    let branch = worktree
        .branch
        .as_deref()
        .map(|full| full.strip_prefix("refs/heads/").unwrap_or(full));
    let new_branch = match branch {
        None => None,
        Some(branch) => match answers.choice(RENAME_BRANCH) {
            None => return Outcome::Ask(branch_question(branch)),
            Some(KEEP) => None,
            Some(_) => match answers.text(BRANCH_NAME) {
                None => return Outcome::Ask(branch_name_question(context, branch, (&old_label, typed))),
                Some(name) => Some((branch, name.trim())),
            },
        },
    };
    Outcome::Run(plan(context, worktree, (&new_folder, new_branch), &base))
}

/// Why a worktree is not renamed: it is missing, it is the main worktree, or it is open in this window.
fn refused(context: &Context, worktree: &Worktree) -> Option<Outcome> {
    if let Some(refusal) = refuse_missing(worktree) {
        return Some(refusal);
    }
    if worktree.is_main {
        return Some(Outcome::Refuse(Refusal::saying(
            "Git cannot move the main worktree, the folder the repository was created in.",
        )));
    }
    let window = context
        .settings
        .worktrees
        .window_folder
        .iter()
        .cloned()
        .collect::<Vec<PathBuf>>();
    (worktree.is_current || is_listed(&window, &worktree.folder)).then(|| {
        Outcome::Refuse(Refusal::saying(
            "This worktree is the folder open in this window: rename it from another one.",
        ))
    })
}

/// Where the new name puts the folder: inside the folder of the worktrees when the worktree is there, beside where it
/// is otherwise.
fn base_of(context: &Context, worktree: &Worktree, old_label: &str) -> FolderBase {
    let inside = FolderBase::Worktrees(folder_of_worktrees(context));
    if is_listed(&[inside.folder_for(old_label)], &worktree.folder) {
        inside
    } else {
        FolderBase::Beside(worktree.folder.parent().map_or_default(Path::to_path_buf))
    }
}

/// The new name of the folder, the current one by default, checked while it is typed.
fn name_question(context: &Context, worktree: &Worktree, (base, old_label): (&FolderBase, &str)) -> Question {
    let detail = match base {
        FolderBase::Worktrees(_) => "New name of its folder, which can have slashes",
        FolderBase::Beside(_) => "New name of its folder",
    };
    let others = context
        .repository
        .worktrees
        .iter()
        .filter(|other| other.folder != worktree.folder)
        .map(|other| other.folder.clone())
        .collect();
    Question {
        key: NEW_NAME,
        kind: QuestionKind::Text {
            placeholder: None,
            validation: Validation::WorktreeName(Box::new(WorktreeNameRule {
                branches: None,
                empty_name: None,
                base: base.clone(),
                taken_folders: others,
            })),
        },
        title: Text::from("Rename worktree"),
        detail: Some(Text::from(format!("{detail}. {HELD}"))),
        default: Some(Answer::Text(old_label.to_owned())),
    }
}

/// Whether the branch is renamed too.
fn branch_question(branch: &str) -> Question {
    Question {
        key: RENAME_BRANCH,
        kind: QuestionKind::Pick {
            choices: vec![
                choice(RENAME, &format!("Rename the branch `{branch}` too"), None),
                choice(KEEP, &format!("Keep the branch `{branch}`"), None),
            ],
        },
        title: Text::from("Rename worktree"),
        detail: None,
        default: Some(Answer::Choice(RENAME.to_owned())),
    }
}

/// The new name of the branch, as `branch.rename` asks it: the name typed for the folder when the folder was named
/// after the branch and that name is free and valid, the branch's own name otherwise.
fn branch_name_question(context: &Context, branch: &str, (old_label, typed): (&str, &str)) -> Question {
    let taken: Vec<String> = context
        .repository
        .branches
        .iter()
        .filter(|known| known.name != branch)
        .map(|known| known.name.clone())
        .collect();
    let validation = Validation::ReferenceName {
        taken,
        taken_message: Text::from("A branch with that name already exists"),
    };
    let proposed = if old_label == branch && validation.problem(typed).is_none() {
        typed
    } else {
        branch
    };
    Question {
        key: BRANCH_NAME,
        kind: QuestionKind::Text {
            placeholder: None,
            validation,
        },
        title: Text::from("Rename branch"),
        detail: Some(Text::from(format!("New name of the branch `{branch}`"))),
        default: Some(Answer::Text(proposed.to_owned())),
    }
}

/// The folders that lead to the new place made when it is in another folder, `git worktree move`, the folders left
/// empty deleted, then `git branch -m` when the branch is renamed.
fn plan(
    context: &Context,
    worktree: &Worktree,
    (new_folder, branch): (&Path, Option<(&str, &str)>),
    base: &FolderBase,
) -> Plan {
    let directory = context.repository.folder.clone();
    let mut steps = Vec::new();
    if let Some(parent) = new_folder
        .parent()
        .filter(|parent| Some(*parent) != worktree.folder.parent())
    {
        steps.push(Step::MakeFolder {
            path: parent.to_path_buf(),
        });
    }
    let (old, new) = (worktree.folder.to_string_lossy(), new_folder.to_string_lossy());
    steps.push(Step::git(directory.clone(), ["worktree", "move", &old, &new]));
    // The folders left empty go before the branch is renamed, so that a rename git refuses leaves none behind.
    if let FolderBase::Worktrees(trees) = base {
        steps.extend(remove_empty_folders(&EmptyFoldersInput {
            worktrees_folder: trees,
            worktree: &worktree.folder,
        }));
    }
    if let Some((branch, name)) = branch.filter(|(branch, name)| branch != name) {
        steps.push(Step::git(directory, ["branch", "-m", branch, name]));
    }
    Plan {
        title: Text::from(format!(
            "Rename worktree {} to {}",
            label(context, &worktree.folder),
            label(context, new_folder)
        )),
        steps,
        danger: Danger::None,
        undo: Undo::Impossible(Text::from("Not taken back by Undo: rename it again.")),
        after: Vec::new(),
        on_failure: Vec::new(),
    }
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests;
