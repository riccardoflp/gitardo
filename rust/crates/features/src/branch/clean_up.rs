//! `branch.clean-up`: lists the local branches that are done with, merged or left without their upstream, and deletes
//! the ones ticked, with the worktrees that have them.

use std::ffi::OsString;
use std::path::PathBuf;

use model::branch::other_worktree;
use model::contract::{
    Answer, AnswerKey, Answers, Choice, Context, Danger, Outcome, Plan, Query, Question, QuestionKind, Step, Text, Undo,
};
use model::state::{Branch, RepositoryState, Worktree};
use model::worktree::{EmptyFoldersInput, remove_empty_folders};

use super::target::refuse;
use crate::worktree::folder_of_worktrees;

/// The local branches merged into `HEAD`.
pub const MERGED: AnswerKey = AnswerKey("merged");

/// The branches ticked, by their names.
pub const BRANCHES: AnswerKey = AnswerKey("branches");

/// The links of the worktrees picked to folders outside them, which git on Windows would empty and git elsewhere would
/// refuse when it does not ignore them.
pub const LINKS: AnswerKey = AnswerKey("folder links");

/// Lists every local branch merged into `HEAD` or whose upstream was deleted from the remote, but the one checked out
/// here and the one of the main worktree, ticks the merged ones whose worktree has nothing uncommitted, and deletes
/// those ticked with their worktrees: the merged ones with `git branch -d`, the others with `git branch -D`.
///
/// Feature: branch.clean-up
#[must_use]
pub fn clean_up(context: &Context, answers: &Answers) -> Outcome {
    let Some(merged) = answers.names(MERGED) else {
        let into = String::from("HEAD");
        return Outcome::Read(MERGED, Query::MergedBranches { into });
    };
    let candidates = candidates(&context.repository, merged);
    if candidates.is_empty() {
        return refuse("No local branches are merged or have lost their upstream.");
    }
    let Some(ticked) = answers.choices(BRANCHES) else {
        return Outcome::Ask(pick(&context.repository, &candidates));
    };
    let picked: Vec<&Candidate<'_>> = candidates
        .iter()
        .filter(|candidate| ticked.contains(&candidate.branch.name))
        .collect();
    if picked.is_empty() {
        return refuse("No branches selected.");
    }
    let removed: Vec<PathBuf> = picked
        .iter()
        .filter_map(|candidate| candidate.worktree)
        .filter(|worktree| !worktree.is_missing)
        .map(|worktree| worktree.folder.clone())
        .collect();
    let links = if removed.is_empty() {
        Vec::new()
    } else {
        match answers.paths(LINKS) {
            Some(links) => links.to_vec(),
            None => return Outcome::Read(LINKS, Query::FolderLinks { worktrees: removed }),
        }
    };
    Outcome::Run(plan(context, &picked, &links))
}

/// A branch that may be cleaned up.
#[derive(Debug, Clone, Copy)]
struct Candidate<'a> {
    /// The branch.
    branch: &'a Branch,
    /// Whether it is merged into `HEAD`; otherwise its upstream is gone.
    is_merged: bool,
    /// The other worktree that has it checked out.
    worktree: Option<&'a Worktree>,
}

impl Candidate<'_> {
    /// Whether it is ticked to start with: merged, and its worktree, if any, has nothing uncommitted.
    fn is_ticked(&self) -> bool {
        self.is_merged
            && self
                .worktree
                .is_none_or(|worktree| worktree.uncommitted.unwrap_or(0) == 0)
    }

    /// How many files its worktree has not committed.
    fn uncommitted(&self) -> usize {
        self.worktree.and_then(|worktree| worktree.uncommitted).unwrap_or(0)
    }
}

/// The branches listed, in the order of the repository.
fn candidates<'a>(state: &'a RepositoryState, merged: &[String]) -> Vec<Candidate<'a>> {
    let reads_another_worktree = state
        .worktrees
        .iter()
        .any(|worktree| worktree.is_current && !worktree.is_main);
    let main_branch = state
        .worktrees
        .iter()
        .find(|worktree| worktree.is_main)
        .and_then(|worktree| worktree.branch.as_deref())
        .and_then(|full| full.strip_prefix("refs/heads/"))
        .filter(|_| reads_another_worktree);
    state
        .branches
        .iter()
        .filter(|branch| !branch.is_checked_out && Some(branch.name.as_str()) != main_branch)
        .filter_map(|branch| {
            let is_merged = merged.contains(&branch.name);
            let is_gone = branch.upstream.as_ref().is_some_and(|upstream| upstream.is_gone);
            (is_merged || is_gone).then(|| Candidate {
                branch,
                is_merged,
                worktree: other_worktree(state, &branch.name),
            })
        })
        .collect()
}

/// The pick of the branches to delete.
fn pick(state: &RepositoryState, candidates: &[Candidate<'_>]) -> Question {
    let into = state
        .checked_out_branch()
        .map_or_else(|| String::from("HEAD"), str::to_owned);
    let choices = candidates
        .iter()
        .map(|candidate| Choice {
            identifier: candidate.branch.name.clone(),
            label: Text::from(candidate.branch.name.as_str()),
            description: Some(Text::from(why(candidate, &into))),
        })
        .collect();
    let ticked = candidates
        .iter()
        .filter(|candidate| candidate.is_ticked())
        .map(|candidate| candidate.branch.name.clone())
        .collect();
    Question {
        key: BRANCHES,
        kind: QuestionKind::PickMany { choices },
        title: Text::from("Clean up branches"),
        detail: Some(Text::from("Branches to delete, with their worktrees")),
        default: Some(Answer::Choices(ticked)),
    }
}

/// Why a branch is listed, and the worktree that goes with it.
fn why(candidate: &Candidate<'_>, into: &str) -> String {
    let mut words = vec![if candidate.is_merged {
        format!("merged into {into}")
    } else {
        String::from("upstream is gone, not merged")
    }];
    match candidate.worktree {
        Some(worktree) if worktree.is_missing => words.push(String::from("with its missing worktree")),
        Some(worktree) => {
            words.push(format!("with its worktree {}", folder_name(worktree)));
            match candidate.uncommitted() {
                0 => {}
                1 => words.push(String::from("1 file not committed")),
                files => words.push(format!("{files} files not committed")),
            }
        }
        None => {}
    }
    words.join(", ")
}

/// The name of the folder of a worktree.
fn folder_name(worktree: &Worktree) -> String {
    worktree.folder.file_name().map_or_else(
        || worktree.folder.display().to_string(),
        |name| name.to_string_lossy().into_owned(),
    )
}

/// The plan: the missing worktrees pruned, the others removed as `worktree.remove` removes one, then the merged branches
/// deleted with `-d` and the others with `-D`, each in one command, then the folders the worktrees leave empty.
fn plan(context: &Context, picked: &[&Candidate<'_>], links: &[PathBuf]) -> Plan {
    let folder = context.repository.folder.clone();
    let worktrees: Vec<&Worktree> = picked.iter().filter_map(|candidate| candidate.worktree).collect();
    let mut steps = worktree_steps(context, &worktrees, links);
    let names = |is_merged: bool| -> Vec<&str> {
        picked
            .iter()
            .filter(|candidate| candidate.is_merged == is_merged)
            .map(|candidate| candidate.branch.name.as_str())
            .collect()
    };
    for (flag, branches) in [("-d", names(true)), ("-D", names(false))] {
        if !branches.is_empty() {
            let mut arguments = vec!["branch", flag];
            arguments.extend(branches);
            steps.push(Step::git(folder.clone(), arguments));
        }
    }
    steps.extend(empty_folder_steps(context, &worktrees));
    let title = Text::from(title(picked.len(), worktrees.len()));
    Plan {
        danger: danger(picked).unwrap_or_else(|| Danger::Confirm(title.clone())),
        title,
        steps,
        undo: Undo::Impossible(Text::from(
            "The reflog keeps no record of deleted branches, and Gitardo takes back only a branch deleted on its own.",
        )),
        after: Vec::new(),
        on_failure: Vec::new(),
    }
}

/// `git worktree prune` when a worktree picked is missing; then, for each of the others, its links to folders outside
/// it unlinked, so that git on Windows does not empty those folders through them and git elsewhere does not refuse a
/// link it does not ignore, and `git worktree remove`, with
/// `--force` when it has files not committed.
fn worktree_steps(context: &Context, worktrees: &[&Worktree], links: &[PathBuf]) -> Vec<Step> {
    let folder = &context.repository.folder;
    let mut steps = Vec::new();
    if worktrees.iter().any(|worktree| worktree.is_missing) {
        steps.push(Step::git(folder.clone(), ["worktree", "prune"]));
    }
    for worktree in worktrees.iter().filter(|worktree| !worktree.is_missing) {
        steps.extend(
            links
                .iter()
                .filter(|link| link.starts_with(&worktree.folder))
                .map(|link| Step::Unlink { path: link.clone() }),
        );
        let mut arguments: Vec<OsString> = vec!["worktree".into(), "remove".into()];
        if worktree.uncommitted.unwrap_or(0) > 0 {
            arguments.push("--force".into());
        }
        arguments.push(worktree.folder.clone().into_os_string());
        steps.push(Step::git(folder.clone(), arguments));
    }
    steps
}

/// The folders the worktrees removed leave empty in the folder of the worktrees, each deleted once.
fn empty_folder_steps(context: &Context, worktrees: &[&Worktree]) -> Vec<Step> {
    let worktrees_folder = folder_of_worktrees(context);
    let mut steps: Vec<Step> = Vec::new();
    for worktree in worktrees.iter().filter(|worktree| !worktree.is_missing) {
        let input = EmptyFoldersInput {
            worktrees_folder: &worktrees_folder,
            worktree: &worktree.folder,
        };
        for step in remove_empty_folders(&input) {
            if !steps.contains(&step) {
                steps.push(step);
            }
        }
    }
    steps
}

/// What the plan says it does: `Delete 3 branches and 1 worktree`.
fn title(branches: usize, worktrees: usize) -> String {
    let counted = |count: usize, one: &str, many: &str| {
        if count == 1 {
            format!("1 {one}")
        } else {
            format!("{count} {many}")
        }
    };
    let branches = counted(branches, "branch", "branches");
    if worktrees == 0 {
        format!("Delete {branches}")
    } else {
        format!("Delete {branches} and {}", counted(worktrees, "worktree", "worktrees"))
    }
}

/// The warning, when work can be lost: branches not merged, worktrees with files not committed.
fn danger(picked: &[&Candidate<'_>]) -> Option<Danger> {
    let not_merged: Vec<&str> = picked
        .iter()
        .filter(|candidate| !candidate.is_merged)
        .map(|candidate| candidate.branch.name.as_str())
        .collect();
    let with_changes: Vec<String> = picked
        .iter()
        .filter(|candidate| candidate.uncommitted() > 0)
        .filter_map(|candidate| candidate.worktree)
        .map(|worktree| worktree.folder.display().to_string())
        .collect();
    let mut reasons = Vec::new();
    if !not_merged.is_empty() {
        reasons.push(format!(
            "Not merged into the current branch, so their commits may become unreachable: {}.",
            not_merged.join(", ")
        ));
    }
    if !with_changes.is_empty() {
        reasons.push(format!(
            "These worktrees have files not committed, which are deleted with them: {}.",
            with_changes.join(", ")
        ));
    }
    (!reasons.is_empty()).then(|| Danger::Destructive(Text::from(reasons.join(" "))))
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests;
