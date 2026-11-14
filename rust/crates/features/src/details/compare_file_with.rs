//! `details.compare-file-with`: compares one file, as a commit, a comparison or the working tree has it, with the
//! branch, tag, commit or working tree the user chooses, in the Diff page.

use model::compare::CheckedOut;
use model::contract::{Answers, Context, FollowUp, Outcome, Refusal, Target};

use crate::compare::{Picked, SideKeys, SideQuestion, no_step, side};

/// Said of a menu opened on something else than a file of a list.
const CHOOSE: &str = "Choose a file of a commit, of a comparison or of the changes to compare.";

/// Asks for the other side, the side the file is taken from left out and **Working tree** offered unless the file is
/// taken from it, then opens the Diff page of the file between the two: the working tree on the right, otherwise the
/// side chosen on the left and the file's own on the right.
///
/// Feature: details.compare-file-with
#[must_use]
pub fn compare_file_with(context: &Context, answers: &Answers) -> Outcome {
    let Target::ListedFile(listed) = &context.target else {
        return Outcome::Refuse(Refusal::saying(CHOOSE));
    };
    let repository = &context.repository;
    let checked_out = CheckedOut {
        head: repository.head.commit.as_ref(),
        branch: repository.checked_out_branch(),
    };
    let Some(own) = listed.own_side(checked_out) else {
        return Outcome::Refuse(Refusal::saying(&format!(
            "{} has no version to compare here.",
            listed.name()
        )));
    };
    let question = SideQuestion {
        title: format!("Compare {} of {} with", listed.name(), own.name),
        placeholder: "The branch, tag or commit to compare the file with",
        chosen: Some(&own),
        may_offer_working_tree: !own.is_working_tree(),
        recent: None,
    };
    let chosen = match side(context, answers, SideKeys::FIRST, &question) {
        Ok(Picked::Side(chosen)) => chosen,
        Ok(Picked::Recent(_)) => return Outcome::Refuse(Refusal::saying("Choose a branch, tag or commit.")),
        Err(outcome) => return *outcome,
    };
    let sides = listed.sides_with(own, chosen);
    let title = format!(
        "Compare {} of {} with {}",
        listed.name(),
        sides.right.name,
        sides.left.name
    );
    Outcome::Run(no_step(title, vec![FollowUp::OpenFileDiff(Box::new(sides))]))
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests {
    use model::compare::{CompareSide, FileList, FileSides, ListedFile, NamedCommit, Side};
    use model::contract::{Answers, FollowUp, Outcome, QuestionKind, Target};
    use model::state::{ChangeKind, FileChange};

    use super::compare_file_with;
    use crate::compare::FIRST;
    use crate::compare::testing::picked;
    use crate::testing::{branch, commit, context, plan, question, state};

    /// A repository with `main` checked out at `a` and a branch `feature` at `b`.
    fn repository() -> model::state::RepositoryState {
        let mut state = state();
        state.branches.push(branch("feature", 'b', false));
        state
    }

    /// `src/app.rs` as the commit `b` lists it, changed in some way.
    fn of_commit(kind: ChangeKind) -> Target {
        Target::ListedFile(Box::new(ListedFile {
            file: FileChange {
                path: String::from("src/app.rs"),
                original_path: None,
                kind,
            },
            list: FileList::Commit {
                commit: NamedCommit {
                    name: commit('b').short().to_owned(),
                    commit: commit('b'),
                },
                first_parent: Some(commit('c')),
            },
        }))
    }

    /// The two sides the Diff page opens with.
    fn opened(outcome: Outcome) -> FileSides {
        let plan = plan(outcome);
        let Some(FollowUp::OpenFileDiff(sides)) = plan.after.first() else {
            panic!("opens a Diff page: {plan:?}");
        };
        assert_eq!(plan.steps, []);
        sides.as_ref().clone()
    }

    /// A side at a commit, named so, and not followed.
    fn fixed(name: &str, digit: char) -> CompareSide {
        CompareSide {
            name: name.to_owned(),
            side: Side::Commit(commit(digit)),
            is_followed: false,
            is_gone: false,
        }
    }

    /// The identifiers of the choices of a question.
    fn choices(outcome: Outcome) -> Vec<String> {
        match question(outcome).kind {
            QuestionKind::Pick { choices } => choices.into_iter().map(|choice| choice.identifier).collect(),
            other => panic!("a pick, not {other:?}"),
        }
    }

    #[test]
    fn details_compare_file_with__choosing_main_puts_main_on_the_left_and_the_commit_on_the_right() {
        let context = context(repository(), of_commit(ChangeKind::Modified));
        let asked = question(compare_file_with(&context, &Answers::new()));
        assert_eq!(
            asked.title.as_str(),
            format!("Compare app.rs of {} with", commit('b').short())
        );
        let sides = opened(compare_file_with(&context, &picked(&[(FIRST, "ref:main")])));
        assert_eq!(sides.left.name, "main");
        assert_eq!(sides.left.side, Side::Commit(commit('a')));
        assert_eq!(sides.right, fixed(commit('b').short(), 'b'));
        assert_eq!(sides.path, "src/app.rs");
    }

    #[test]
    fn details_compare_file_with__choosing_the_working_tree_puts_the_file_on_disk_on_the_right() {
        let context = context(repository(), of_commit(ChangeKind::Modified));
        let sides = opened(compare_file_with(&context, &picked(&[(FIRST, "working-tree")])));
        assert_eq!(sides.left, fixed(commit('b').short(), 'b'));
        assert!(sides.right.is_working_tree());
        let changes = Target::ListedFile(Box::new(ListedFile {
            file: FileChange {
                path: String::from("src/app.rs"),
                original_path: None,
                kind: ChangeKind::Modified,
            },
            list: FileList::Changes,
        }));
        let context = model::contract::Context {
            target: changes,
            ..context
        };
        let sides = opened(compare_file_with(&context, &picked(&[(FIRST, "ref:feature")])));
        assert_eq!(
            (sides.left.name.as_str(), sides.right.is_working_tree()),
            ("feature", true)
        );
    }

    #[test]
    fn details_compare_file_with__a_file_the_commit_deleted_is_compared_as_it_was_in_the_parent() {
        let context = context(repository(), of_commit(ChangeKind::Deleted));
        let asked = question(compare_file_with(&context, &Answers::new()));
        assert_eq!(
            asked.title.as_str(),
            format!("Compare app.rs of {} with", commit('c').short())
        );
        let sides = opened(compare_file_with(&context, &picked(&[(FIRST, "ref:main")])));
        assert_eq!(sides.right, fixed(commit('c').short(), 'c'));
    }

    #[test]
    fn details_compare_file_with__the_side_the_file_is_taken_from_is_not_among_the_choices() {
        let compared = Target::ListedFile(Box::new(ListedFile {
            file: FileChange {
                path: String::from("src/app.rs"),
                original_path: None,
                kind: ChangeKind::Modified,
            },
            list: FileList::Comparison {
                first: crate::compare::testing::branch("main", 'a'),
                second: crate::compare::testing::branch("feature", 'b'),
            },
        }));
        let offered = choices(compare_file_with(&context(repository(), compared), &Answers::new()));
        assert!(!offered.contains(&String::from("ref:feature")), "{offered:?}");
        assert!(offered.contains(&String::from("ref:main")), "{offered:?}");
        assert!(offered.contains(&String::from("working-tree")), "{offered:?}");
        let changes = Target::ListedFile(Box::new(ListedFile {
            file: FileChange {
                path: String::from("src/app.rs"),
                original_path: None,
                kind: ChangeKind::Modified,
            },
            list: FileList::Changes,
        }));
        let offered = choices(compare_file_with(&context(repository(), changes), &Answers::new()));
        assert!(!offered.contains(&String::from("working-tree")), "{offered:?}");
    }
}
