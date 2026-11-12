//! The update of the submodules that ends a checkout, as `submodule.update-on-checkout` says: with
//! `submodules.updateOnCheckout` on and submodules on either side, `git submodule update --init --recursive`.

use model::contract::{AnswerKey, Answers, Context, Outcome, Query, Step};

/// Whether what is checked out now has submodules.
pub(crate) const SUBMODULES_BEFORE: AnswerKey = AnswerKey("submodules-before");

/// Whether what is checked out next has submodules.
pub(crate) const SUBMODULES_AFTER: AnswerKey = AnswerKey("submodules-after");

/// The file that says a commit has submodules.
const GITMODULES: &str = ".gitmodules";

/// The steps that follow a checkout of `revision`: the update of the submodules, or nothing; or the read that must
/// come first, as the outcome to return.
pub(crate) fn update_after_checkout(
    context: &Context,
    answers: &Answers,
    revision: &str,
) -> Result<Vec<Step>, Box<Outcome>> {
    if !context.settings.update_submodules_on_checkout {
        return Ok(Vec::new());
    }
    let has_submodules = match answers.flag(SUBMODULES_BEFORE) {
        None => return Err(Box::new(read(SUBMODULES_BEFORE, "HEAD"))),
        Some(true) => true,
        Some(false) => match answers.flag(SUBMODULES_AFTER) {
            None => return Err(Box::new(read(SUBMODULES_AFTER, revision))),
            Some(after) => after,
        },
    };
    Ok(if has_submodules {
        vec![Step::git(
            context.repository.folder.clone(),
            ["submodule", "update", "--init", "--recursive"],
        )]
    } else {
        Vec::new()
    })
}

/// Reads whether a commit has `.gitmodules`.
fn read(key: AnswerKey, revision: &str) -> Outcome {
    Outcome::Read(
        key,
        Query::HasFile {
            revision: revision.to_owned(),
            path: String::from(GITMODULES),
        },
    )
}

/// Answers saying that neither side of a checkout has submodules, for the tests of the operations that check out.
#[cfg(test)]
pub(crate) fn none(answers: Answers) -> Answers {
    use model::contract::Answer;
    answers
        .with(SUBMODULES_BEFORE, Answer::Flag(false))
        .with(SUBMODULES_AFTER, Answer::Flag(false))
}
