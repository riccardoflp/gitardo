//! `changes.unstage-all`: takes everything out of the index; the files on disk are not touched.

use model::contract::{Answers, Context, Danger, Outcome, Refusal, Step};

use super::files::{plan, staged_names, with_files};

/// Unstages everything: `git reset`, or `git rm --cached -r -- .` before the first commit. While files are in
/// conflict, whose conflicts `git reset` would forget, the files staged are named instead:
/// `git restore --staged -- <path>…`, a staged rename with both of its names.
///
/// Feature: changes.unstage-all
#[must_use]
pub fn unstage_all(context: &Context, _answers: &Answers) -> Outcome {
    let repository = &context.repository;
    let tree = &repository.working_tree;
    if tree.staged.is_empty() {
        return Outcome::Refuse(Refusal::saying("There is nothing to unstage."));
    }
    let folder = repository.folder.clone();
    let step = if !tree.conflicted.is_empty() {
        let names = tree.staged.iter().flat_map(|change| staged_names(tree, &change.path));
        with_files(folder, &["restore", "--staged"], names)
    } else if repository.has_commit() {
        Step::git(folder, ["reset"])
    } else {
        with_files(folder, &["rm", "--cached", "-r"], ["."])
    };
    Outcome::Run(plan(
        String::from("Unstage all changes"),
        vec![step],
        Danger::None,
        "Staging all changes takes it back.",
    ))
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests {
    use super::unstage_all;
    use crate::testing::{context, lines, modified, plan, renamed, state};
    use model::contract::{Answers, Target};
    use model::state::{ConflictKind, ConflictedFile, Head};

    #[test]
    fn changes_unstage_all__with_no_conflict_resets() {
        let mut state = state();
        state.working_tree.staged.push(modified("a.txt"));
        let plan = plan(unstage_all(&context(state, Target::Changes), &Answers::new()));
        assert_eq!(lines(&plan), ["git reset"]);
    }

    #[test]
    fn changes_unstage_all__with_a_conflict_names_the_staged_files() {
        let mut state = state();
        state.working_tree.conflicted.push(ConflictedFile {
            path: String::from("c.txt"),
            kind: ConflictKind::BothModified,
        });
        state.working_tree.staged.push(modified("a.txt"));
        state.working_tree.staged.push(renamed("new", "old"));
        let plan = plan(unstage_all(&context(state, Target::Changes), &Answers::new()));
        assert_eq!(lines(&plan), ["git restore --staged -- a.txt new old"]);
    }

    #[test]
    fn changes_unstage_all__before_the_first_commit_removes_everything_from_the_index() {
        let mut state = state();
        state.head = Head {
            commit: None,
            branch: Some(String::from("refs/heads/main")),
        };
        state.working_tree.staged.push(modified("a.txt"));
        let plan = plan(unstage_all(&context(state, Target::Changes), &Answers::new()));
        assert_eq!(lines(&plan), ["git rm --cached -r -- ."]);
    }
}
