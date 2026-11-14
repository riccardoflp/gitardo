//! `revert.commit`: makes a new commit that takes back the changes of a commit.

use model::contract::{AnswerKey, Answers, Context, Danger, Outcome, Plan, Query, Step, Text, Undo};

use crate::cherry_pick::skip_when_empty;
use crate::history::{refuse_under_way, target_commit};

/// The commit reverted, read for its parents.
const REVERTED: AnswerKey = AnswerKey("reverted");

/// Reverts the commit it was invoked on with `git revert --no-edit [-m 1] <hash>`: a merge commit against its first
/// parent, with the message git writes, `Revert "<subject>"`, since no editor can open.
///
/// Feature: revert.commit
#[must_use]
pub fn commit(context: &Context, answers: &Answers) -> Outcome {
    let commit = match target_commit(context) {
        Ok(commit) => commit,
        Err(outcome) => return *outcome,
    };
    if let Some(refusal) = refuse_under_way(context) {
        return refusal;
    }
    let Some(reverted) = answers.commits(REVERTED) else {
        return Outcome::Read(
            REVERTED,
            Query::Commits {
                revisions: vec![commit.to_string()],
            },
        );
    };
    let mut arguments = vec!["revert", "--no-edit"];
    if reverted.first().is_some_and(model::commit::Commit::is_merge) {
        arguments.extend(["-m", "1"]);
    }
    arguments.push(commit.as_str());
    Outcome::Run(Plan {
        title: Text::from(format!("Revert {}", commit.short())),
        steps: vec![Step::git(context.repository.folder.clone(), arguments)],
        danger: Danger::None,
        undo: Undo::ByReflog,
        after: vec![model::contract::FollowUp::SelectHead],
        on_failure: skip_when_empty(),
    })
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests {
    use super::{REVERTED, commit};
    use crate::testing::{self, context, lines, plan, state};
    use model::commit::Commit;
    use model::contract::{Answer, Answers, Target};
    use model::{CommitId, Timestamp};

    fn reverted(parents: &[CommitId]) -> Answers {
        Answers::new().with(
            REVERTED,
            Answer::Commits(vec![Commit {
                id: testing::commit('b'),
                parents: parents.to_vec(),
                author_name: String::new(),
                author_email: String::new(),
                author_date: Timestamp(0),
                subject: String::from("Fix a"),
            }]),
        )
    }

    #[test]
    fn revert_commit__a_commit_with_one_parent_runs_git_revert_no_edit() {
        let context = context(state(), Target::Commit(testing::commit('b')));
        let plan = plan(commit(&context, &reverted(&[testing::commit('c')])));
        assert_eq!(lines(&plan), [format!("git revert --no-edit {}", testing::commit('b'))]);
    }

    #[test]
    fn revert_commit__a_merge_commit_is_reverted_against_its_first_parent() {
        let context = context(state(), Target::Commit(testing::commit('b')));
        let plan = plan(commit(
            &context,
            &reverted(&[testing::commit('c'), testing::commit('d')]),
        ));
        assert_eq!(
            lines(&plan),
            [format!("git revert --no-edit -m 1 {}", testing::commit('b'))]
        );
    }
}
