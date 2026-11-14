//! `branch.rename`: gives a local branch another name.

use model::contract::{
    Answer, AnswerKey, Answers, Context, Danger, Outcome, Plan, Question, QuestionKind, Step, Text, Undo, Validation,
};

use super::target::local_branch;

/// The new name typed.
pub const NAME: AnswerKey = AnswerKey("name");

/// Asks for the new name of the local branch it was invoked on, starting from the name it has, and renames it with
/// `git branch -m`, which takes its configuration with it, its upstream included.
///
/// Feature: branch.rename
#[must_use]
pub fn rename(context: &Context, answers: &Answers) -> Outcome {
    let branch = match local_branch(context) {
        Ok(branch) => branch,
        Err(refusal) => return *refusal,
    };
    let old = branch.name.as_str();
    let Some(new) = answers.text(NAME) else {
        return Outcome::Ask(name_question(context, old));
    };
    Outcome::Run(Plan {
        title: Text::from(format!("Rename branch {old} to {new}")),
        steps: vec![Step::git(context.repository.folder.clone(), ["branch", "-m", old, new])],
        danger: Danger::None,
        undo: Undo::Impossible(Text::from("Not taken back by Undo: renaming it back does.")),
        after: Vec::new(),
        on_failure: Vec::new(),
    })
}

/// The input box of the new name, which takes no name of a branch there is, the old one included.
fn name_question(context: &Context, old: &str) -> Question {
    let taken = context
        .repository
        .branches
        .iter()
        .map(|branch| branch.name.clone())
        .collect();
    Question {
        key: NAME,
        kind: QuestionKind::Text {
            placeholder: None,
            validation: Validation::ReferenceName {
                taken,
                taken_message: Text::from("A branch with that name already exists"),
            },
        },
        title: Text::from(format!("Rename branch `{old}`")),
        detail: None,
        default: Some(Answer::Text(old.to_owned())),
    }
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests {
    use super::{NAME, rename};
    use crate::testing::{branch, context, lines, plan, question, state};
    use model::contract::{Answer, Answers, QuestionKind, Target};

    fn on_feature() -> model::contract::Context {
        let mut state = state();
        state.branches.push(branch("feature", 'b', false));
        context(state, Target::Branch(String::from("feature")))
    }

    #[test]
    fn branch_rename__renames_with_branch_m() {
        let context = on_feature();
        let asked = question(rename(&context, &Answers::new()));
        assert_eq!(asked.title.as_str(), "Rename branch `feature`");
        assert_eq!(asked.default, Some(Answer::Text(String::from("feature"))));
        let answers = Answers::new().with(NAME, Answer::Text(String::from("feature/login")));
        assert_eq!(
            lines(&plan(rename(&context, &answers))),
            ["git branch -m feature feature/login"]
        );
    }

    #[test]
    fn branch_rename__the_old_name_unchanged_is_taken() {
        let asked = question(rename(&on_feature(), &Answers::new()));
        let QuestionKind::Text { validation, .. } = asked.kind else {
            panic!("the name is typed")
        };
        assert_eq!(
            validation.problem("feature").map(|text| text.to_string()),
            Some(String::from("A branch with that name already exists"))
        );
    }
}
