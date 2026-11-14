//! What a new worktree gets once `git worktree add` made it: the hand-over from the operations that make worktrees to
//! the setup of new worktrees (`worktree-setup.link-and-copy-steps`, `worktree-setup.command-steps`).

use model::contract::{Answers, Context, Outcome, Step};

pub use crate::worktree_setup::NewWorktree;

/// What a new worktree gets after `git worktree add`.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Setup {
    /// The steps that link, copy and run commands in it, in order.
    pub steps: Vec<Step>,
    /// Whether some of those steps run commands: the offer to open the worktree then comes once they all succeeded.
    pub has_commands: bool,
    /// Whether a link or copy pattern or a command applies (`worktree-setup.settings-in-effect`): when none does, the
    /// first worktree made offers **Choose What It Gets…**.
    pub is_set: bool,
    /// What Gitardo says once the plan ran, when the setup leaves something undone, such as commands not run in a
    /// folder that is not trusted.
    pub said: Option<String>,
    /// The folder the user has just trusted to run them (`trust.ask`), which the follow-ups add to the folders
    /// trusted.
    pub trusts: Option<std::path::PathBuf>,
}

/// What a new worktree gets: the steps of [`crate::worktree_setup::new_worktree_steps`], or what must be read before
/// they can be said. The operations that make worktrees (`worktree.create`, `worktree.create-from-branch`,
/// `worktree.create-at-commit`, `worktree.open-in-worktree`) call it after their questions and before their plan, again
/// with the same answers until it gives the steps.
///
/// # Errors
///
/// The outcome to return first: a read of the settings files, or of what the patterns take for the new worktree.
pub fn setup_steps(context: &Context, answers: &Answers, worktree: &NewWorktree<'_>) -> Result<Setup, Box<Outcome>> {
    let found = crate::worktree_setup::new_worktree_steps(context, answers, worktree)?;
    let has_commands = found.steps.iter().any(|step| matches!(step, Step::Task { .. }));
    // A pattern or a command in effect always says what it does in a note, even when it takes nothing.
    let is_set = found.note.is_some() || !found.steps.is_empty();
    Ok(Setup {
        steps: found.steps,
        has_commands,
        is_set,
        said: found.said,
        trusts: crate::trust::trusted_by_answer(context, answers),
    })
}
