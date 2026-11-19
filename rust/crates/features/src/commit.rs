//! Committing: the commit of what is staged, with the message of the commit box, and amending the last commit
//! (`docs/features/05-changes-and-commit.md`).

mod amend;
mod create;

use model::commit::{CoAuthor, CommitOptions};
use model::contract::{AnswerKey, Answers};

pub use amend::amend;
pub use create::{MESSAGE, create};

/// **Sign this commit**, ticked or not; absent while it is as git's configuration signs (`commit.options`).
pub const SIGN: AnswerKey = AnswerKey("sign");

/// **Add Signed-off-by**.
pub const SIGN_OFF: AnswerKey = AnswerKey("sign-off");

/// **Skip the hooks**.
pub const SKIP_HOOKS: AnswerKey = AnswerKey("skip-hooks");

/// The co-authors chosen, each as `Name <email>`, in the order chosen.
pub const CO_AUTHORS: AnswerKey = AnswerKey("co-authors");

/// The options of the commit box, from the answers the window or the extension gives with the run: none given is a
/// commit as git's configuration makes it. They are never asked, so that the command line commits as before.
#[must_use]
pub fn options_of(answers: &Answers) -> CommitOptions {
    CommitOptions {
        sign: answers.flag(SIGN),
        sign_off: answers.flag(SIGN_OFF).unwrap_or(false),
        skip_hooks: answers.flag(SKIP_HOOKS).unwrap_or(false),
        co_authors: answers
            .choices(CO_AUTHORS)
            .unwrap_or_default()
            .iter()
            .filter_map(|typed| CoAuthor::parse(typed))
            .collect(),
    }
}
