//! `cherry-pick.commit`: applies the changes of one commit onto what is checked out, as a new commit, after saying
//! which files will conflict.

use model::contract::{AnswerKey, Answers, Context, Outcome, Plan, Query, Refusal, Step, Text, Undo};

use super::skip_when_empty;
use crate::history::{conflicts_danger, current, refuse_under_way, target_commit};

/// The commit picked, read for its parents.
const PICKED: AnswerKey = AnswerKey("picked");

/// The files the pick would leave in conflict.
const CONFLICTS: AnswerKey = AnswerKey("conflicts");

/// Picks the commit it was invoked on with `git cherry-pick [-x] [-m 1] <hash>`: a merge commit against its first
/// parent, and with `cherryPick.recordOrigin` the origin written in the new message. It asks first when conflicts are
/// predicted, merging the commit into `HEAD` from its parent; a merge commit is not predicted.
///
/// Feature: cherry-pick.commit
#[must_use]
pub fn commit(context: &Context, answers: &Answers) -> Outcome {
    let commit = match target_commit(context) {
        Ok(commit) => commit,
        Err(outcome) => return *outcome,
    };
    if let Some(refusal) = refuse_under_way(context) {
        return refusal;
    }
    if context.repository.head.commit.as_ref() == Some(commit) {
        return Outcome::Refuse(Refusal::saying(&format!(
            "{} is the commit checked out already.",
            commit.short()
        )));
    }
    let Some(picked) = answers.commits(PICKED) else {
        return Outcome::Read(
            PICKED,
            Query::Commits {
                revisions: vec![commit.to_string()],
            },
        );
    };
    let parents = picked.first().map_or(0, |picked| picked.parents.len());
    let conflicts = if parents == 1 {
        let Some(conflicts) = answers.conflicts(CONFLICTS) else {
            return Outcome::Read(
                CONFLICTS,
                Query::PredictedConflicts {
                    ours: None,
                    theirs: commit.to_string(),
                    base: Some(format!("{commit}^")),
                },
            );
        };
        conflicts
    } else {
        None
    };
    let mut arguments = vec!["cherry-pick"];
    if context.settings.cherry_pick_record_origin {
        arguments.push("-x");
    }
    if parents > 1 {
        arguments.extend(["-m", "1"]);
    }
    arguments.push(commit.as_str());
    Outcome::Run(Plan {
        title: Text::from(format!("Cherry-pick {} onto {}", commit.short(), current(context))),
        steps: vec![Step::git(context.repository.folder.clone(), arguments)],
        danger: conflicts_danger(None, conflicts),
        undo: Undo::ByReflog,
        after: Vec::new(),
        on_failure: skip_when_empty(),
    })
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests {
    use super::{CONFLICTS, PICKED, commit};
    use crate::testing::{self, context, lines, plan, state};
    use model::commit::Commit;
    use model::contract::{Answer, Answers, Danger, Outcome, Query, Target};
    use model::{CommitId, Timestamp};

    fn picked(parents: &[CommitId]) -> Answers {
        Answers::new().with(
            PICKED,
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
    fn cherry_pick_commit__asks_first_naming_the_file_predicted_to_conflict() {
        let context = context(state(), Target::Commit(testing::commit('b')));
        let answers = picked(&[testing::commit('c')]);
        let Outcome::Read(CONFLICTS, Query::PredictedConflicts { base, theirs, .. }) = commit(&context, &answers)
        else {
            panic!("the conflicts are predicted")
        };
        assert_eq!(base, Some(format!("{}^", testing::commit('b'))));
        assert_eq!(theirs, testing::commit('b').to_string());
        let answers = answers.with(CONFLICTS, Answer::Conflicts(Some(vec![String::from("a.txt")])));
        let plan = plan(commit(&context, &answers));
        assert!(matches!(plan.danger, Danger::Confirm(ref reason) if reason.as_str().contains("\na.txt")));
    }

    #[test]
    fn cherry_pick_commit__picks_a_merge_against_its_first_parent_and_predicts_nothing() {
        let context = context(state(), Target::Commit(testing::commit('b')));
        let answers = picked(&[testing::commit('c'), testing::commit('d')]);
        let plan = plan(commit(&context, &answers));
        assert_eq!(lines(&plan), [format!("git cherry-pick -m 1 {}", testing::commit('b'))]);
        assert_eq!(plan.danger, Danger::None);
    }

    #[test]
    fn cherry_pick_commit__with_git_older_than_2_40_runs_as_with_no_conflict_predicted() {
        let context = context(state(), Target::Commit(testing::commit('b')));
        let answers = picked(&[testing::commit('c')]).with(CONFLICTS, Answer::Conflicts(None));
        let plan = plan(commit(&context, &answers));
        assert_eq!(lines(&plan), [format!("git cherry-pick {}", testing::commit('b'))]);
        assert_eq!(plan.danger, Danger::None);
    }

    #[test]
    fn cherry_pick_commit__the_commit_checked_out_is_not_picked() {
        let context = context(state(), Target::Commit(testing::commit('a')));
        assert!(matches!(commit(&context, &Answers::new()), Outcome::Refuse(_)));
    }

    #[test]
    fn cherry_pick_commit__record_origin_adds_x() {
        let mut context = context(state(), Target::Commit(testing::commit('b')));
        context.settings.cherry_pick_record_origin = true;
        let answers = picked(&[testing::commit('c')]).with(CONFLICTS, Answer::Conflicts(Some(Vec::new())));
        assert_eq!(
            lines(&plan(commit(&context, &answers))),
            [format!("git cherry-pick -x {}", testing::commit('b'))]
        );
    }
}
