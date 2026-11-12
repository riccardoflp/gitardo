//! `remote.rename`: renames a remote; git renames its remote-tracking branches and the upstreams that name it.

use model::contract::{
    Answer, AnswerKey, Answers, Context, Danger, Outcome, Plan, Question, QuestionKind, Step, Text, Undo, Validation,
};

use super::{target_remote, unchanged};

/// The new name typed for the remote.
pub const NEW_NAME: AnswerKey = AnswerKey("new-name");

/// Asks the new name of the remote it was invoked on, starting from the name it has, and renames it; nothing runs when
/// the name is unchanged.
///
/// Feature: remote.rename
#[must_use]
pub fn rename(context: &Context, answers: &Answers) -> Outcome {
    let name = match target_remote(context) {
        Ok(name) => name,
        Err(refusal) => return *refusal,
    };
    let Some(new_name) = answers.text(NEW_NAME) else {
        let taken = context
            .repository
            .remotes
            .iter()
            .filter(|remote| *remote != name)
            .cloned()
            .collect();
        return Outcome::Ask(Question {
            key: NEW_NAME,
            kind: QuestionKind::Text {
                placeholder: None,
                validation: Validation::ReferenceName {
                    taken,
                    taken_message: Text::from("A remote with that name already exists"),
                },
            },
            title: Text::from(format!("Rename remote `{name}`")),
            detail: None,
            default: Some(Answer::Text(name.to_owned())),
        });
    };
    if new_name == name {
        return Outcome::Run(unchanged(format!("The remote {name} keeps its name.")));
    }
    Outcome::Run(Plan {
        title: Text::from(format!("Rename remote {name} to {new_name}")),
        steps: vec![Step::git(
            context.repository.folder.clone(),
            ["remote", "rename", name, new_name],
        )],
        danger: Danger::None,
        undo: Undo::Impossible(Text::from("Not taken back by Undo: renaming it back does.")),
        after: vec![model::contract::FollowUp::ReadPullRequests],
        on_failure: Vec::new(),
    })
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests {
    use super::{NEW_NAME, rename};
    use crate::testing::{context, lines, plan, question, state};
    use model::contract::{Answer, Answers, QuestionKind, Target};

    fn origin() -> model::contract::Context {
        let mut state = state();
        state.remotes = vec![String::from("origin"), String::from("upstream")];
        context(state, Target::Remote(String::from("origin")))
    }

    #[test]
    fn remote_rename__starts_from_its_name_and_refuses_another_remote_name() {
        let asked = question(rename(&origin(), &Answers::new()));
        assert_eq!(asked.default, Some(Answer::Text(String::from("origin"))));
        let QuestionKind::Text { validation, .. } = asked.kind else {
            panic!("the name is typed")
        };
        assert!(validation.problem("upstream").is_some());
        assert_eq!(validation.problem("origin"), None);
    }

    #[test]
    fn remote_rename__renames_and_runs_nothing_when_unchanged() {
        let answers = Answers::new().with(NEW_NAME, Answer::Text(String::from("github")));
        assert_eq!(
            lines(&plan(rename(&origin(), &answers))),
            ["git remote rename origin github"]
        );
        let same = Answers::new().with(NEW_NAME, Answer::Text(String::from("origin")));
        assert_eq!(plan(rename(&origin(), &same)).steps, Vec::new());
    }
}
