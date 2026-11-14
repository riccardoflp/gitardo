//! `worktree.reveal`: shows a worktree's folder in the file manager of the system.

use model::contract::{Answers, Context, Outcome};

use super::{label, opening, refuse_missing, target_worktree};

/// Shows the folder of the worktree invoked on in the file manager of the system: File Explorer on Windows, Finder on
/// macOS, the folder's program elsewhere; `run <command>`.
///
/// Feature: worktree.reveal
#[must_use]
pub fn reveal(context: &Context, _answers: &Answers) -> Outcome {
    let worktree = match target_worktree(context) {
        Ok(worktree) => worktree,
        Err(refusal) => return *refusal,
    };
    if let Some(refusal) = refuse_missing(worktree) {
        return refusal;
    }
    Outcome::Run(opening::in_file_manager(
        &worktree.folder,
        &label(context, &worktree.folder),
    ))
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests {
    use model::contract::{Answers, Target};

    use super::reveal;
    use crate::testing::{lines, plan};
    use crate::worktree::testing::{LOGIN, with_worktrees};

    #[test]
    fn worktree_reveal__the_file_manager_of_the_system_opens_on_the_worktrees_folder() {
        let context = with_worktrees(Target::Worktree(LOGIN.into()));
        let plan = plan(reveal(&context, &Answers::new()));
        let expected = if cfg!(windows) {
            String::from(r#"run start "" \code\app.worktrees\feature\login"#)
        } else if cfg!(target_os = "macos") {
            format!("run open {LOGIN}")
        } else {
            format!("run xdg-open {LOGIN}")
        };
        assert_eq!(lines(&plan), [expected]);
    }
}
