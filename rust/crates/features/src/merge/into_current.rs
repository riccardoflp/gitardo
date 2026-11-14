//! `merge.into-current`: merges a commit, a branch or a remote branch into what is checked out, as the configuration
//! of git says.

use model::contract::{Answers, Context, Outcome, Plan, Step, Text, Undo};

use super::{merged, predicted};
use crate::history::{conflicts_danger, current};

/// Merges what it was invoked on into what is checked out with `git merge <ref>`: a fast-forward when one is possible,
/// otherwise a merge commit. It asks first when conflicts are predicted, naming them.
///
/// Feature: merge.into-current
#[must_use]
pub fn into_current(context: &Context, answers: &Answers) -> Outcome {
    let merged = match merged(context) {
        Ok(merged) => merged,
        Err(outcome) => return *outcome,
    };
    let conflicts = match predicted(answers, &merged) {
        Ok(conflicts) => conflicts,
        Err(outcome) => return *outcome,
    };
    Outcome::Run(Plan {
        title: Text::from(format!("Merge {} into {}", merged.name, current(context))),
        steps: vec![Step::git(
            context.repository.folder.clone(),
            ["merge", merged.argument.as_str()],
        )],
        danger: conflicts_danger(None, conflicts),
        undo: Undo::ByReflog,
        after: Vec::new(),
        on_failure: Vec::new(),
    })
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests {
    use super::into_current;
    use crate::merge::CONFLICTS;
    use crate::testing::{branch, commit, context, lines, plan, state, tag};
    use model::contract::{Answer, Answers, Danger, Outcome, Query, Target};

    fn predicting(paths: &[&str]) -> Answers {
        Answers::new().with(
            CONFLICTS,
            Answer::Conflicts(Some(paths.iter().map(|path| (*path).to_owned()).collect())),
        )
    }

    #[test]
    fn merge_into_current__merges_feature_into_main_with_git_merge_feature() {
        let mut state = state();
        state.branches.push(branch("feature", 'b', false));
        let context = context(state, Target::Branch(String::from("feature")));
        assert!(matches!(
            into_current(&context, &Answers::new()),
            Outcome::Read(CONFLICTS, Query::PredictedConflicts { ref theirs, .. }) if theirs == "feature"
        ));
        let plan = plan(into_current(&context, &predicting(&[])));
        assert_eq!(lines(&plan), ["git merge feature"]);
        assert_eq!(plan.title.as_str(), "Merge feature into main");
        assert_eq!(plan.danger, Danger::None);
    }

    #[test]
    fn merge_into_current__names_a_branch_in_full_when_a_tag_has_its_name() {
        let mut state = state();
        state.branches.push(branch("feature", 'b', false));
        state.tags.push(tag("feature", 'c'));
        let context = context(state, Target::Branch(String::from("feature")));
        let plan = plan(into_current(&context, &predicting(&[])));
        assert_eq!(lines(&plan), ["git merge refs/heads/feature"]);
    }

    #[test]
    fn merge_into_current__the_commit_checked_out_is_not_merged() {
        let context = context(state(), Target::Commit(commit('a')));
        assert!(matches!(into_current(&context, &Answers::new()), Outcome::Refuse(_)));
    }

    #[test]
    fn merge_into_current__asks_first_naming_the_file_predicted_to_conflict() {
        let mut state = state();
        state.branches.push(branch("feature", 'b', false));
        let context = context(state, Target::Branch(String::from("feature")));
        let plan = plan(into_current(&context, &predicting(&["src/a.rs"])));
        let Danger::Confirm(reason) = plan.danger else {
            panic!("a merge predicted to conflict asks first")
        };
        assert_eq!(reason.as_str(), "This will stop with conflicts in 1 file:\nsrc/a.rs");
    }
}
