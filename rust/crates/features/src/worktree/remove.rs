//! `worktree.remove`: removes a worktree's folder with `git worktree remove`, and its branch too if wanted.

use model::contract::{
    Answer, AnswerKey, Answers, Context, Danger, FeatureId, FollowUp, OfferedRun, Outcome, Plan, Query, Question,
    QuestionKind, Refusal, Step, Target, Text, Undo,
};
use model::state::Worktree;
use model::worktree::{EmptyFoldersInput, is_listed, remove_empty_folders};

use super::names::choice;
use super::{folder_of_worktrees, label, refuse_missing, target_worktree};

pub use model::worktree::{DELETE_BRANCH, KEEP_BRANCH};

/// Whether the branch goes too: [`KEEP_BRANCH`] or [`DELETE_BRANCH`].
pub const BRANCH: AnswerKey = model::worktree::REMOVE_BRANCH;
/// Confirmed by **Remove anyway**: `git worktree remove --force`.
pub const FORCE: AnswerKey = AnswerKey("force");
/// The links to folders outside the worktree, read on Windows.
const LINKS: AnswerKey = AnswerKey("folder links");

/// What `--force` deletes.
const FORCED: &str = "This worktree has modified or untracked files. They will be deleted with it.";
/// Among git's words when it refuses a worktree with changes.
const CHANGES_SAID: [&str; 2] = ["modified or untracked", "--force"];

/// Removes the worktree invoked on: on Windows the links it has to folders outside it first, which git would empty;
/// then `git worktree remove`, with `--force` when **Remove anyway** asked it after git refused a worktree with
/// changes; then the folders left empty; then `git branch -d` when the branch goes too, which keeps a branch that is not
/// merged and says so. The main worktree, the one the views show and the folder of the window are not removed.
///
/// Feature: worktree.remove
#[must_use]
pub fn remove(context: &Context, answers: &Answers) -> Outcome {
    let worktree = match target_worktree(context) {
        Ok(worktree) => worktree,
        Err(refusal) => return *refusal,
    };
    if let Some(refusal) = refused(context, worktree) {
        return refusal;
    }
    let branch = worktree
        .branch
        .as_deref()
        .map(|full| full.strip_prefix("refs/heads/").unwrap_or(full));
    let deletes_branch = match branch {
        None => false,
        Some(branch) => match answers.choice(BRANCH) {
            None => return Outcome::Ask(branch_question(branch)),
            Some(choice) => choice == DELETE_BRANCH,
        },
    };
    let links = if cfg!(windows) {
        match answers.paths(LINKS) {
            Some(links) => links.to_vec(),
            None => {
                return Outcome::Read(
                    LINKS,
                    Query::FolderLinks {
                        worktrees: vec![worktree.folder.clone()],
                    },
                );
            }
        }
    } else {
        Vec::new()
    };
    let is_forced = answers.is_confirmed(FORCE);
    Outcome::Run(plan(
        context,
        worktree,
        (branch.filter(|_| deletes_branch), &links, is_forced),
    ))
}

/// Why a worktree is not removed: it is missing, the main worktree, or open in this window.
fn refused(context: &Context, worktree: &Worktree) -> Option<Outcome> {
    if let Some(refusal) = refuse_missing(worktree) {
        return Some(refusal);
    }
    if worktree.is_main {
        return Some(Outcome::Refuse(Refusal::saying(
            "Git cannot remove the main worktree, the folder the repository was created in.",
        )));
    }
    let window: Vec<_> = context.settings.worktrees.window_folder.iter().cloned().collect();
    (worktree.is_current || is_listed(&window, &worktree.folder)).then(|| {
        Outcome::Refuse(Refusal::saying(
            "This worktree is the folder open in this window: remove it from another one.",
        ))
    })
}

/// Whether the branch goes too, kept by default.
fn branch_question(branch: &str) -> Question {
    Question {
        key: BRANCH,
        kind: QuestionKind::Pick {
            choices: vec![
                choice(
                    KEEP_BRANCH,
                    "Remove the worktree",
                    Some(format!("Keep the branch `{branch}`")),
                ),
                choice(
                    DELETE_BRANCH,
                    &format!("Remove the worktree and delete `{branch}`"),
                    Some(String::from("Only if it is merged")),
                ),
            ],
        },
        title: Text::from("Remove worktree"),
        detail: None,
        default: Some(Answer::Choice(KEEP_BRANCH.to_owned())),
    }
}

/// The plan: the links unlinked, the worktree removed, the folders left empty deleted, then the branch.
fn plan(
    context: &Context,
    worktree: &Worktree,
    (branch, links, is_forced): (Option<&str>, &[std::path::PathBuf], bool),
) -> Plan {
    let directory = context.repository.folder.clone();
    let folder = worktree.folder.to_string_lossy().into_owned();
    let mut steps: Vec<Step> = links.iter().map(|link| Step::Unlink { path: link.clone() }).collect();
    let mut arguments = vec!["worktree", "remove"];
    if is_forced {
        arguments.push("--force");
    }
    arguments.push(&folder);
    steps.push(Step::git(directory.clone(), arguments));
    steps.extend(remove_empty_folders(&EmptyFoldersInput {
        worktrees_folder: &folder_of_worktrees(context),
        worktree: &worktree.folder,
    }));
    if let Some(branch) = branch {
        steps.push(Step::git(directory, ["branch", "-d", branch]));
    }
    let danger = if is_forced {
        Danger::Destructive(Text::from(FORCED))
    } else {
        Danger::Confirm(Text::from(match branch {
            Some(branch) => format!(
                "The directory is deleted. The branch `{branch}` is deleted too if it is merged; if it is not, git keeps it and says so."
            ),
            None => String::from("The directory is deleted. Its branch and commits stay in the repository."),
        }))
    };
    let mut answers = Answers::new().with(FORCE, Answer::Confirmed);
    answers = answers.with(
        BRANCH,
        Answer::Choice(if branch.is_some() { DELETE_BRANCH } else { KEEP_BRANCH }.to_owned()),
    );
    let on_failure = if is_forced {
        Vec::new()
    } else {
        vec![FollowUp::OfferRun(Box::new(OfferedRun {
            label: Text::from("Remove anyway"),
            feature: FeatureId("worktree.remove"),
            target: Target::Worktree(worktree.folder.clone()),
            answers,
            when_said: CHANGES_SAID.iter().map(|said| (*said).to_owned()).collect(),
        }))]
    };
    Plan {
        title: Text::from(format!("Remove worktree {}", label(context, &worktree.folder))),
        steps,
        danger,
        undo: Undo::Impossible(Text::from(
            "Cannot be undone: the folder is deleted. Its commits stay in the repository.",
        )),
        after: Vec::new(),
        on_failure,
    }
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests;
