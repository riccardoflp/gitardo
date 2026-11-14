//! `diff.unstage-file`: takes the whole file of a Diff page of the index out of the index, from its title button
//! **Unstage file**.

use model::contract::{Answers, Context, Outcome, Refusal, Target};

use super::change::{index_plan, with_file};
use super::stage_file::CHOOSE_A_FILE;

/// Unstages the file: `git restore --staged -- <file>`; before the first commit, which has no `HEAD` to restore the
/// index from, `git rm --cached -r -- <file>`.
///
/// Feature: diff.unstage-file
#[must_use]
pub fn unstage_file(context: &Context, _answers: &Answers) -> Outcome {
    let Target::File(path) = &context.target else {
        return Outcome::Refuse(Refusal::saying(CHOOSE_A_FILE));
    };
    let folder = context.repository.folder.clone();
    let step = if context.repository.has_commit() {
        with_file(folder, &["restore", "--staged"], path)
    } else {
        with_file(folder, &["rm", "--cached", "-r"], path)
    };
    Outcome::Run(index_plan(
        format!("Unstage {path}"),
        vec![step],
        "Stage file takes it back: the index has no reflog.",
    ))
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests {
    use super::unstage_file;
    use crate::testing::{context, lines, plan, state};
    use model::contract::{Answers, Target};
    use model::state::RepositoryState;

    fn unstaged(state: RepositoryState) -> Vec<String> {
        lines(&plan(unstage_file(
            &context(state, Target::File(String::from("src/app.rs"))),
            &Answers::new(),
        )))
    }

    #[test]
    fn diff_unstage_file__in_the_diff_of_the_index_against_head_it_restores_the_index() {
        assert_eq!(unstaged(state()), ["git restore --staged -- src/app.rs"]);
    }

    #[test]
    fn diff_unstage_file__with_no_commit_yet_it_takes_the_file_out_of_the_index() {
        let mut state = state();
        state.head.commit = None;
        assert_eq!(unstaged(state), ["git rm --cached -r -- src/app.rs"]);
    }
}
