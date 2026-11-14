//! `changes.unstage`: takes the changes of a file out of the index; the file on disk is not touched.

use model::contract::{Answers, Context, Danger, Outcome};

use super::files::{plan, staged_names, target_file, with_files};

/// Unstages a file of the Changes page: `git restore --staged -- <path>`, with the old name too for a staged rename;
/// before the first commit, which has no `HEAD` to restore the index from, `git rm --cached -r -- <path>`.
///
/// Feature: changes.unstage
#[must_use]
pub fn unstage(context: &Context, _answers: &Answers) -> Outcome {
    let path = match target_file(context) {
        Ok(path) => path,
        Err(refusal) => return Outcome::Refuse(refusal),
    };
    let repository = &context.repository;
    let folder = repository.folder.clone();
    let step = if repository.has_commit() {
        with_files(
            folder,
            &["restore", "--staged"],
            staged_names(&repository.working_tree, path),
        )
    } else {
        with_files(folder, &["rm", "--cached", "-r"], [path])
    };
    Outcome::Run(plan(
        format!("Unstage {path}"),
        vec![step],
        Danger::None,
        "Staging it again takes it back.",
    ))
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests {
    use super::unstage;
    use crate::testing::{context, lines, modified, plan, renamed, state};
    use model::contract::{Answers, Target};
    use model::state::Head;

    #[test]
    fn changes_unstage__a_staged_file_is_restored_from_head() {
        let mut state = state();
        state.working_tree.staged.push(modified("a.txt"));
        let plan = plan(unstage(
            &context(state, Target::File(String::from("a.txt"))),
            &Answers::new(),
        ));
        assert_eq!(lines(&plan), ["git restore --staged -- a.txt"]);
    }

    #[test]
    fn changes_unstage__a_staged_rename_names_both_names() {
        let mut state = state();
        state.working_tree.staged.push(renamed("b", "a"));
        let plan = plan(unstage(
            &context(state, Target::File(String::from("b"))),
            &Answers::new(),
        ));
        assert_eq!(lines(&plan), ["git restore --staged -- b a"]);
    }

    #[test]
    fn changes_unstage__before_the_first_commit_the_file_leaves_the_index() {
        let mut state = state();
        state.head = Head {
            commit: None,
            branch: Some(String::from("refs/heads/main")),
        };
        let plan = plan(unstage(
            &context(state, Target::File(String::from("a.txt"))),
            &Answers::new(),
        ));
        assert_eq!(lines(&plan), ["git rm --cached -r -- a.txt"]);
    }
}
