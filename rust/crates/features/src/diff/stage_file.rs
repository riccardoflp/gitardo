//! `diff.stage-file`: stages the whole file of a Diff page, from its title button **Stage file**.

use model::contract::{Answers, Context, Outcome, Refusal, Target};

use super::change::{index_plan, with_file};

/// Said when the operation is given no file.
pub(super) const CHOOSE_A_FILE: &str = "Choose the file of a Diff page.";

/// Stages the file: `git add -- <file>`, the path named as itself.
///
/// Feature: diff.stage-file
#[must_use]
pub fn stage_file(context: &Context, _answers: &Answers) -> Outcome {
    let Target::File(path) = &context.target else {
        return Outcome::Refuse(Refusal::saying(CHOOSE_A_FILE));
    };
    let step = with_file(context.repository.folder.clone(), &["add"], path);
    Outcome::Run(index_plan(
        format!("Stage {path}"),
        vec![step],
        "Unstage file takes it back: the index has no reflog.",
    ))
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests {
    use super::stage_file;
    use crate::testing::{context, lines, plan, state};
    use model::contract::{Answers, Target};

    #[test]
    fn diff_stage_file__runs_exactly_git_add_of_the_file() {
        let staged = plan(stage_file(
            &context(state(), Target::File(String::from("src/app.rs"))),
            &Answers::new(),
        ));
        assert_eq!(lines(&staged), ["git add -- src/app.rs"]);
        assert_eq!(staged.title.as_str(), "Stage src/app.rs");
    }
}
