//! `changes.stage-all`: stages every change at once.

use model::contract::{Answers, Context, Danger, Outcome, Refusal, Step};

use super::files::{plan, with_files};

/// Stages every change: `git add --all`. While files are in conflict, which `--all` would take as resolved, markers and
/// all, every other file that is not all staged is named instead: `git add -- <path>…`.
///
/// Feature: changes.stage-all
#[must_use]
pub fn stage_all(context: &Context, _answers: &Answers) -> Outcome {
    let tree = &context.repository.working_tree;
    let mut paths: Vec<&str> = tree
        .unstaged
        .iter()
        .map(|change| change.path.as_str())
        .chain(tree.untracked.iter().map(String::as_str))
        .collect();
    paths.sort_unstable();
    paths.dedup();
    if paths.is_empty() {
        return Outcome::Refuse(Refusal::saying("There is nothing to stage."));
    }
    let folder = context.repository.folder.clone();
    let step = if tree.conflicted.is_empty() {
        Step::git(folder, ["add", "--all"])
    } else {
        with_files(folder, &["add"], paths)
    };
    Outcome::Run(plan(
        String::from("Stage all changes"),
        vec![step],
        Danger::None,
        "Unstaging all changes takes it back.",
    ))
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests {
    use super::stage_all;
    use crate::testing::{context, lines, modified, plan, state};
    use model::contract::{Answers, Outcome, Target};
    use model::state::{ConflictKind, ConflictedFile};

    #[test]
    fn changes_stage_all__with_no_conflict_adds_all_untracked_included() {
        let mut state = state();
        state.working_tree.unstaged.push(modified("a.txt"));
        state.working_tree.untracked.push(String::from("new.txt"));
        let plan = plan(stage_all(&context(state, Target::Changes), &Answers::new()));
        assert_eq!(lines(&plan), ["git add --all"]);
    }

    #[test]
    fn changes_stage_all__with_a_conflict_names_every_other_file() {
        let mut state = state();
        state.working_tree.conflicted.push(ConflictedFile {
            path: String::from("a.txt"),
            kind: ConflictKind::BothModified,
        });
        state.working_tree.unstaged.push(modified("c.txt"));
        state.working_tree.untracked.push(String::from("b.txt"));
        let plan = plan(stage_all(&context(state, Target::Changes), &Answers::new()));
        assert_eq!(lines(&plan), ["git add -- b.txt c.txt"]);
    }

    #[test]
    fn refuses_with_nothing_to_stage() {
        let mut state = state();
        state.working_tree.staged.push(modified("a.txt"));
        assert!(matches!(
            stage_all(&context(state, Target::Changes), &Answers::new()),
            Outcome::Refuse(_)
        ));
    }
}
