//! `changes.discard`: takes one file back to the last commit, staged changes included, or deletes it when it is
//! untracked.

use model::contract::{Answers, Context, Danger, Outcome, Refusal, Step, Target, Text};

use super::files::{
    NEVER_COMMITTED, NOTHING_TO_DISCARD, UNSTAGED_WARNING, back_to_last_commit_warning, is_staged, is_unstaged,
    is_untracked, plan, staged_deleted_warning, staged_names, target_file, untracked_warning, with_files,
};

/// Discards the changes of a file of the Changes page, after a warning that says what is lost: with nothing of it
/// staged, `git restore -- <path>`, or `git clean -f -- <path>` for an untracked file; with some of it staged,
/// `git restore --source=HEAD --staged --worktree -- <path>`, with the old name of a rename; before the first commit, a
/// staged file is deleted with `git rm -r --force --quiet -- <path>`, and the warning says so. The files ticked (`Target::Files`) are discarded
/// at once, as **Discard all** discards the files of the list, with one warning that counts them.
///
/// Feature: changes.discard
#[must_use]
pub fn discard(context: &Context, _answers: &Answers) -> Outcome {
    if let Target::Files(paths) = &context.target
        && paths.len() > 1
    {
        return super::discard_all::the_files(&context.repository, paths);
    }
    let path = match target_file(context) {
        Ok(path) => path,
        Err(refusal) => return Outcome::Refuse(refusal),
    };
    let repository = &context.repository;
    let tree = &repository.working_tree;
    let folder = repository.folder.clone();
    let (step, warning): (Step, String) = if is_staged(tree, path) {
        if repository.has_commit() {
            let names = staged_names(tree, path);
            let arguments = ["restore", "--source=HEAD", "--staged", "--worktree"];
            (with_files(folder, &arguments, names), back_to_last_commit_warning(1))
        } else {
            let step = with_files(folder, &["rm", "-r", "--force", "--quiet"], [path]);
            (step, staged_deleted_warning(1))
        }
    } else if is_unstaged(tree, path) {
        (with_files(folder, &["restore"], [path]), UNSTAGED_WARNING.to_owned())
    } else if is_untracked(tree, path) {
        (with_files(folder, &["clean", "-f"], [path]), untracked_warning(1))
    } else {
        return Outcome::Refuse(Refusal::saying(NOTHING_TO_DISCARD));
    };
    Outcome::Run(plan(
        format!("Discard the changes of {path}"),
        vec![step],
        Danger::Destructive(Text::from(warning)),
        NEVER_COMMITTED,
    ))
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests;
