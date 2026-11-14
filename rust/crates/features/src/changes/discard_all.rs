//! `changes.discard-all`: takes every changed file back to the last commit, staged changes included, and deletes the
//! untracked files. Neither way removes the files git ignores: there is no `-x`.

use model::contract::{Answers, Context, Danger, Outcome, Refusal, Step, Target, Text};
use model::state::{Operation, RepositoryState};

use super::files::{
    NEVER_COMMITTED, NOTHING_TO_DISCARD, UNSTAGED_WARNING, back_to_last_commit_warning, is_staged, plan, staged_names,
    untracked_warning, with_files,
};

/// Discards every change, after a warning that counts what is lost. From the header of the Changes page
/// (`Target::Changes`) it names the files of its list and leaves the files in conflict alone; from the draft of the
/// timeline it acts on the whole working tree, files in conflict included, and is refused while an operation stopped
/// half way, which **Abort** takes back.
///
/// Feature: changes.discard-all
#[must_use]
pub fn discard_all(context: &Context, _answers: &Answers) -> Outcome {
    let repository = &context.repository;
    let discarded = if context.target == Target::Changes {
        from_list(repository)
    } else if let Some(operation) = repository.operation {
        return Outcome::Refuse(Refusal::saying(&format!(
            "Abort the {} to discard all of it.",
            operation_name(operation)
        )));
    } else {
        from_working_tree(repository)
    };
    if discarded.steps.is_empty() {
        return Outcome::Refuse(Refusal::saying(NOTHING_TO_DISCARD));
    }
    Outcome::Run(plan(
        String::from("Discard all changes"),
        discarded.steps,
        Danger::Destructive(Text::from(discarded.warning.join(" "))),
        NEVER_COMMITTED,
    ))
}

/// What a way of discarding runs, and what its warning says.
#[derive(Default)]
struct Discarded {
    /// The steps.
    steps: Vec<Step>,
    /// The sentences of the warning.
    warning: Vec<String>,
}

/// The paths of the tracked files changed, each once, by path: staged, not staged, and in conflict when asked.
fn tracked_paths(repository: &RepositoryState, with_conflicts: bool) -> Vec<&str> {
    let tree = &repository.working_tree;
    let conflicts = tree
        .conflicted
        .iter()
        .filter(|_| with_conflicts)
        .map(|file| file.path.as_str());
    let mut tracked: Vec<&str> = tree
        .staged
        .iter()
        .chain(&tree.unstaged)
        .map(|change| change.path.as_str())
        .chain(conflicts)
        .collect();
    tracked.sort_unstable();
    tracked.dedup();
    tracked
}

/// The files of the list of the Changes page, each named: the tracked ones restored, the untracked ones not staged
/// cleaned.
fn from_list(repository: &RepositoryState) -> Discarded {
    let tree = &repository.working_tree;
    let folder = &repository.folder;
    let tracked = tracked_paths(repository, false);
    let untracked: Vec<&str> = tree
        .untracked
        .iter()
        .map(String::as_str)
        .filter(|path| !is_staged(tree, path))
        .collect();
    let mut discarded = Discarded::default();
    if !tracked.is_empty() && tree.staged.is_empty() {
        discarded
            .steps
            .push(with_files(folder.clone(), &["restore"], tracked.iter().copied()));
        discarded.warning.push(UNSTAGED_WARNING.to_owned());
    } else if !tracked.is_empty() {
        let step = if repository.has_commit() {
            let names = tracked.iter().flat_map(|path| staged_names(tree, path));
            with_files(
                folder.clone(),
                &["restore", "--source=HEAD", "--staged", "--worktree"],
                names,
            )
        } else {
            let staged = tree.staged.iter().map(|change| change.path.as_str());
            with_files(folder.clone(), &["rm", "-r", "--force", "--quiet"], staged)
        };
        discarded.steps.push(step);
        discarded.warning.push(back_to_last_commit_warning(tracked.len()));
    }
    if !untracked.is_empty() {
        discarded.warning.push(untracked_warning(untracked.len()));
        discarded
            .steps
            .push(with_files(folder.clone(), &["clean", "-f"], untracked));
    }
    discarded
}

/// The whole working tree, files in conflict included: `git restore --source=HEAD --staged --worktree -- .` when a
/// tracked file changed, `git rm -r --force --quiet -- .` in its place before the first commit, then
/// `git clean -f -d -- .` when there are untracked files, folders included.
fn from_working_tree(repository: &RepositoryState) -> Discarded {
    let tree = &repository.working_tree;
    let folder = &repository.folder;
    let tracked = tracked_paths(repository, true);
    let mut discarded = Discarded::default();
    if repository.has_commit() && !tracked.is_empty() {
        discarded.steps.push(with_files(
            folder.clone(),
            &["restore", "--source=HEAD", "--staged", "--worktree"],
            ["."],
        ));
    } else if !repository.has_commit() && !tree.staged.is_empty() {
        discarded
            .steps
            .push(with_files(folder.clone(), &["rm", "-r", "--force", "--quiet"], ["."]));
    }
    if !discarded.steps.is_empty() {
        discarded.warning.push(back_to_last_commit_warning(tracked.len()));
    }
    if !tree.untracked.is_empty() {
        discarded
            .steps
            .push(with_files(folder.clone(), &["clean", "-f", "-d"], ["."]));
        discarded.warning.push(untracked_warning(tree.untracked.len()));
    }
    discarded
}

/// The name of an operation stopped half way, as the refusal says it.
fn operation_name(operation: Operation) -> &'static str {
    match operation {
        Operation::Merge => "merge",
        Operation::Rebase => "rebase",
        Operation::CherryPick => "cherry-pick",
        Operation::Revert => "revert",
        Operation::Patch => "patch",
    }
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests;
