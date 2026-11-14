//! `worktree.copy-path`: puts the full path of a worktree's folder on the clipboard.

use model::contract::{Answers, Context, FollowUp, Outcome};

use super::{label, no_steps, target_worktree};

/// Puts the folder of the worktree invoked on, as git lists it, on the clipboard, a missing worktree's too.
///
/// Feature: worktree.copy-path
#[must_use]
pub fn copy_path(context: &Context, _answers: &Answers) -> Outcome {
    let worktree = match target_worktree(context) {
        Ok(worktree) => worktree,
        Err(refusal) => return *refusal,
    };
    let path = worktree.folder.to_string_lossy().into_owned();
    Outcome::Run(no_steps(
        format!("Copy the path of the worktree {}", label(context, &worktree.folder)),
        vec![FollowUp::CopyToClipboard(path)],
    ))
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests {
    use model::contract::{Answers, FollowUp, Target};

    use super::copy_path;
    use crate::testing::plan;
    use crate::worktree::testing::{GONE, LOGIN, with_worktrees};

    #[test]
    fn worktree_copy_path__the_clipboard_holds_the_folder_as_git_lists_it_a_missing_one_too() {
        for folder in [LOGIN, GONE] {
            let context = with_worktrees(Target::Worktree(folder.into()));
            let plan = plan(copy_path(&context, &Answers::new()));
            assert_eq!(plan.steps, []);
            assert_eq!(plan.after, [FollowUp::CopyToClipboard(folder.to_owned())]);
        }
    }
}
