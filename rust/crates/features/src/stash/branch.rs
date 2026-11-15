//! `stash.branch`: makes a branch at the commit a stash was made on, checks it out and applies the stash there.

use model::contract::{
    AnswerKey, Answers, Context, Danger, Outcome, Plan, Question, QuestionKind, Step, Text, Undo, Validation,
};

use super::target_stash;
use crate::branch::submodules::update_after_checkout;

/// The name typed for the new branch.
pub const NAME: AnswerKey = AnswerKey("name");

/// Asks the name of the branch, refusing while typing a name git does not take or a branch has, then runs
/// `git stash branch <name> stash@{N}`, which drops the stash once it applied cleanly, and updates the submodules as
/// `submodule.update-on-checkout` says, since the branch is checked out.
///
/// Feature: stash.branch
#[must_use]
pub fn branch(context: &Context, answers: &Answers) -> Outcome {
    let stash = match target_stash(context) {
        Ok(stash) => stash,
        Err(refusal) => return *refusal,
    };
    let stash_name = format!("stash@{{{}}}", stash.index);
    let Some(name) = answers.text(NAME) else {
        let taken = context
            .repository
            .branches
            .iter()
            .map(|branch| branch.name.clone())
            .collect();
        return Outcome::Ask(Question {
            key: NAME,
            kind: QuestionKind::Text {
                placeholder: Some(Text::from("feature/my-change")),
                validation: Validation::ReferenceName {
                    taken,
                    taken_message: Text::from("A branch with that name already exists"),
                },
            },
            title: Text::from(format!("Create branch from {stash_name}")),
            detail: None,
            default: None,
        });
    };
    // The branch starts at the commit the stash was made on, its first parent.
    let start = format!("{stash_name}^");
    let plan = Plan {
        title: Text::from(format!("Create branch `{name}` from {stash_name}")),
        steps: vec![Step::git(
            context.repository.folder.clone(),
            ["stash", "branch", name, &stash_name],
        )],
        danger: Danger::None,
        undo: Undo::ByReflog,
        after: vec![model::contract::FollowUp::SelectHead],
        on_failure: Vec::new(),
    };
    update_after_checkout(context, answers, &start, plan)
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests {
    use super::{NAME, branch};
    use crate::branch::submodules::{SUBMODULES_AFTER, SUBMODULES_BEFORE, none};
    use crate::testing::{context, lines, plan, question, stash, state};
    use model::contract::{Answer, Answers, Outcome, Query, QuestionKind, Target};

    fn on_stash() -> model::contract::Context {
        let mut state = state();
        state.stashes.push(stash(0, "On main: try this"));
        context(state, Target::Stash(0))
    }

    #[test]
    fn stash_branch__makes_the_branch_from_the_stash() {
        let asked = question(branch(&on_stash(), &Answers::new()));
        assert_eq!(asked.title.as_str(), "Create branch from stash@{0}");
        let answers = none(Answers::new().with(NAME, Answer::Text(String::from("rescue"))));
        assert_eq!(
            lines(&plan(branch(&on_stash(), &answers))),
            ["git stash branch rescue stash@{0}"]
        );
    }

    #[test]
    fn stash_branch__where_the_commit_of_the_stash_has_submodules_it_ends_with_their_update() {
        let answers = Answers::new()
            .with(NAME, Answer::Text(String::from("rescue")))
            .with(SUBMODULES_BEFORE, Answer::Flag(false));
        let Outcome::Read(key, Query::HasFile { revision, .. }) = branch(&on_stash(), &answers) else {
            panic!("whether the commit of the stash has submodules is read")
        };
        assert_eq!((key, revision.as_str()), (SUBMODULES_AFTER, "stash@{0}^"));
        let plan = plan(branch(&on_stash(), &answers.with(SUBMODULES_AFTER, Answer::Flag(true))));
        assert_eq!(
            lines(&plan).last().map(String::as_str),
            Some("git submodule update --init --recursive")
        );
    }

    #[test]
    fn stash_branch__a_taken_name_or_one_git_refuses_cannot_be_accepted() {
        let QuestionKind::Text { validation, .. } = question(branch(&on_stash(), &Answers::new())).kind else {
            panic!("the name is typed")
        };
        assert_eq!(
            validation.problem("main").map(|text| text.to_string()),
            Some(String::from("A branch with that name already exists"))
        );
        assert!(validation.problem("bad..name").is_some());
        assert_eq!(validation.problem("rescue"), None);
    }
}
